//! The synchronous client drives the same pure core over a blocking backend.

#![cfg(feature = "blocking")]

mod support;

use std::sync::Arc;

use oblodai::blocking::Client;
use oblodai::contract::requests::{PaymentHistoryRequest, PaymentRequest};
use oblodai::{BlockingHttpBackend, ErrorKind, RetryOptions};
use serde_json::json;
use support::{api_error, ok, MockBackend, Scripted};

fn harness(script: Vec<Scripted>) -> (Client, Arc<MockBackend>) {
    let mock = MockBackend::new(script);
    let client = Client::builder()
        .public_id("pk")
        .secret("s")
        .base_url("https://api.test")
        .retry(RetryOptions {
            base_delay_ms: 1,
            max_delay_ms: 2,
            ..Default::default()
        })
        .env(Vec::<(String, String)>::new())
        .blocking_http_backend(mock.clone() as Arc<dyn BlockingHttpBackend>)
        .build_blocking()
        .unwrap();
    (client, mock)
}

fn recorded_payment() -> serde_json::Value {
    support::result_of("POST /v1/payment")
}

#[test]
fn signs_generates_a_key_and_decodes_the_same_way_the_async_client_does() {
    let (client, mock) = harness(vec![ok(recorded_payment())]);
    let invoice = client
        .payments()
        .create(PaymentRequest {
            amount: "25".into(),
            currency: "USDT".into(),
            ..Default::default()
        })
        .send()
        .unwrap();
    assert!(!invoice.uuid.is_empty());
    let call = mock.first();
    assert_eq!(call.header("x-public-id"), Some("pk"));
    assert_eq!(call.header("x-signature").unwrap().len(), 64);
    assert_eq!(call.header("idempotency-key").unwrap().len(), 36);
}

#[test]
fn retries_and_classification_are_the_same_decisions() {
    // A read route survives a transport failure.
    let (client, mock) = harness(vec![
        support::network_error(),
        ok(json!({ "balance": { "merchant": [] } })),
    ]);
    client.account().balance().send().unwrap();
    assert_eq!(mock.call_count(), 2);

    // An unsafe write after a proxy answer with no envelope is never re-sent.
    let (client, mock) = harness(vec![support::no_envelope(503), ok(json!({}))]);
    let err = client.payouts().approve("p1").send_raw().unwrap_err();
    assert!(err.synthetic());
    assert_eq!(mock.call_count(), 1);

    // And a caller key on a route the core does not deduplicate never leaves the process.
    let (client, mock) = harness(vec![]);
    let err = client
        .payouts()
        .approve("p1")
        .idempotency_key("k")
        .send_raw()
        .unwrap_err();
    assert_eq!(err.code(), "sdk.idempotency_unsupported");
    assert_eq!(err.kind(), ErrorKind::Config);
    assert_eq!(mock.call_count(), 0);
}

#[test]
fn pages_walk_with_a_plain_iterator() {
    let item = || support::result_of("POST /v1/payment/history")["items"][0].clone();
    let page = |items: Vec<serde_json::Value>, offset: i64, total: i64| {
        let count = items.len() as i64;
        ok(json!({
            "items": items,
            "paginate": {
                "total": total, "per_page": 1, "offset": offset,
                "has_pages": offset + count < total
            }
        }))
    };
    let (client, mock) = harness(vec![page(vec![item()], 0, 2), page(vec![item()], 1, 2)]);
    let pager = client
        .payments()
        .history(PaymentHistoryRequest::default())
        .limit(1);
    let collected: Vec<_> = pager.iter().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(collected.len(), 2);
    assert_eq!(mock.call_count(), 2);

    let (client, mock) = harness(vec![page(vec![], 0, 0)]);
    assert!(client
        .payments()
        .history(PaymentHistoryRequest::default())
        .page()
        .unwrap()
        .items
        .is_empty());
    assert_eq!(mock.call_count(), 1);
}

#[test]
fn an_error_envelope_is_classified_identically() {
    let (client, _) = harness(vec![api_error(
        409,
        json!({ "code": "idempotency.key_reused", "message": "reused", "retryable": false }),
    )]);
    let err = client
        .payments()
        .create(PaymentRequest {
            amount: "1".into(),
            currency: "USDT".into(),
            ..Default::default()
        })
        .send_raw()
        .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::IdempotencyConflict);
    assert_eq!(err.code(), "idempotency.key_reused");
}
