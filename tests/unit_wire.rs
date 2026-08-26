//! Regressions on what actually leaves the process: headers, secrets in logs and in `Debug`,
//! and the shared clock under concurrency.

// A `Client` only exists with an HTTP backend feature on.
#![cfg(feature = "reqwest-client")]
// The SDK's error carries the gateway's whole envelope by value, exactly as callers match on it;
// boxing it here would only make the test disagree with the crate it exercises.
#![allow(clippy::result_large_err)]

mod support;

use std::sync::{Arc, Mutex};

use oblodai::contract::models::{ApiKeyPair, PayoutLink, WebhookEndpoint, WebhookSecretRotated};
use oblodai::core::engine::RawResponse;
use oblodai::{
    BackendFuture, Client, ErrorKind, HttpBackend, HttpRequest, LogLevel, Logger, Result,
    RetryOptions,
};
use serde_json::json;
use support::{ok, MockBackend, Scripted};

fn client_with(
    mock: Arc<MockBackend>,
    build: impl FnOnce(oblodai::ClientBuilder) -> oblodai::ClientBuilder,
) -> Client {
    build(
        Client::builder()
            .public_id("pk")
            .secret("s")
            .payout_public_id("wk")
            .payout_secret("s2")
            .base_url("https://api.test")
            .env(Vec::<(String, String)>::new()),
    )
    .http_backend(mock as Arc<dyn HttpBackend>)
    .build()
    .unwrap()
}

fn balance_ok() -> Scripted {
    ok(json!({ "balance": { "merchant": [] } }))
}

// --- headers ----------------------------------------------------------------------------------

/// `reqwest` appends headers rather than replacing them, so a caller header the SDK also sets used
/// to put BOTH values on the wire: two `accept` lines, two `user-agent` lines, and — worst — a
/// second `x-admin-token` next to the configured one.
#[tokio::test]
async fn caller_headers_never_duplicate_the_ones_the_sdk_owns() {
    let mock = MockBackend::new(vec![balance_ok()]);
    let client = client_with(mock.clone(), |b| {
        b.header("Accept", "text/csv")
            .header("accept", "application/xml")
            .header("User-Agent", "evil/1")
            .header("X-Admin-Token", "stolen")
            .header("X-Signature", "forged")
            .header("Idempotency-Key", "forged")
            .header("Content-Type", "text/plain")
    });
    client.account().balance().await.unwrap();

    let sent = mock.first();
    let count = |name: &str| sent.headers.iter().filter(|(k, _)| k == name).count();
    assert_eq!(count("accept"), 1);
    assert_eq!(sent.header("accept"), Some("application/json"));
    assert_eq!(count("user-agent"), 1);
    assert!(sent
        .header("user-agent")
        .unwrap()
        .starts_with("oblodai-rust/"));
    assert_eq!(count("content-type"), 1);
    assert_eq!(sent.header("content-type"), Some("application/json"));
    assert_eq!(count("x-signature"), 1);
    assert_eq!(sent.header("x-signature").unwrap().len(), 64);
    assert_eq!(count("idempotency-key"), 0, "balance is not deduplicated");
    assert_eq!(count("x-admin-token"), 0, "not an onboard route");
}

#[tokio::test]
async fn the_admin_token_goes_only_to_onboard_routes() {
    let mock = MockBackend::new(vec![balance_ok()]);
    let client = client_with(mock.clone(), |b| b.admin_token("adm"));
    client.account().balance().await.unwrap();
    assert_eq!(mock.first().header("x-admin-token"), None);

    let mock = MockBackend::new(vec![ok(json!({
        "merchant_id": "m", "project_id": "p",
        "api_key": {"public_id":"a","secret":"b","kind":"api"},
        "payment_key": {"public_id":"a","secret":"b","kind":"api"},
        "payout_key": {"public_id":"a","secret":"b","kind":"api"}
    }))]);
    let client = client_with(mock.clone(), |b| b.admin_token("adm"));
    let _ = client.merchants().create(Default::default()).await;
    assert_eq!(mock.first().header("x-admin-token"), Some("adm"));
}

#[tokio::test]
async fn a_caller_header_given_twice_collapses_to_the_last_value() {
    let mock = MockBackend::new(vec![balance_ok()]);
    let client = client_with(mock.clone(), |b| {
        b.header("X-Trace", "first").header("x-trace", "second")
    });
    client.account().balance().await.unwrap();
    let sent = mock.first();
    assert_eq!(
        sent.headers.iter().filter(|(k, _)| k == "x-trace").count(),
        1
    );
    assert_eq!(sent.header("x-trace"), Some("second"));
}

