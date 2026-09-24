//! Webhook verification, against real signed deliveries and against the rules.

// The SDK's error carries the gateway's whole envelope by value, exactly as callers match on it;
// boxing it here would only make the test disagree with the crate it exercises.
#![allow(clippy::result_large_err)]

mod support;

use oblodai::webhooks::{
    is_stale_event, is_test_event, parse_webhook, verify_webhook, verify_webhook_delivery, Headers,
    VerifyOptions,
};
use oblodai::{sign_webhook, ErrorKind, WebhookEvent};
use support::load_webhook_samples;

/// The samples were delivered by the core's real dispatcher, signed with the endpoint secret in
/// force at that moment — the one the recorded rotate-secret call returned.
fn endpoint_secret() -> String {
    "70200ecc6784c713e4fcda1c7b4d3e520713bb109edeacc541c3a90fa8cfd91f".to_string()
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
        assert_eq!(
            delivery.event_id.as_deref(),
            sample.header("X-Webhook-Event-Id")
        );
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
            assert_eq!(delivery.event.sequence(), Some(0), "{event_name}");
        } else {
            assert!(delivery.event.sequence().unwrap_or(0) > 0, "{event_name}");
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
        // An optional field that arrived as `null` is absent after the round trip: the models
        // keep "not there" and "null" as one `None`.
        let mut body = sample.body.clone();
        if let Some(map) = body.as_object_mut() {
            map.retain(|_, v| !v.is_null());
        }
        assert_eq!(
            round_tripped,
            body,
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
fn reads_the_event_id_apart_from_the_delivery_id() {
    let mut headers = headers_for("whsec");
    headers.insert("x-webhook-id", "d-1");
    headers.insert("x-webhook-event-id", "e-1");
    let options = VerifyOptions::new("whsec").now(TS);
    let delivery = verify_webhook_delivery(&body(), &headers, &options).unwrap();
    assert_eq!(delivery.id.as_deref(), Some("d-1"));
    assert_eq!(delivery.event_id.as_deref(), Some("e-1"));
    let bare = verify_webhook_delivery(&body(), &headers_for("whsec"), &options).unwrap();
    assert_eq!(bare.event_id, None);
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
    assert_eq!(event.sequence(), Some(7));
    assert!(
        is_stale_event(&event, Some(7)),
        "the same sequence is not newer"
    );
    assert!(is_stale_event(&event, Some(8)));
    assert!(!is_stale_event(&event, Some(6)));
    assert!(!is_stale_event(&event, None), "nothing processed yet");
}

#[test]
fn an_unknown_event_type_is_kept_not_refused() {
    // A `type` this snapshot does not model must never make a receiver reject an authentic
    // delivery: it arrives as `Other` with the body intact, and the helpers keep working.
    let event = parse_webhook(br#"{"type":"alien","uuid":"x","sequence":9,"test":true}"#)
        .expect("an unknown event type is data, not an error");
    assert!(matches!(event, oblodai::WebhookEvent::Other(_)));
    assert_eq!(event.event_kind(), "alien");
    assert_eq!(event.uuid(), "x");
    assert_eq!(event.sequence(), Some(9));
    assert!(event.is_test());
    assert!(is_test_event(&event));
    assert!(is_stale_event(&event, Some(9)));
    assert!(!is_stale_event(&event, Some(8)));
    assert!(event.raw().is_some());
}

#[test]
fn an_unreadable_body_is_a_contract_error_not_a_signature_failure() {
    // A 401 answer to an authentic delivery makes the gateway retry it for ~26 h. Anything that
    // verified but cannot be read is `webhook.bad_payload` in the contract family instead.
    for body in [
        &b"not json"[..],
        br#"{"uuid":"x"}"#,                              // no `type`
        br#"{"type":42}"#,                               // `type` is not a string
        br#"{"type":"payment","uuid":{"nested":true}}"#, // known type, wrong field shape
    ] {
        let err = parse_webhook(body).unwrap_err();
        assert_eq!(err.code(), "webhook.bad_payload", "{body:?}");
        assert_eq!(err.kind(), oblodai::ErrorKind::Contract, "{body:?}");
    }
}

#[test]
fn an_event_without_a_sequence_is_never_stale() {
    let event = parse_webhook(br#"{"type":"alien","uuid":"x"}"#).unwrap();
    assert_eq!(event.sequence(), None);
    assert!(!is_stale_event(&event, Some(1_000_000)));
}

#[test]
fn a_non_integer_timestamp_header_is_refused() {
    let mut headers = Headers::new();
    headers.insert("x-webhook-timestamp", "later");
    headers.insert("x-webhook-signature", "aa");
    let err = verify_webhook(&body(), &headers, &VerifyOptions::new("whsec")).unwrap_err();
    assert_eq!(err.code(), "webhook.bad_signature");
}

// --- configuration and ordering ---------------------------------------------------------------

/// An empty secret would compute an HMAC with the empty key and happily "verify" a delivery an
/// attacker signed the same way. It has to be refused before any crypto runs.
#[test]
fn an_empty_secret_is_a_config_error_not_a_verification_attempt() {
    let err = verify_webhook(&body(), &headers_for("whsec"), &VerifyOptions::new("")).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Config);
    assert_eq!(err.field(), Some("secret"));

    let options = VerifyOptions::new("whsec").previous_secret("");
    let err = verify_webhook(&body(), &headers_for("whsec"), &options).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Config);
    assert_eq!(err.field(), Some("previous_secret"));
}

#[test]
fn a_negative_tolerance_is_a_config_error_not_a_disabled_check() {
    let options = VerifyOptions::new("whsec").tolerance_seconds(-1).now(TS);
    let err = verify_webhook(&body(), &headers_for("whsec"), &options).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Config);
    assert_eq!(err.field(), Some("tolerance_seconds"));
}

/// The freshness window used to be checked before the MAC, so an unauthenticated caller could ask
/// "is this timestamp inside your window?" and get an answer. The MAC comes first now.
#[test]
fn the_signature_is_checked_before_the_freshness_window() {
    let mut headers = Headers::new();
    headers.insert("x-webhook-timestamp", (TS - 100_000).to_string());
    headers.insert("x-webhook-signature", "00".repeat(32));
    let options = VerifyOptions::new("whsec").now(TS);
    let err = verify_webhook(&body(), &headers, &options).unwrap_err();
    assert_eq!(
        err.code(),
        "webhook.bad_signature",
        "a forged delivery must not learn anything about the freshness window"
    );

    // The same stale timestamp WITH a valid signature is the one that reports staleness.
    let mut headers = Headers::new();
    headers.insert("x-webhook-timestamp", (TS - 100_000).to_string());
    headers.insert(
        "x-webhook-signature",
        sign_webhook("whsec", TS - 100_000, &body()),
    );
    let err = verify_webhook(&body(), &headers, &options).unwrap_err();
    assert_eq!(err.code(), "webhook.stale_timestamp");
}

#[test]
fn a_signature_header_is_trimmed_and_case_insensitive_but_never_prefixed() {
    let signature = sign_webhook("whsec", TS, &body());
    let with = |value: String| {
        let mut headers = Headers::new();
        headers.insert("x-webhook-timestamp", TS.to_string());
        headers.insert("x-webhook-signature", value);
        verify_webhook(&body(), &headers, &VerifyOptions::new("whsec").now(TS))
    };
    assert!(
        with(format!("  {signature}  ")).is_ok(),
        "whitespace is tolerated"
    );
    assert!(
        with(signature.to_uppercase()).is_ok(),
        "upper-case hex is hex"
    );
    let err = with(format!("0x{signature}")).unwrap_err();
    assert_eq!(err.code(), "webhook.bad_signature");
    assert!(err.message().contains("0x"), "{}", err.message());
}

#[test]
fn inserting_a_header_twice_replaces_it() {
    let mut headers = Headers::new();
    headers.insert("X-Webhook-Id", "first");
    headers.insert("x-webhook-id", "second");
    assert_eq!(headers.get("X-WEBHOOK-ID"), Some("second"));
}

#[test]
fn is_known_event_tells_a_modelled_event_from_a_newer_one() {
    use oblodai::webhooks::is_known_event;
    let known = parse_webhook(&body()).unwrap();
    assert!(is_known_event(&known));
    assert!(known.is_known());
    assert!(known.raw().is_none());

    let newer = parse_webhook(br#"{"type":"settlement","uuid":"s1"}"#).unwrap();
    assert!(!is_known_event(&newer));
    assert!(!newer.is_known());
    assert_eq!(newer.event_kind(), "settlement");
}

/// The kinds and their models come from the contract: every delivered event name maps to a kind
/// this SDK models, and a conversion delivery parses to its own variant.
#[test]
fn every_event_of_the_contract_is_a_known_kind() {
    use oblodai::webhooks::{parse_webhook, KNOWN_EVENT_KINDS, WEBHOOK_EVENTS};
    for (event, kind) in WEBHOOK_EVENTS {
        assert!(KNOWN_EVENT_KINDS.contains(kind), "{event} → {kind}");
    }
    assert!(WEBHOOK_EVENTS.contains(&("conversion.completed", "conversion")));
    let body = serde_json::json!({
        "type": "conversion", "id": "c-1", "mode": "auto", "from": "USDT", "to": "BTC",
        "sent": "10", "fee_percent": "1", "status": "completed", "reason": "", "is_final": true,
        "document_url": "", "created_at": "t", "completed_at": "t", "sequence": 3, "event_at": "t"
    });
    let event = parse_webhook(body.to_string().as_bytes()).unwrap();
    assert!(matches!(event, oblodai::WebhookEvent::Conversion(_)));
    assert!(event.is_known());
    assert_eq!(
        (event.uuid(), event.sequence(), event.order_id()),
        ("c-1", Some(3), None)
    );
}
