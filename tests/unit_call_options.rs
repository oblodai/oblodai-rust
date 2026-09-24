//! Spec §3: explicit call options, the request id, raw responses, client copies, hooks and the
//! printed form of an error — every one on the fake HTTP backend.

// A `Client` only exists with an HTTP backend feature on.
#![cfg(feature = "reqwest-client")]

mod support;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use oblodai::models::{FaucetRequest, HistoryRequest, PaymentRequest};
use oblodai::{Client, ClientOptions, Hooks, HttpBackend, RetryOptions};
use serde_json::json;
use support::{api_error, network_error, no_envelope, ok, sample, MockBackend, Scripted};

fn client_on(
    mock: &Arc<MockBackend>,
    build: impl FnOnce(oblodai::ClientBuilder) -> oblodai::ClientBuilder,
) -> Client {
    build(
        Client::builder()
            .public_id("pk")
            .secret("s")
            .base_url("https://api.test")
            .retry(RetryOptions {
                base_delay_ms: 1,
                max_delay_ms: 2,
                ..Default::default()
            })
            .env(Vec::<(String, String)>::new())
            .http_backend(mock.clone() as Arc<dyn HttpBackend>),
    )
    .build()
    .unwrap()
}

fn payment_ok() -> Scripted {
    ok(sample("payment"))
}

#[tokio::test]
async fn every_call_option_reaches_the_wire() {
    let mock = MockBackend::new(vec![payment_ok()]);
    let client = client_on(&mock, |b| b);
    let invoice = client
        .payments()
        .create(PaymentRequest::new("25.10", "USDT"))
        .idempotency_key("order-1001")
        .timeout(Duration::from_millis(3500))
        .max_retries(0)
        .extra_header("X-Tenant", "eu")
        .request_id("req-1")
        .await
        .unwrap();
    assert_eq!(invoice.uuid, "x");
    let sent = mock.first();
    assert_eq!(sent.header("idempotency-key"), Some("order-1001"));
    assert_eq!(sent.timeout, Duration::from_millis(3500));
    assert_eq!(sent.header("x-tenant"), Some("eu"));
    assert_eq!(sent.header("x-request-id"), Some("req-1"));
    assert_eq!(
        sent.json_body(),
        json!({"amount": "25.10", "currency": "USDT"})
    );
}

#[tokio::test]
async fn max_retries_on_a_call_overrides_the_client() {
    let mock = MockBackend::new(vec![no_envelope(503), no_envelope(503), payment_ok()]);
    let client = client_on(&mock, |b| b);
    let err = client
        .payments()
        .create(PaymentRequest::new("1", "USDT"))
        .max_retries(1)
        .await
        .unwrap_err();
    assert_eq!(mock.call_count(), 2, "one retry, not the client's two");
    assert_eq!(err.http_status(), 503);

    let mock = MockBackend::new(vec![no_envelope(503), payment_ok()]);
    let client = client_on(&mock, |b| b.max_retries(0));
    assert!(client
        .payments()
        .create(PaymentRequest::new("1", "USDT"))
        .await
        .is_err());
    assert_eq!(
        mock.call_count(),
        1,
        "max_retries(0) on the client sends once"
    );
}

#[tokio::test]
async fn one_request_id_names_every_attempt_and_the_error() {
    let mock = MockBackend::new(vec![network_error(), network_error(), network_error()]);
    let client = client_on(&mock, |b| b);
    let err = client.account().get_balance().await.unwrap_err();
    let calls = mock.calls();
    assert_eq!(calls.len(), 3);
    let id = calls[0].header("x-request-id").unwrap().to_string();
    assert_eq!(id.len(), 36, "a fresh UUID when the caller gave none");
    assert!(calls.iter().all(|c| c.header("x-request-id") == Some(&id)));
    assert_eq!(err.request_id(), Some(id.as_str()));

    // A second call gets a new id.
    let mock2 = MockBackend::new(vec![ok(json!({"balance": {"merchant": []}}))]);
    let client2 = client_on(&mock2, |b| b);
    client2.account().get_balance().await.unwrap();
    assert_ne!(mock2.first().header("x-request-id"), Some(id.as_str()));
}

#[tokio::test]
async fn an_error_prints_code_message_and_request_id() {
    let mock = MockBackend::new(vec![api_error(
        400,
        json!({"code": "payment.bad_amount", "message": "bad amount", "retryable": false,
               "request_id": "gw-7"}),
    )]);
    let client = client_on(&mock, |b| b);
    let err = client
        .payments()
        .create(PaymentRequest::new("0", "USDT"))
        .await
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "[payment.bad_amount] bad amount (request_id=gw-7)"
    );

    // Without the gateway's id, the one the SDK sent is the one to quote.
    let mock = MockBackend::new(vec![api_error(
        404,
        json!({"code": "payment.not_found", "message": "no such invoice", "retryable": false}),
    )]);
    let client = client_on(&mock, |b| b);
    let err = client
        .payments()
        .create(PaymentRequest::new("1", "USDT"))
        .request_id("mine-1")
        .await
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "[payment.not_found] no such invoice (request_id=mine-1)"
    );

    // An error raised before sending has no id and no suffix.
    let err = client
        .payments()
        .list_history(HistoryRequest::default())
        .idempotency_key("k")
        .await
        .unwrap_err();
    assert!(
        err.to_string()
            .starts_with("[sdk.idempotency_unsupported] "),
        "{err}"
    );
    assert!(!err.to_string().contains("request_id="), "{err}");
}

