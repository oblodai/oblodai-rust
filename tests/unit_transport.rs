//! Transport behaviour: signing headers, idempotency, retries, clock skew, URL building.
//! Every test runs against a fake HTTP backend — no network, no gateway.

// A `Client` only exists with an HTTP backend feature on.
#![cfg(feature = "reqwest-client")]

mod support;

use std::sync::Arc;
use std::time::Duration;

use oblodai::core::signing::{
    HEADER_IDEMPOTENCY_KEY, HEADER_PUBLIC_ID, HEADER_SIGNATURE, HEADER_TIMESTAMP,
};
use oblodai::models::{CreateWalletRequest, PaymentRequest, PayoutRequest, SetAccuracyRequest};
use oblodai::{Client, ErrorKind, HttpBackend, RetryOptions};
use serde_json::json;
use support::{api_error, network_error, no_envelope, ok, MockBackend, Scripted};

fn harness(script: Vec<Scripted>) -> (Client, Arc<MockBackend>) {
    let mock = MockBackend::new(script);
    let client = Client::builder()
        .public_id("pk_test_1")
        .secret("secret-1")
        .base_url("https://api.test")
        .retry(RetryOptions {
            base_delay_ms: 1,
            max_delay_ms: 2,
            ..Default::default()
        })
        .env(Vec::<(String, String)>::new())
        .http_backend(mock.clone() as Arc<dyn HttpBackend>)
        .build()
        .unwrap();
    (client, mock)
}

/// Same harness, retries disabled, so a test sees the first failure as the caller would.
fn harness_no_retry(script: Vec<Scripted>) -> (Client, Arc<MockBackend>) {
    let mock = MockBackend::new(script);
    let client = Client::builder()
        .public_id("pk_test_1")
        .secret("secret-1")
        .base_url("https://api.test")
        .retry(RetryOptions {
            max_retries: 0,
            ..Default::default()
        })
        .env(Vec::<(String, String)>::new())
        .http_backend(mock.clone() as Arc<dyn HttpBackend>)
        .build()
        .unwrap();
    (client, mock)
}

fn invoice() -> PaymentRequest {
    PaymentRequest {
        amount: "1".into(),
        currency: "USDT".into(),
        ..Default::default()
    }
}

fn payout() -> PayoutRequest {
    PayoutRequest {
        amount: "1".into(),
        currency: "USDT".into(),
        address: "T".into(),
        order_id: "o".into(),
        ..Default::default()
    }
}

fn balance_ok() -> Scripted {
    ok(json!({ "balance": { "merchant": [] } }))
}

#[tokio::test]
async fn signs_path_and_query_on_get_and_sends_no_body() {
    let (client, mock) = harness(vec![ok(
        json!({ "items": [], "paginate": { "total": 0, "per_page": 10, "offset": 0, "has_pages": false } }),
    )]);
    let _ = client
        .sandbox()
        .list_webhooks(oblodai::generated::resources::SandboxListWebhooksQuery::default())
        .limit(10)
        .offset(0)
        .await
        .unwrap();
    let call = mock.first();
    assert_eq!(
        call.url,
        "https://api.test/v1/sandbox/webhooks?limit=10&offset=0"
    );
    assert_eq!(call.method, "GET");
    assert!(call.body.is_none());
    assert_eq!(call.header(HEADER_PUBLIC_ID), Some("pk_test_1"));
    let signature = call.header(HEADER_SIGNATURE).unwrap();
    assert_eq!(signature.len(), 64);
    assert!(signature.bytes().all(|b| b.is_ascii_hexdigit()));
}

