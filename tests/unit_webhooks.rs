//! Webhook verification, against real signed deliveries and against the rules.

mod support;

use oblodai::webhooks::{
    is_stale_event, is_test_event, parse_webhook, verify_webhook, verify_webhook_delivery, Headers,
    VerifyOptions,
};
use oblodai::{sign_webhook, ErrorKind, WebhookEvent};
use support::{load_webhook_samples, result_of};

/// The samples were delivered by the core's real dispatcher, signed with the endpoint secret in
/// force at that moment — the one the rotate-secret call returned.
fn endpoint_secret() -> String {
    result_of("POST /v1/webhooks/rotate-secret")["secret"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn verifies_every_recorded_delivery() {
    let secret = endpoint_secret();
    let samples = load_webhook_samples();
    assert!(!samples.is_empty());
    for sample in &samples {
        let raw = sample.raw_bytes();
        let ts: i64 = sample
            .header("X-Webhook-Timestamp")
            .unwrap()
            .parse()
            .unwrap();
        let headers = Headers::from_pairs(sample.headers.clone());
        let event_name = sample.header("X-Webhook-Event").unwrap();

        let delivery =
            verify_webhook_delivery(&raw, &headers, &VerifyOptions::new(&secret).now(ts))
                .unwrap_or_else(|e| panic!("{event_name}: {e}"));
        assert_eq!(delivery.event.uuid(), sample.body["uuid"].as_str().unwrap());
        assert_eq!(delivery.id.as_deref(), sample.header("X-Webhook-Id"));
        assert_eq!(delivery.event_type.as_deref(), Some(event_name));
        assert_eq!(delivery.sent_at, ts);
        assert_eq!(
            delivery.event.event_kind(),
            sample.body["type"].as_str().unwrap()
        );
        assert!(
            event_name.starts_with("invoice.")
                || event_name.starts_with("payout.")
                || event_name.starts_with("wallet."),
            "{event_name}"
        );
        // Rehearsal deliveries (`webhooks.test`, sandbox) are signed like live ones, carry
        // `test: true` and `X-Webhook-Test: true`, and have no ledger sequence.
        let is_test = sample.body["test"].as_bool() == Some(true);
        assert_eq!(delivery.is_test, is_test, "{event_name}");
        assert_eq!(is_test_event(&delivery.event), is_test, "{event_name}");
        assert_eq!(
            sample.header("X-Webhook-Test").is_some(),
            is_test,
            "{event_name}"
        );
        if is_test {
            assert_eq!(delivery.event.sequence(), 0, "{event_name}");
        } else {
            assert!(delivery.event.sequence() > 0, "{event_name}");
        }

        // The same bytes under a different secret must not verify.
        let err = verify_webhook(
            &raw,
            &headers,
            &VerifyOptions::new("some-other-secret")
                .previous_secret("another")
                .now(ts),
        )
        .unwrap_err();
        assert_eq!(err.code(), "webhook.bad_signature");
    }
}

#[test]
fn a_recorded_delivery_re_serialized_still_carries_every_field() {
    for sample in load_webhook_samples() {
        let event: WebhookEvent = parse_webhook(&sample.raw_bytes()).unwrap();
        let round_tripped = serde_json::to_value(&event).unwrap();
        assert_eq!(
            round_tripped,
            sample.body,
            "the event model lost or invented a field on {}",
            sample.header("X-Webhook-Event").unwrap_or("?")
        );
    }
}

#[test]
fn the_recorded_rehearsal_deliveries_are_flagged_and_the_live_ones_are_not() {
    let samples = load_webhook_samples();
    let rehearsals = samples
        .iter()
        .filter(|s| s.body["test"].as_bool() == Some(true))
        .count();
    assert_eq!(rehearsals, 4, "the snapshot records four rehearsals");
    assert!(samples.len() > rehearsals, "and live deliveries besides");
}

const TS: i64 = 1_755_600_000;

fn body() -> Vec<u8> {
    serde_json::json!({
        "type": "payment",
        "uuid": "u1",
        "order_id": "o",
        "status": "paid",
        "is_final": true,
        "amount": "25",
        "currency": "USDT",
        "network": "tron",
        "payer_amount": "25",
        "payer_currency": "USDT",
        "payment_amount": "25",
        "payer_address": "T",
        "payer_address_is_refundable": true,
        "additional_data": "",
        "txid": "tx",
        "event_at": "2026-01-01T00:00:00Z",
        "sequence": 7
    })
    .to_string()
    .into_bytes()
}

fn headers_for(secret: &str) -> Headers {
    let mut headers = Headers::new();
    headers.insert("x-webhook-timestamp", TS.to_string());
    headers.insert("x-webhook-signature", sign_webhook(secret, TS, &body()));
    headers
}

#[test]
fn accepts_a_valid_signature_with_case_insensitive_headers() {
    let event = verify_webhook(
        &body(),
        &headers_for("whsec"),
        &VerifyOptions::new("whsec").now(TS),
    )
    .unwrap();
    assert_eq!(event.event_kind(), "payment");
    assert_eq!(event.status(), "paid");
    assert!(event.is_final());
    assert_eq!(event.order_id(), Some("o"));
}

#[test]
fn rejects_a_wrong_secret_a_tampered_body_and_a_missing_header() {
    let err = verify_webhook(
        &body(),
        &headers_for("whsec"),
        &VerifyOptions::new("other").now(TS),
    )
    .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Signature);
    assert_eq!(err.code(), "webhook.bad_signature");

    let tampered = String::from_utf8(body())
        .unwrap()
        .replace("\"paid\"", "\"paid_over\"");
    let err = verify_webhook(
        tampered.as_bytes(),
        &headers_for("whsec"),
        &VerifyOptions::new("whsec").now(TS),
    )
    .unwrap_err();
    assert_eq!(err.code(), "webhook.bad_signature");

    let mut only_signature = Headers::new();
    only_signature.insert("x-webhook-signature", "aa");
    let err = verify_webhook(
        &body(),
        &only_signature,
        &VerifyOptions::new("whsec").now(TS),
    )
    .unwrap_err();
    assert_eq!(err.code(), "webhook.missing_header");
}

#[test]
fn rejects_a_stale_delivery_unless_the_tolerance_is_disabled() {
    let err = verify_webhook(
        &body(),
        &headers_for("whsec"),
        &VerifyOptions::new("whsec").now(TS + 600),
    )
    .unwrap_err();
    assert_eq!(err.code(), "webhook.stale_timestamp");

    let event = verify_webhook(
        &body(),
        &headers_for("whsec"),
        &VerifyOptions::new("whsec")
            .now(TS + 600)
            .tolerance_seconds(0),
    )
    .unwrap();
    assert_eq!(event.uuid(), "u1");

    // Within the ±300 s window it passes.
    verify_webhook(
        &body(),
        &headers_for("whsec"),
        &VerifyOptions::new("whsec").now(TS + 299),
    )
    .unwrap();
}

#[test]
fn verifies_during_a_rotation_from_either_side() {
    let mut rotated = Headers::new();
    rotated.insert("x-webhook-timestamp", TS.to_string());
    rotated.insert("x-webhook-signature", sign_webhook("new", TS, &body()));
    rotated.insert("x-webhook-signature-prev", sign_webhook("old", TS, &body()));

    // The merchant has not swapped its stored secret yet: the Prev header carries it.
    assert_eq!(
        verify_webhook(&body(), &rotated, &VerifyOptions::new("old").now(TS))
            .unwrap()
            .uuid(),
        "u1"
    );
    // The merchant already swapped: the main header verifies.
    assert_eq!(
        verify_webhook(&body(), &rotated, &VerifyOptions::new("new").now(TS))
            .unwrap()
            .uuid(),
        "u1"
    );
    // Or it keeps both, in either slot.
    assert_eq!(
        verify_webhook(
            &body(),
            &rotated,
            &VerifyOptions::new("unrelated")
                .previous_secret("old")
                .now(TS)
        )
        .unwrap()
        .uuid(),
        "u1"
    );
}

#[test]
fn a_rehearsal_is_recognised_from_either_the_header_or_the_body() {
    // Neither: a live delivery.
    let delivery = verify_webhook_delivery(
        &body(),
        &headers_for("whsec"),
        &VerifyOptions::new("whsec").now(TS),
    )
    .unwrap();
    assert!(!delivery.is_test);
    assert!(!is_test_event(&delivery.event));

    // The header alone — the body of an older rehearsal did not carry the flag.
    let mut with_header = headers_for("whsec");
    with_header.insert("x-webhook-test", "true");
    let delivery =
        verify_webhook_delivery(&body(), &with_header, &VerifyOptions::new("whsec").now(TS))
            .unwrap();
    assert!(delivery.is_test);
    assert!(!is_test_event(&delivery.event), "the body carries no flag");

    // The body alone — the signed source of truth.
    let raw = String::from_utf8(body())
        .unwrap()
        .replace("\"sequence\":7", "\"sequence\":7,\"test\":true")
        .into_bytes();
    let mut headers = Headers::new();
    headers.insert("x-webhook-timestamp", TS.to_string());
    headers.insert("x-webhook-signature", sign_webhook("whsec", TS, &raw));
    let delivery =
        verify_webhook_delivery(&raw, &headers, &VerifyOptions::new("whsec").now(TS)).unwrap();
    assert!(delivery.is_test);
    assert!(is_test_event(&delivery.event));
    assert!(delivery.event.is_test());
}

#[test]
fn parses_the_discriminated_union_and_detects_stale_sequences() {
    let event = parse_webhook(&body()).unwrap();
    assert!(matches!(event, WebhookEvent::Payment(_)));
    assert_eq!(event.sequence(), 7);
    assert!(
        is_stale_event(&event, Some(7)),
        "the same sequence is not newer"
    );
    assert!(is_stale_event(&event, Some(8)));
    assert!(!is_stale_event(&event, Some(6)));
    assert!(!is_stale_event(&event, None), "nothing processed yet");
}

#[test]
fn refuses_a_body_that_is_not_an_event() {
    assert_eq!(
        parse_webhook(br#"{"type":"alien","uuid":"x"}"#)
            .unwrap_err()
            .code(),
        "webhook.bad_signature"
    );
    assert!(parse_webhook(b"not json").is_err());
    assert!(parse_webhook(br#"{"uuid":"x"}"#).is_err());
}

#[test]
fn a_non_integer_timestamp_header_is_refused() {
    let mut headers = Headers::new();
    headers.insert("x-webhook-timestamp", "later");
    headers.insert("x-webhook-signature", "aa");
    let err = verify_webhook(&body(), &headers, &VerifyOptions::new("whsec")).unwrap_err();
    assert_eq!(err.code(), "webhook.bad_signature");
}
