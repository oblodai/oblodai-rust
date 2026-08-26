//! The reqwest backend against a real socket: the response-size cap and the redirect guard.
//!
//! Everything else in the suite stops at the `HttpBackend` seam, so these two live here — they are
//! properties of the socket layer itself, not of the pure core.

// The SDK's error carries the gateway's whole envelope by value, exactly as callers match on it;
// boxing it here would only make the test disagree with the crate it exercises.
#![allow(clippy::result_large_err)]
#![cfg(feature = "reqwest-client")]

use std::time::Duration;

use oblodai::core::http::{
    HttpBackend, HttpRequest, ReqwestBackend, MAX_FILE_BODY_BYTES, MAX_JSON_BODY_BYTES,
};
use oblodai::ErrorKind;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Serve exactly one request with a canned raw response, then close.
async fn serve_once(response: Vec<u8>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 4096];
        let _ = socket.read(&mut buf).await;
        let _ = socket.write_all(&response).await;
        let _ = socket.flush().await;
    });
    format!("http://{addr}/v1/probe")
}

fn request(url: String, max_body_bytes: usize) -> HttpRequest {
    HttpRequest {
        url,
        method: "GET",
        headers: vec![("Accept".into(), "application/json".into())],
        body: None,
        timeout: Duration::from_secs(5),
        max_body_bytes,
    }
}

fn http_response(status: &str, headers: &str, body: &[u8]) -> Vec<u8> {
    let mut out = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n",
        body.len()
    )
    .into_bytes();
    out.extend_from_slice(body);
    out
}

#[test]
fn the_caps_are_the_documented_ones() {
    assert_eq!(MAX_JSON_BODY_BYTES, 8 * 1024 * 1024);
    assert_eq!(MAX_FILE_BODY_BYTES, 64 * 1024 * 1024);
}

/// The body used to be read with `res.bytes()`, which buffers whatever the peer sends. A proxy
/// streaming gigabytes at an SDK holding a payout key is an out-of-memory kill, not an error.
#[tokio::test]
async fn a_body_over_the_cap_is_a_contract_error_not_an_allocation() {
    let body = vec![b'x'; 64 * 1024];
    let url = serve_once(http_response(
        "200 OK",
        "Content-Type: text/plain\r\n",
        &body,
    ))
    .await;
    let backend = ReqwestBackend::new().unwrap();
    let err = backend
        .send(request(url, 1024))
        .await
        .expect_err("the cap must refuse this");
    assert_eq!(err.kind(), ErrorKind::Contract);
    assert!(
        err.message().contains("1024") && err.message().contains("exceeds"),
        "{}",
        err.message()
    );
}

#[tokio::test]
async fn a_body_within_the_cap_is_read_whole() {
    let body = vec![b'y'; 4096];
    let url = serve_once(http_response(
        "200 OK",
        "Content-Type: text/plain\r\n",
        &body,
    ))
    .await;
    let backend = ReqwestBackend::new().unwrap();
    let raw = backend.send(request(url, 8192)).await.unwrap();
    assert_eq!(raw.status, 200);
    assert_eq!(raw.body.len(), 4096);
}

/// The default client refuses redirects, but a caller may inject their own via
/// `ReqwestBackend::with_client`. If that one follows a redirect, the answer did not come from the
/// origin the request was signed for — and the signed headers went to a third party.
#[tokio::test]
async fn a_redirect_followed_by_an_injected_client_is_detected() {
    let target = serve_once(http_response(
        "200 OK",
        "Content-Type: application/json\r\n",
        br#"{"state":0,"result":{}}"#,
    ))
    .await;
    let url = serve_once(http_response(
        "302 Found",
        &format!("Location: {target}\r\n"),
        b"",
    ))
    .await;

    let following = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .unwrap();
    let backend = ReqwestBackend::with_client(following);
    let err = backend
        .send(request(url, MAX_JSON_BODY_BYTES))
        .await
        .expect_err("a followed redirect must be reported, not accepted");
    assert_eq!(err.kind(), ErrorKind::Contract);
    assert!(
        err.message().contains("unexpected redirect"),
        "{}",
        err.message()
    );
}

/// The default backend does not follow anything: the 302 comes back as a 3xx for the envelope
/// layer to turn into the "unexpected redirect" error.
#[tokio::test]
async fn the_default_backend_does_not_follow_a_redirect() {
    let url = serve_once(http_response(
        "302 Found",
        "Location: https://evil.example/\r\n",
        b"",
    ))
    .await;
    let backend = ReqwestBackend::new().unwrap();
    let raw = backend
        .send(request(url, MAX_JSON_BODY_BYTES))
        .await
        .unwrap();
    assert_eq!(raw.status, 302);

    let err = oblodai::core::envelope::decode_envelope(
        raw.status,
        &raw.body,
        None,
        raw.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("location"))
            .map(|(_, v)| v.as_str()),
    )
    .unwrap_err();
    assert!(
        err.message().contains("unexpected redirect"),
        "{}",
        err.message()
    );
    assert!(err.message().contains("evil.example"));
}