#[tokio::test]
async fn generates_one_idempotency_key_per_call_and_reuses_it_across_retries() {
    let (client, mock) = harness(vec![
        api_error(
            503,
            json!({ "code": "db.unavailable", "message": "down", "retryable": true }),
        ),
        ok(json!({ "uuid": "u" })),
    ]);
    let _ = client
        .payments()
        .create(invoice())
        .send_json()
        .await
        .unwrap();
    let calls = mock.calls();
    assert_eq!(calls.len(), 2);
    let key = calls[0].header(HEADER_IDEMPOTENCY_KEY).unwrap();
    assert_eq!(key.len(), 36, "a v4 uuid");
    assert_eq!(calls[1].header(HEADER_IDEMPOTENCY_KEY), Some(key));
    // Re-signed per attempt: same key, the timestamp may differ but a signature is always present.
    assert_eq!(calls[1].header(HEADER_SIGNATURE).unwrap().len(), 64);
}

#[tokio::test]
async fn honours_a_caller_key_and_adds_none_to_read_routes() {
    let (client, mock) = harness(vec![ok(json!({ "uuid": "u" })), ok(json!({ "uuid": "u" }))]);
    let _ = client
        .payouts()
        .create(payout())
        .idempotency_key("my-key-1")
        .send_json()
        .await
        .unwrap();
    let _ = client
        .payments()
        .get_info(oblodai::models::LookupRequest {
            uuid: Some("u".into()),
            ..Default::default()
        })
        .send_json()
        .await
        .unwrap();
    let calls = mock.calls();
    assert_eq!(calls[0].header(HEADER_IDEMPOTENCY_KEY), Some("my-key-1"));
    assert_eq!(calls[1].header(HEADER_IDEMPOTENCY_KEY), None);
}

#[tokio::test]
async fn rejects_a_caller_key_on_a_route_the_core_does_not_deduplicate() {
    let (client, mock) = harness(vec![]);
    let err = client
        .payouts()
        .approve(oblodai::models::ApproveRequest::new("p1"))
        .idempotency_key("k1")
        .send_json()
        .await
        .unwrap_err();
    assert_eq!(err.code(), "sdk.idempotency_unsupported");
    assert_eq!(err.kind(), ErrorKind::Config);
    assert_eq!(mock.call_count(), 0, "nothing may leave the process");
}

#[tokio::test]
async fn rejects_an_unusable_caller_key_before_signing() {
    let (client, mock) = harness(vec![]);
    let err = client
        .payments()
        .create(invoice())
        .idempotency_key("has space")
        .send_json()
        .await
        .unwrap_err();
    assert_eq!(err.code(), "sdk.bad_idempotency_key");
    assert_eq!(mock.call_count(), 0);
}

#[tokio::test]
async fn does_not_retry_a_non_retryable_error_even_on_a_5xx() {
    let (client, mock) = harness(vec![api_error(
        500,
        json!({ "code": "internal", "retryable": false }),
    )]);
    let err = client.account().get_balance().await.unwrap_err();
    assert_eq!(err.code(), "internal");
    assert_eq!(err.http_status(), 500);
    assert!(!err.retryable());
    assert_eq!(mock.call_count(), 1);
}

#[tokio::test]
async fn retries_a_retryable_error_and_surfaces_it_after_the_budget() {
    let rate_limited = || {
        api_error(
            429,
            json!({ "code": "request.rate_limited", "retryable": true, "retry_after": 0 }),
        )
    };
    let (client, mock) = harness(vec![rate_limited(), rate_limited(), rate_limited()]);
    let err = client.account().get_balance().await.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::RateLimit);
    assert_eq!(err.retry_after(), Some(0));
    assert_eq!(mock.call_count(), 3, "1 attempt + max_retries(2)");
}

#[tokio::test]
async fn retries_a_transport_failure_only_when_the_request_is_safe_to_repeat() {
    // A read route: retried.
    let (client, mock) = harness(vec![network_error(), balance_ok()]);
    client.account().get_balance().await.unwrap();
    assert_eq!(mock.call_count(), 2);

    // A write the core does not deduplicate: never re-sent.
    let (client, mock) = harness(vec![network_error(), ok(json!({}))]);
    let err = client
        .settings()
        .set_accuracy(SetAccuracyRequest {
            enabled: true,
            ..Default::default()
        })
        .send_json()
        .await
        .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Transport);
    assert_eq!(err.code(), "transport.network");
    assert_eq!(mock.call_count(), 1);

    // A keyed write: retried, because the core deduplicates it.
    let (client, mock) = harness(vec![network_error(), ok(json!({ "uuid": "u" }))]);
    let _ = client
        .payments()
        .create(invoice())
        .send_json()
        .await
        .unwrap();
    assert_eq!(mock.call_count(), 2);
}