#[tokio::test]
async fn a_header_value_with_crlf_or_non_ascii_is_refused_before_sending() {
    for (name, value) in [
        ("X-Trace", "ok\r\nX-Injected: yes"),
        ("X-Trace", "ok\nX-Injected: yes"),
        ("X-Trace", "naïve"),
        ("X-Bad\nName", "ok"),
    ] {
        let mock = MockBackend::new(vec![balance_ok()]);
        let client = client_with(mock.clone(), |b| b.header(name, value));
        let err = client.account().balance().await.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Config, "{name}: {value:?}");
        assert_eq!(err.code(), "sdk.bad_header", "{name}: {value:?}");
        assert_eq!(mock.call_count(), 0, "nothing left the process");
    }
}

// --- secrets ----------------------------------------------------------------------------------

#[test]
fn one_time_secrets_are_never_printed_by_debug() {
    let endpoint = WebhookEndpoint {
        endpoint_id: "ep".into(),
        url: "https://shop.example/hook".into(),
        secret: Some("whsec_LIVE_SECRET".into()),
    };
    let shown = format!("{endpoint:?}");
    assert!(!shown.contains("whsec_LIVE_SECRET"), "{shown}");
    assert!(shown.contains("[redacted]"), "{shown}");
    // Serialization still carries it: the merchant has to be able to store what was shown once.
    assert!(serde_json::to_string(&endpoint)
        .unwrap()
        .contains("whsec_LIVE_SECRET"));

    let rotated = WebhookSecretRotated {
        endpoint_id: "ep".into(),
        url: "u".into(),
        secret: "whsec_NEW".into(),
        previous_secret_valid_until: "2026-01-01T00:00:00Z".into(),
    };
    assert!(!format!("{rotated:?}").contains("whsec_NEW"));

    let keys = ApiKeyPair {
        public_id: "pk_live_1".into(),
        secret: "SIGNING_KEY".into(),
        kind: "api".into(),
    };
    let shown = format!("{keys:?}");
    assert!(!shown.contains("SIGNING_KEY"), "{shown}");
    assert!(
        shown.contains("pk_live_1"),
        "the public half is still useful"
    );

    let link = PayoutLink {
        link_id: "lnk".into(),
        claim_token: Some("CLAIM_TOKEN".into()),
        claim_url: Some("https://pay.example/claim/CLAIM_TOKEN".into()),
        passcode: Some("4821".into()),
        ..Default::default()
    };
    let shown = format!("{link:?}");
    assert!(!shown.contains("CLAIM_TOKEN"), "{shown}");
    assert!(!shown.contains("4821"), "{shown}");
    assert!(shown.contains("lnk"), "the id is still there");
    // A link with no secrets says so, rather than pretending it had them.
    assert!(format!("{:?}", PayoutLink::default()).contains("claim_token: None"));
}

#[derive(Default)]
struct SpyLogger {
    lines: Mutex<Vec<String>>,
}

impl Logger for SpyLogger {
    fn log(&self, _level: LogLevel, message: &str, fields: &[(&str, String)]) {
        let rendered = fields
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(" ");
        self.lines
            .lock()
            .unwrap()
            .push(format!("{message} {rendered}"));
    }
}

/// The module doc claimed values were redacted "before they reach the logger"; they were redacted
/// inside `StderrLogger`, so a logger supplied by the caller saw the raw values.
#[tokio::test]
async fn a_caller_supplied_logger_only_ever_sees_redacted_values() {
    let spy = Arc::new(SpyLogger::default());
    let mock = MockBackend::new(vec![
        support::api_error(401, json!({ "code": "merchant.bad_signature" })),
        support::api_error(401, json!({ "code": "merchant.bad_signature" })),
        support::api_error(401, json!({ "code": "merchant.bad_signature" })),
    ]);
    let client = client_with(mock, |b| {
        b.logger(spy.clone() as Arc<dyn Logger>)
            .retry(RetryOptions {
                base_delay_ms: 1,
                max_delay_ms: 1,
                ..Default::default()
            })
    });
    let _ = client.account().balance().await;
    let lines = spy.lines.lock().unwrap().clone();
    assert!(!lines.is_empty(), "the logger was used at all");
    for line in &lines {
        assert!(!line.contains("signature="), "{line}");
    }
    // The sensitive-looking keys the SDK does log are replaced, not dropped.
    assert!(lines.iter().any(|l| l.starts_with("request ")), "{lines:?}");
}

// --- the shared clock under concurrency -------------------------------------------------------

/// A backend that behaves like a core whose clock is an hour ahead: it refuses any timestamp
/// outside ±300 s and reveals its own time in `Date`, exactly once per wrong-timestamp attempt.
struct SkewedCore {
    server_now: i64,
    refusals: Mutex<usize>,
}

