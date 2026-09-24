//! Every example in `examples/` runs here, end to end, against a fake gateway: the code a merchant
//! copies is code that works.

#![cfg(feature = "reqwest-client")]
#![allow(dead_code)]

mod support;

#[path = "../examples/accept_payment.rs"]
mod accept_payment;
#[path = "../examples/payout.rs"]
mod payout;
#[path = "../examples/webhook_receiver.rs"]
mod webhook_receiver;

use std::sync::Arc;

use oblodai::models::{
    BalanceResult, PaymentInfoResult, PayoutCalculation, PayoutItem, PayoutValidateResult,
};
use oblodai::webhooks::Headers;
use oblodai::{sign_webhook, Client, HttpBackend};
use serde_json::{json, Value};
use support::{api_error, ok, sample, MockBackend};

/// The smallest answer a model accepts, with some fields set.
fn model<T: Default + serde::Serialize>(fields: Value) -> Value {
    let mut value = serde_json::to_value(T::default()).unwrap();
    for (k, v) in fields.as_object().unwrap() {
        value[k] = v.clone();
    }
    value
}

fn client_on(mock: &Arc<MockBackend>) -> Client {
    Client::builder()
        .public_id("test_oblodai_1")
        .secret("oblodai_test_1")
        .base_url("https://api.test")
        .env(Vec::<(String, String)>::new())
        .http_backend(mock.clone() as Arc<dyn HttpBackend>)
        .build()
        .unwrap()
}

#[tokio::test(start_paused = true)]
async fn accept_payment_creates_and_watches_an_invoice() {
    let mock = MockBackend::new(vec![
        ok(sample("payment")),
        ok(model::<PaymentInfoResult>(
            json!({"uuid": "x", "status": "created"}),
        )),
        ok(model::<PaymentInfoResult>(
            json!({"uuid": "x", "status": "paid"}),
        )),
    ]);
    accept_payment::run(&client_on(&mock)).await.unwrap();
    let calls = mock.calls();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0].path(), "/v1/payment");
    assert_eq!(calls[0].json_body()["amount"], "25");
    assert_eq!(calls[1].path(), "/v1/payment/info");
    assert_eq!(calls[1].json_body(), json!({"uuid": "x"}));
}

#[tokio::test]
async fn payout_quotes_dry_runs_and_sends() {
    let mock = MockBackend::new(vec![
        ok(model::<BalanceResult>(json!({"balance": {"merchant": [
            {"currency": "USDT", "balance": "100"}
        ]}}))),
        ok(model::<PayoutCalculation>(
            json!({"currency": "USDT", "amount": "10"}),
        )),
        ok(model::<PayoutValidateResult>(json!({"valid": true}))),
        ok(model::<PayoutItem>(
            json!({"uuid": "po-1", "status": "pending"}),
        )),
    ]);
    payout::run(&client_on(&mock)).await.unwrap();
    let calls = mock.calls();
    assert_eq!(calls.len(), 4);
    let create = &calls[3];
    assert_eq!(create.path(), "/v1/payout");
    assert_eq!(
        create.header("idempotency-key"),
        create.json_body()["order_id"].as_str(),
        "the order id doubles as the idempotency key"
    );
}

#[tokio::test(start_paused = true)]
async fn payout_reports_a_retryable_refusal_instead_of_failing() {
    let refusal = || {
        api_error(
            409,
            json!({"code": "payout.insufficient_funds", "message": "not enough", "retryable": true,
                   "retry_after": 1}),
        )
    };
    let mock = MockBackend::new(vec![
        ok(model::<BalanceResult>(json!({}))),
        ok(model::<PayoutCalculation>(json!({}))),
        ok(model::<PayoutValidateResult>(json!({"valid": true}))),
        refusal(),
        refusal(),
        refusal(),
    ]);
    payout::run(&client_on(&mock)).await.unwrap();
    assert_eq!(
        mock.call_count(),
        6,
        "the SDK retried the keyed create, then gave up"
    );
}

#[test]
fn the_webhook_receiver_accepts_a_signed_delivery_once() {
    let secret = "whsec_example";
    let body = serde_json::to_vec(&model::<oblodai::models::PaymentWebhook>(json!({
        "type": "payment", "uuid": "inv-1", "status": "paid", "sequence": 3,
        "amount": "25", "payer_amount": "25", "payment_amount": "25"
    })))
    .unwrap();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let headers = |signature: String| {
        Headers::from_pairs([
            ("X-Webhook-Timestamp".to_string(), ts.to_string()),
            ("X-Webhook-Signature".to_string(), signature),
            ("X-Webhook-Id".to_string(), "d-1".to_string()),
        ])
    };
    let mut receiver = webhook_receiver::Receiver::new(secret.into(), None);
    let signed = headers(sign_webhook(secret, ts, &body));
    assert_eq!(receiver.handle(&signed, &body), (200, "ok"));
    assert_eq!(
        receiver.handle(&signed, &body),
        (200, "ok"),
        "a retry is acknowledged"
    );
    let forged = headers(sign_webhook("not-the-secret", ts, &body));
    assert_eq!(receiver.handle(&forged, &body), (400, "bad signature"));
}