#[tokio::test]
async fn never_re_sends_an_unsafe_write_after_a_proxy_answer_without_an_envelope() {
    let (client, mock) = harness(vec![no_envelope(503), ok(json!({}))]);
    let err = client
        .payouts()
        .approve(oblodai::models::ApproveRequest::new("p1"))
        .send_json()
        .await
        .unwrap_err();
    assert_eq!(err.http_status(), 503);
    assert!(
        err.synthetic(),
        "no envelope: the core may still have done the work"
    );
    assert!(
        err.retryable(),
        "the flag is honest — but re-sending is not safe here"
    );
    assert_eq!(mock.call_count(), 1);
}

#[tokio::test]
async fn retries_a_read_route_after_a_proxy_502_or_504() {
    let (client, mock) = harness(vec![
        no_envelope(502),
        no_envelope(504).header("retry-after", "0"),
        balance_ok(),
    ]);
    client.account().get_balance().await.unwrap();
    assert_eq!(mock.call_count(), 3);
}

#[tokio::test]
async fn honours_the_retry_after_header_when_the_body_has_none() {
    let (client, _mock) = harness_no_retry(vec![no_envelope(429).header("retry-after", "120")]);
    let err = client.account().get_balance().await.unwrap_err();
    assert_eq!(err.retry_after(), Some(120));
    assert_eq!(err.http_status(), 429);
}

#[tokio::test]
async fn retries_an_enveloped_retryable_error_on_an_unsafe_write() {
    // The core answered, so it did not perform the operation — re-sending cannot duplicate it.
    let (client, mock) = harness(vec![
        api_error(
            409,
            json!({ "code": "payout.funds_maturing", "retryable": true, "retry_after": 0 }),
        ),
        ok(json!({ "uuid": "p" })),
    ]);
    let _ = client
        .payouts()
        .approve(oblodai::models::ApproveRequest::new("p1"))
        .send_json()
        .await
        .unwrap();
    assert_eq!(mock.call_count(), 2);
}

#[tokio::test]
async fn classifies_the_envelope_and_keeps_request_id_and_field() {
    let (client, _) = harness(vec![api_error(
        400,
        json!({
            "code": "payment.below_minimum",
            "message": "too small",
            "field": "amount",
            "retryable": false,
            "request_id": "rq-1"
        }),
    )]);
    let err = client
        .payments()
        .create(invoice())
        .send_json()
        .await
        .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Validation);
    assert_eq!(err.code(), "payment.below_minimum");
    assert_eq!(err.family(), "payment");
    assert_eq!(err.field(), Some("amount"));
    assert_eq!(err.request_id(), Some("rq-1"));
    assert_eq!(err.message(), "too small");

    let (client, _) = harness(vec![api_error(
        401,
        json!({ "code": "merchant.bad_signature", "retryable": false }),
    )]);
    assert_eq!(
        client.account().get_balance().await.unwrap_err().kind(),
        ErrorKind::Authentication
    );

    let (client, _) = harness(vec![api_error(
        409,
        json!({ "code": "idempotency.key_reused", "message": "reused", "retryable": false }),
    )]);
    let err = client
        .payments()
        .create(invoice())
        .send_json()
        .await
        .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::IdempotencyConflict);
}