impl HttpBackend for SkewedCore {
    fn send(&self, request: HttpRequest) -> BackendFuture<'_> {
        let ts: i64 = request
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("x-timestamp"))
            .map(|(_, v)| v.parse().unwrap())
            .expect("a signed request");
        let in_window = (ts - self.server_now).abs() <= 300;
        if !in_window {
            *self.refusals.lock().unwrap() += 1;
        }
        let date = httpdate::fmt_http_date(
            std::time::UNIX_EPOCH + std::time::Duration::from_secs(self.server_now as u64),
        );
        Box::pin(async move {
            if in_window {
                Ok(RawResponse {
                    status: 200,
                    headers: vec![("content-type".into(), "application/json".into())],
                    body: br#"{"state":0,"result":{"balance":{"merchant":[]}}}"#.to_vec(),
                })
            } else {
                Ok(RawResponse {
                    status: 401,
                    headers: vec![
                        ("content-type".into(), "application/json".into()),
                        ("date".into(), date),
                    ],
                    body: br#"{"error":{"code":"merchant.bad_signature","retryable":false}}"#
                        .to_vec(),
                })
            }
        })
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_calls_share_one_clock_correction() {
    let backend = Arc::new(SkewedCore {
        server_now: oblodai::core::util::unix_now() + 3600,
        refusals: Mutex::new(0),
    });
    let client = Client::builder()
        .public_id("pk")
        .secret("s")
        .base_url("https://api.test")
        .env(Vec::<(String, String)>::new())
        .http_backend(backend.clone() as Arc<dyn HttpBackend>)
        .build()
        .unwrap();

    let calls: Vec<_> = (0..16)
        .map(|_| {
            let client = client.clone();
            tokio::spawn(async move { client.account().balance().await })
        })
        .collect();
    for call in calls {
        call.await
            .unwrap()
            .expect("every call succeeds after the correction");
    }

    // Each call may discover the skew for itself, but never more than once.
    let first_wave = *backend.refusals.lock().unwrap();
    assert!(
        first_wave <= 16,
        "one refusal per call at most, got {first_wave}"
    );

    // The correction is shared and survives: a second wave signs correctly on the first attempt.
    // Before the fix, a call that measured the same offset another call had already installed
    // compared against the *current* shared offset, decided it was not skew, and then reverted the
    // shared offset out from under its neighbours.
    let calls: Vec<_> = (0..8)
        .map(|_| {
            let client = client.clone();
            tokio::spawn(async move { client.account().balance().await })
        })
        .collect();
    for call in calls {
        call.await
            .unwrap()
            .expect("the shared correction still holds");
    }
    assert_eq!(
        *backend.refusals.lock().unwrap(),
        first_wave,
        "the second wave was signed with the corrected clock, with no further refusals"
    );
}

// --- alias parity -----------------------------------------------------------------------------

#[tokio::test]
async fn payment_link_get_is_info_with_the_same_signature_and_paging() {
    use oblodai::resources::PageParams;
    let link = json!({
        "link_id": "l1", "url": "u", "active": true, "title": "", "description": "",
        "amount_mode": "open", "currency": "USDT", "document_url": "", "created_at": ""
    });
    for which in ["info", "get"] {
        let mock = MockBackend::new(vec![ok(link.clone())]);
        let client = client_with(mock.clone(), |b| b);
        let page = PageParams::default().limit(10).offset(20);
        let _ = match which {
            "info" => client.payment_links().info("l1", page).await,
            _ => client.payment_links().get("l1", page).await,
        };
        let body = mock.first().json_body();
        assert_eq!(body["link_id"], "l1", "{which}");
        assert_eq!(body["limit"], 10, "{which}");
        assert_eq!(body["offset"], 20, "{which}");
    }
}

#[tokio::test]
async fn the_sandbox_webhook_inspector_pages_like_every_other_list() {
    use oblodai::resources::PageParams;
    let mock = MockBackend::new(vec![ok(json!({
        "items": [], "paginate": {"total":0,"per_page":5,"offset":0,"has_pages":false}
    }))]);
    let client = client_with(mock.clone(), |b| b);
    let _ = client
        .sandbox()
        .webhooks(PageParams::default().limit(5).offset(15))
        .await;
    let sent = mock.first();
    assert_eq!(sent.query("limit").as_deref(), Some("5"));
    assert_eq!(sent.query("offset").as_deref(), Some("15"));
}

#[tokio::test]
async fn a_file_builder_takes_the_same_per_call_options_as_a_request_builder() -> Result<()> {
    let mock = MockBackend::new(vec![Scripted {
        status: 200,
        body: "%PDF".into(),
        headers: vec![("content-type".into(), "application/pdf".into())],
        ..Default::default()
    }]);
    let client = client_with(mock.clone(), |b| b);
    let file = client
        .payout_links()
        .cheque(Default::default())
        .prefer_payout_key(true)
        .timeout(std::time::Duration::from_secs(5))
        .deadline(std::time::Duration::from_secs(10))
        .await?;
    assert_eq!(file.content_type, "application/pdf");
    assert_eq!(mock.first().header("x-public-id"), Some("wk"));
    Ok(())
}