#[tokio::test]
async fn a_raw_response_carries_status_headers_and_request_id() {
    let mock = MockBackend::new(vec![payment_ok()
        .header("x-request-id", "gw-1")
        .header("x-rate", "9")]);
    let client = client_on(&mock, |b| b);
    let raw = client
        .payments()
        .create(PaymentRequest::new("25", "USDT"))
        .with_raw_response()
        .await
        .unwrap();
    assert_eq!(raw.status(), 200);
    assert_eq!(raw.request_id(), "gw-1");
    assert_eq!(raw.header("X-Rate"), Some("9"));
    assert!(!raw.body().is_empty());
    assert_eq!(raw.parse().unwrap().uuid, "x");

    // No id in the answer: the one the SDK sent.
    let mock = MockBackend::new(vec![payment_ok()]);
    let client = client_on(&mock, |b| b);
    let raw = client
        .payments()
        .create(PaymentRequest::new("25", "USDT"))
        .request_id("mine-2")
        .with_raw_response()
        .await
        .unwrap();
    assert_eq!(raw.request_id(), "mine-2");

    // A list's raw response is its first page.
    let mock = MockBackend::new(vec![ok(json!({
        "items": [sample("payment")],
        "paginate": {"total": 1, "per_page": 5, "offset": 0, "has_pages": false}
    }))]);
    let client = client_on(&mock, |b| b);
    let raw = client
        .payments()
        .list_history(HistoryRequest::default())
        .limit(5)
        .with_raw_response()
        .await
        .unwrap();
    assert_eq!(raw.parse().unwrap().items.len(), 1);
    assert_eq!(mock.first().json_body()["limit"], 5);
}

#[tokio::test]
async fn with_options_is_a_copy_that_leaves_the_original_alone() {
    let mock = MockBackend::new(vec![payment_ok(), payment_ok()]);
    let client = client_on(&mock, |b| b.timeout(Duration::from_secs(30)));
    let patient = client.with_options(
        ClientOptions::new()
            .timeout(Duration::from_secs(60))
            .extra_header("X-Copy", "1"),
    );
    patient
        .payments()
        .create(PaymentRequest::new("1", "USDT"))
        .await
        .unwrap();
    client
        .payments()
        .create(PaymentRequest::new("1", "USDT"))
        .await
        .unwrap();
    let calls = mock.calls();
    assert_eq!(calls[0].timeout, Duration::from_secs(60));
    assert_eq!(calls[0].header("x-copy"), Some("1"));
    assert_eq!(calls[1].timeout, Duration::from_secs(30));
    assert_eq!(calls[1].header("x-copy"), None);
}

#[tokio::test]
async fn hooks_see_every_attempt_without_the_signature() {
    let seen = Arc::new(Mutex::new(Vec::<String>::new()));
    let (req_log, res_log) = (seen.clone(), seen.clone());
    let hooks = Hooks::new()
        .on_request(move |r| {
            assert_eq!(r.operation_id, "getBalance");
            let signature = r
                .headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("x-signature"))
                .map(|(_, v)| v.clone());
            assert_eq!(signature.as_deref(), Some("[redacted]"));
            req_log.lock().unwrap().push(format!(
                "req #{} {} {}",
                r.attempt,
                r.method,
                r.request_id.len()
            ));
        })
        .on_response(move |r| {
            res_log.lock().unwrap().push(format!(
                "res #{} {} {}",
                r.request.attempt,
                r.status,
                r.error
                    .as_ref()
                    .map(|e| e.code().to_string())
                    .unwrap_or_default()
            ));
        });
    let mock = MockBackend::new(vec![
        no_envelope(503),
        ok(json!({"balance": {"merchant": []}})),
    ]);
    let client = client_on(&mock, |b| b.hooks(hooks));
    client.account().get_balance().await.unwrap();
    assert_eq!(
        *seen.lock().unwrap(),
        [
            "req #1 POST 36",
            "res #1 503 internal",
            "req #2 POST 36",
            "res #2 200 "
        ]
    );
}

/// Ruling 10: on a route the gateway does not deduplicate by header, a body field
/// `idempotency_key` takes the call option, and no `Idempotency-Key` header is sent.
#[tokio::test]
async fn the_faucet_takes_the_idempotency_key_in_its_body() {
    let mock = MockBackend::new(vec![ok(json!({}))]);
    let client = client_on(&mock, |b| b);
    let _ = client
        .sandbox()
        .faucet(FaucetRequest::new("10", "USDT"))
        .idempotency_key("tap-1")
        .send_json()
        .await
        .unwrap();
    let sent = mock.first();
    assert_eq!(sent.header("idempotency-key"), None);
    assert_eq!(
        sent.json_body(),
        json!({"amount": "10", "asset": "USDT", "idempotency_key": "tap-1"})
    );
}

/// The faucet's key given twice — in the request's own field and as the call option — is
/// ambiguous: refused before the network, as in every Oblodai SDK.
#[tokio::test]
async fn the_faucet_key_given_twice_is_an_error_before_the_network() {
    let mock = MockBackend::new(vec![ok(json!({}))]);
    let client = client_on(&mock, |b| b);
    let mut req = FaucetRequest::new("10", "USDT");
    req.idempotency_key = Some("own".into());
    let err = client
        .sandbox()
        .faucet(req)
        .idempotency_key("tap-2")
        .send_json()
        .await
        .unwrap_err();
    assert_eq!(err.code(), "sdk.bad_idempotency_key");
    assert!(err.to_string().contains("idempotency_key"), "{err}");
    assert_eq!(mock.call_count(), 0);
}