#[tokio::test]
async fn serializes_an_error_without_the_raw_body() {
    let (client, _) = harness(vec![api_error(
        400,
        json!({ "code": "payment.below_minimum", "message": "too small", "retryable": false }),
    )]);
    let err = client
        .payments()
        .create(invoice())
        .send_json()
        .await
        .unwrap_err();
    assert!(
        err.raw_body().is_some(),
        "the body is available when you ask for it"
    );
    let json = serde_json::to_value(&err).unwrap();
    assert_eq!(json["code"], "payment.below_minimum");
    assert_eq!(json["message"], "too small");
    assert_eq!(json["http_status"], 400);
    assert!(
        json.get("raw").is_none(),
        "the raw body must never reach a log"
    );
    assert!(!format!("{err:?}").contains("payment.below_minimum\",\"message"));
}

#[tokio::test]
async fn re_signs_once_with_the_server_clock_when_a_401_reveals_skew() {
    let server_now = oblodai::core::util::unix_now() + 3600;
    let date =
        httpdate::fmt_http_date(std::time::UNIX_EPOCH + Duration::from_secs(server_now as u64));
    let (client, mock) = harness(vec![
        api_error(
            401,
            json!({ "code": "merchant.bad_signature", "retryable": false }),
        )
        .header("date", date),
        balance_ok(),
    ]);
    client.account().get_balance().await.unwrap();
    let calls = mock.calls();
    assert_eq!(calls.len(), 2);
    let ts: i64 = calls[1].header(HEADER_TIMESTAMP).unwrap().parse().unwrap();
    assert!(
        (ts - server_now).abs() < 5,
        "the second attempt is signed with the server's time"
    );
}

#[tokio::test]
async fn ignores_the_date_header_on_a_401_that_is_not_a_signature_failure() {
    let date = httpdate::fmt_http_date(
        std::time::UNIX_EPOCH
            + Duration::from_secs((oblodai::core::util::unix_now() + 4000) as u64),
    );
    let (client, mock) = harness(vec![api_error(
        401,
        json!({ "code": "auth.ip_not_allowed", "retryable": false }),
    )
    .header("date", date)]);
    let err = client.account().get_balance().await.unwrap_err();
    assert_eq!(err.code(), "auth.ip_not_allowed");
    assert_eq!(mock.call_count(), 1);
}

#[tokio::test]
async fn reverts_the_correction_when_the_re_signed_attempt_is_still_rejected() {
    let date = httpdate::fmt_http_date(
        std::time::UNIX_EPOCH
            + Duration::from_secs((oblodai::core::util::unix_now() + 4000) as u64),
    );
    let bad = || {
        api_error(
            401,
            json!({ "code": "merchant.bad_signature", "retryable": false }),
        )
        .header("date", date.clone())
    };
    let (client, mock) = harness(vec![bad(), bad(), balance_ok()]);
    let err = client.account().get_balance().await.unwrap_err();
    assert_eq!(err.code(), "merchant.bad_signature");
    client.account().get_balance().await.unwrap();
    let calls = mock.calls();
    let ts: i64 = calls[2].header(HEADER_TIMESTAMP).unwrap().parse().unwrap();
    assert!(
        (ts - oblodai::core::util::unix_now()).abs() < 5,
        "one bad Date must not wedge the client"
    );
}

#[tokio::test]
async fn times_out_and_reports_transport_timeout() {
    let (client, _) = harness_no_retry(vec![ok(json!({})).delay(Duration::from_millis(200))]);
    let err = client
        .account()
        .get_balance()
        .timeout(Duration::from_millis(20))
        .await
        .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Transport);
    assert_eq!(err.code(), "transport.timeout");
}

#[tokio::test]
async fn stops_retrying_when_the_deadline_would_be_exceeded() {
    let (client, mock) = harness(vec![
        api_error(
            503,
            json!({ "code": "db.unavailable", "retryable": true, "retry_after": 2 }),
        ),
        ok(json!({})),
    ]);
    let err = client
        .account()
        .get_balance()
        .deadline(Duration::from_millis(100))
        .await
        .unwrap_err();
    assert_eq!(err.code(), "transport.deadline");
    assert_eq!(mock.call_count(), 1);
}

#[tokio::test]
async fn names_the_redirect_target_instead_of_a_bare_envelope_error() {
    let (client, _) = harness(vec![Scripted {
        status: 301,
        body: String::new(),
        headers: vec![("location".into(), "https://www.api.test/v1/balance".into())],
        ..Default::default()
    }]);
    let err = client.account().get_balance().await.unwrap_err();
    assert_eq!(err.http_status(), 301);
    assert!(err.message().contains("redirect"), "{}", err.message());
    assert!(err.message().contains("www.api.test"));
}

/// One API key signs everything: money out and money in are the same credential.
#[tokio::test]
async fn signs_every_route_with_the_one_api_key() {
    let mock = MockBackend::new(vec![ok(json!({ "uuid": "p" })), ok(json!({ "uuid": "i" }))]);
    let client = Client::builder()
        .public_id("oblodai_test_1")
        .secret("secret-1")
        .base_url("https://api.test")
        .env(Vec::<(String, String)>::new())
        .http_backend(mock.clone() as Arc<dyn HttpBackend>)
        .build()
        .unwrap();
    let _ = client.payouts().create(payout()).send_json().await.unwrap();
    let _ = client
        .payments()
        .create(invoice())
        .send_json()
        .await
        .unwrap();
    let calls = mock.calls();
    assert_eq!(calls[0].header(HEADER_PUBLIC_ID), Some("oblodai_test_1"));
    assert_eq!(calls[1].header(HEADER_PUBLIC_ID), Some("oblodai_test_1"));
}

#[tokio::test]
async fn refuses_a_signed_call_with_no_credentials_but_allows_public_routes() {
    let mock = MockBackend::new(vec![ok(
        json!({ "currencies": [], "pricing_currencies": [] }),
    )]);
    let client = Client::builder()
        .base_url("https://api.test")
        .env(Vec::<(String, String)>::new())
        .http_backend(mock.clone() as Arc<dyn HttpBackend>)
        .build()
        .unwrap();
    client.checkout().list_currencies().await.unwrap();
    let err = client.account().get_balance().await.unwrap_err();
    assert_eq!(err.code(), "sdk.missing_credentials");
    assert_eq!(mock.call_count(), 1);
}

#[tokio::test]
async fn keeps_a_path_prefix_on_the_base_url() {
    let mock = MockBackend::new(vec![balance_ok()]);
    let client = Client::builder()
        .public_id("pk")
        .secret("s")
        .base_url("https://gw.corp/oblodai/")
        .env(Vec::<(String, String)>::new())
        .http_backend(mock.clone() as Arc<dyn HttpBackend>)
        .build()
        .unwrap();
    client.account().get_balance().await.unwrap();
    assert_eq!(mock.first().url, "https://gw.corp/oblodai/v1/balance");
}

#[tokio::test]
async fn drops_caller_headers_that_collide_with_signed_headers() {
    let mock = MockBackend::new(vec![balance_ok()]);
    let client = Client::builder()
        .public_id("pk")
        .secret("s")
        .base_url("https://api.test")
        .header(HEADER_SIGNATURE, "zz")
        .header("X-Trace", "t1")
        .env(Vec::<(String, String)>::new())
        .http_backend(mock.clone() as Arc<dyn HttpBackend>)
        .build()
        .unwrap();
    client.account().get_balance().await.unwrap();
    let call = mock.first();
    assert_eq!(call.header(HEADER_SIGNATURE).unwrap().len(), 64);
    assert_eq!(call.header("x-trace"), Some("t1"));
}

#[tokio::test]
async fn refuses_path_parameters_that_would_rewrite_the_url() {
    let (client, mock) = harness(vec![]);
    for bad in ["..", ".", "a/b", ""] {
        let err = client.checkout().get(bad).send_json().await.unwrap_err();
        assert_eq!(err.code(), "sdk.bad_path_param", "for {bad:?}");
    }
    assert_eq!(mock.call_count(), 0);
}

#[tokio::test]
async fn percent_encodes_a_path_parameter() {
    let (client, mock) = harness(vec![ok(json!({}))]);
    let _ = client.checkout().get("a b").send_json().await;
    assert_eq!(mock.first().path(), "/v1/pay/a%20b");
}

#[tokio::test]
async fn sends_uuid_for_document_reports_keyed_by_batch_or_link_id() {
    let (client, mock) = harness(vec![Scripted {
        status: 200,
        body: "%PDF".into(),
        headers: vec![("content-type".into(), "application/pdf".into())],
        ..Default::default()
    }]);
    let file = client
        .documents()
        .get_batch(oblodai::generated::resources::GetBatchDocumentQuery::new(
            "b-1",
        ))
        .await
        .unwrap();
    assert_eq!(mock.first().query("uuid").as_deref(), Some("b-1"));
    assert_eq!(file.content_type, "application/pdf");
    assert_eq!(file.bytes, b"%PDF");
}

#[tokio::test]
async fn reads_the_filename_from_content_disposition() {
    let (client, _) = harness(vec![Scripted {
        status: 200,
        body: "%PDF".into(),
        headers: vec![
            ("content-type".into(), "application/pdf".into()),
            (
                "content-disposition".into(),
                "attachment; filename=\"statement.pdf\"".into(),
            ),
        ],
        ..Default::default()
    }]);
    let file = client
        .documents()
        .get_balance(oblodai::generated::resources::GetBalanceDocumentQuery::default())
        .await
        .unwrap();
    assert_eq!(file.filename.as_deref(), Some("statement.pdf"));
}

#[tokio::test]
async fn surfaces_an_idempotent_replay_the_core_could_not_cache() {
    let (client, _) = harness(vec![ok(
        json!({ "ok": true, "idempotent_replay": true, "detail": "response too large" }),
    )]);
    let err = client
        .payments()
        .create(invoice())
        .send_json()
        .await
        .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Contract);
    assert!(
        err.message().contains("already processed"),
        "{}",
        err.message()
    );
}

#[tokio::test]
async fn a_success_body_that_is_not_the_envelope_is_a_contract_error() {
    let (client, _) = harness(vec![Scripted {
        status: 200,
        body: r#"{"whatever": 1}"#.into(),
        headers: vec![("content-type".into(), "application/json".into())],
        ..Default::default()
    }]);
    let err = client
        .wallets()
        .create(CreateWalletRequest::default())
        .send_json()
        .await
        .unwrap_err();
    assert_eq!(err.code(), "sdk.bad_envelope");
}

/// `batch/info` is an ordinary signed call: one key, one attempt, no key-kind dance.
#[tokio::test]
async fn batch_info_signs_with_the_api_key_and_does_not_retry_a_refusal() {
    let mock = MockBackend::new(vec![
        api_error(
            403,
            json!({ "code": "merchant.no_access", "retryable": false }),
        ),
        ok(json!({
            "batch_id": "b1", "kind": "payout", "status": "done", "on_error": "continue",
            "total": 0, "succeeded": 0, "failed": 0, "items": [],
            "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"
        })),
    ]);
    let client = Client::builder()
        .public_id("pk")
        .secret("s")
        .base_url("https://api.test")
        .env(Vec::<(String, String)>::new())
        .http_backend(mock.clone() as Arc<dyn HttpBackend>)
        .build()
        .unwrap();
    let err = client
        .batches()
        .get_info(oblodai::models::BatchInfoRequest {
            batch_id: "b1".into(),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert_eq!(err.code(), "merchant.no_access");
    let calls = mock.calls();
    assert_eq!(
        calls.len(),
        1,
        "a refusal is surfaced, not retried with another key"
    );
    assert_eq!(calls[0].header(HEADER_PUBLIC_ID), Some("pk"));
}
