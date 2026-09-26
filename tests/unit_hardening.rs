//! Regressions for the defects the 1.3 review found. Every test here failed before the fix.
//!
//! No I/O: envelope decoding, amount helpers, header building and the `Money` ordering trap.

// The SDK's error carries the gateway's whole envelope by value, exactly as callers match on it;
// boxing it here would only make the test disagree with the crate it exercises.
#![allow(clippy::result_large_err)]

mod support;

use std::marker::PhantomData;

use oblodai::core::envelope::{decode_envelope, parse_retry_after};
use oblodai::helpers::{add_amounts, compare_amounts, is_zero_amount, subtract_amounts};
use oblodai::{ErrorKind, Money};

// --- the error envelope, field by field -------------------------------------------------------

/// A malformed `retry_after` used to wipe the whole envelope: the code became `internal`, the
/// request id was lost, and the core's authoritative `retryable: false` flipped to `true` because
/// the status was 503 — so the SDK retried what the core said not to retry.
#[test]
fn a_float_retry_after_does_not_destroy_the_rest_of_the_envelope() {
    let body = br#"{"error":{"code":"payout.funds_maturing","message":"not mature",
                   "retryable":false,"retry_after":30.0,"request_id":"rq-1","field":"amount"}}"#;
    let err = decode_envelope(503, body, None, None).unwrap_err();
    assert_eq!(err.code(), "payout.funds_maturing");
    assert!(!err.retryable(), "the core's own flag survives");
    assert!(!err.synthetic(), "there WAS an envelope");
    assert_eq!(err.retry_after(), Some(30));
    assert_eq!(err.request_id(), Some("rq-1"));
    assert_eq!(err.field(), Some("amount"));
    assert_eq!(err.message(), "not mature");
}

#[test]
fn details_keep_only_string_values() {
    let body = br#"{"error":{"code":"cli.permission_denied","retryable":false,
                   "details":{"required_role":"finance","role":"viewer","n":3,"x":null}}}"#;
    let err = decode_envelope(403, body, None, None).unwrap_err();
    let details = err.details().expect("details");
    assert_eq!(details.len(), 2);
    assert_eq!(details["required_role"], "finance");
    assert_eq!(details["role"], "viewer");
    let json = serde_json::to_value(&err).unwrap();
    assert_eq!(json["details"]["role"], "viewer");

    let list = br#"{"error":{"code":"cli.permission_denied","details":["finance"]}}"#;
    assert!(decode_envelope(403, list, None, None)
        .unwrap_err()
        .details()
        .is_none());
    let none = br#"{"error":{"code":"cli.permission_denied"}}"#;
    assert!(decode_envelope(403, none, None, None)
        .unwrap_err()
        .details()
        .is_none());
}

#[test]
fn a_numeric_string_retry_after_is_read_and_other_shapes_are_dropped() {
    let with = |slot: &str| {
        let body = format!(
            r#"{{"error":{{"code":"request.rate_limited","message":"slow down","retry_after":{slot}}}}}"#
        );
        decode_envelope(429, body.as_bytes(), None, None).unwrap_err()
    };
    assert_eq!(with("\"12\"").retry_after(), Some(12));
    assert_eq!(with("12.7").retry_after(), Some(12));
    assert_eq!(with("true").retry_after(), None);
    assert_eq!(with("{}").retry_after(), None);
    assert_eq!(with("\"soon\"").retry_after(), None);
    // Whatever the slot held, the code is intact.
    for slot in ["\"12\"", "12.7", "true", "{}", "\"soon\""] {
        assert_eq!(with(slot).code(), "request.rate_limited", "{slot}");
    }
}

#[test]
fn only_a_literal_boolean_overrides_the_status_derived_retryable() {
    let with = |slot: &str| {
        let body = format!(r#"{{"error":{{"code":"x.y","retryable":{slot}}}}}"#);
        decode_envelope(503, body.as_bytes(), None, None).unwrap_err()
    };
    assert!(!with("false").retryable(), "the core said no");
    assert!(with("true").retryable());
    // "false" as a string, or 0, is not the core speaking: fall back to the status (503 → true).
    assert!(with("\"false\"").retryable());
    assert!(with("0").retryable());
    assert!(with("null").retryable());
}

#[test]
fn a_field_of_the_wrong_type_is_absent_not_fatal() {
    let body = br#"{"error":{"code":"payment.not_found","request_id":42,"field":["a"],
                   "message":{"nested":true}}}"#;
    let err = decode_envelope(404, body, None, None).unwrap_err();
    assert_eq!(err.code(), "payment.not_found");
    assert_eq!(err.kind(), ErrorKind::NotFound);
    assert_eq!(err.request_id(), None);
    assert_eq!(err.field(), None);
    assert!(err.message().contains("404"), "a usable fallback message");
}

#[test]
fn an_envelope_without_a_usable_code_is_synthetic_but_keeps_the_request_id() {
    let body = br#"{"error":{"code":"","request_id":"rq-7"}}"#;
    let err = decode_envelope(502, body, None, None).unwrap_err();
    assert!(err.synthetic(), "nothing here is the core's classification");
    assert_eq!(err.http_status(), 502, "the real status is kept");
    assert_eq!(err.request_id(), Some("rq-7"), "still worth quoting");
}

#[test]
fn retry_after_is_clamped_never_negative_and_never_overflowing() {
    let now = 1_700_000_000;
    // A date in the past is 0, not a negative delay.
    assert_eq!(
        parse_retry_after(Some("Thu, 01 Jan 1970 00:00:00 GMT"), now),
        Some(0)
    );
    // A date far in the future is capped, not an overflow.
    let far = parse_retry_after(Some("Fri, 31 Dec 9999 23:59:59 GMT"), now).unwrap();
    assert_eq!(far, oblodai::error::MAX_RETRY_AFTER_SECONDS);
    // A 30-digit delta-seconds header is capped rather than silently dropped.
    assert_eq!(
        parse_retry_after(Some("999999999999999999999999999999"), now),
        Some(oblodai::error::MAX_RETRY_AFTER_SECONDS)
    );
    assert_eq!(parse_retry_after(Some("  "), now), None);
    assert_eq!(parse_retry_after(Some("whenever"), now), None);
    assert_eq!(parse_retry_after(Some("7"), now), Some(7));
}

#[test]
fn an_absurd_envelope_retry_after_is_clamped_too() {
    let body = br#"{"error":{"code":"x.y","retry_after":1e30}}"#;
    let err = decode_envelope(429, body, None, None).unwrap_err();
    assert_eq!(
        err.retry_after(),
        Some(oblodai::error::MAX_RETRY_AFTER_SECONDS)
    );
    let body = br#"{"error":{"code":"x.y","retry_after":-5}}"#;
    let err = decode_envelope(429, body, None, None).unwrap_err();
    assert_eq!(err.retry_after(), Some(0), "never a negative wait");
}

// --- amount helpers ---------------------------------------------------------------------------

/// The scale was fed to a format width, which is a `u16`: a fraction of 65 536 digits panicked
/// inside a crate that forbids unsafe and returns `Result` everywhere.
#[test]
fn a_pathologically_long_decimal_is_an_error_not_a_panic() {
    for len in [64, 100, 65_535, 65_536, 200_000] {
        let hostile = format!("0.{}", "1".repeat(len));
        assert!(add_amounts(&hostile, "1").is_err(), "{len} digits");
        assert!(subtract_amounts("1", &hostile).is_err(), "{len} digits");
        assert!(compare_amounts(&hostile, "1").is_err(), "{len} digits");
        assert!(is_zero_amount(&hostile).is_err(), "{len} digits");
    }
}

#[test]
fn the_length_bound_is_the_documented_one_and_real_amounts_fit_inside_it() {
    // 18 decimals (the widest asset on the gateway) plus a big integer part is far under the bound.
    let wide = format!("123456789.{}", "9".repeat(18));
    assert!(add_amounts(&wide, "0").is_ok());
    let at_bound = "1".repeat(oblodai::helpers::MAX_AMOUNT_LEN);
    assert!(
        compare_amounts(&at_bound, &at_bound).is_err(),
        "i128 overflow, still an error"
    );
    let over_bound = "1".repeat(oblodai::helpers::MAX_AMOUNT_LEN + 1);
    assert!(compare_amounts("1", &over_bound).is_err());
}

#[test]
fn padding_to_a_common_scale_is_exact() {
    assert_eq!(add_amounts("0.1", "0.2").unwrap().as_str(), "0.3");
    assert_eq!(add_amounts("25", "0.000001").unwrap().as_str(), "25.000001");
    assert_eq!(
        subtract_amounts("1", "0.999999999999999999")
            .unwrap()
            .as_str(),
        "0.000000000000000001"
    );
    assert_eq!(
        compare_amounts("25", "25.000000").unwrap(),
        std::cmp::Ordering::Equal
    );
    assert_eq!(
        compare_amounts("9.00", "10.00").unwrap(),
        std::cmp::Ordering::Less
    );
}

#[test]
fn garbage_is_refused_without_a_panic() {
    for bad in [
        "", "-", ".", "1.", ".5", "1.2.3", "1e9", "٣", "1 2", "--1", "0x10",
    ] {
        assert!(compare_amounts(bad, "1").is_err(), "{bad:?}");
    }
}

// --- `Money` must not be ordered lexicographically --------------------------------------------

struct Probe<T>(PhantomData<T>);

trait NoOrd {
    fn is_ord(&self) -> bool {
        false
    }
}

impl<T> NoOrd for Probe<T> {}

impl<T: Ord> Probe<T> {
    fn is_ord(&self) -> bool {
        true
    }
}

/// `Money` derived `Ord`, so `Money("9.00") > Money("10.00")` compiled and was `true`, and sorting
/// a list of amounts produced `["10", "2", "9"]`. Dropping the derives turns that into a compile
/// error; this probe fails if anyone puts them back.
#[test]
fn money_does_not_implement_ord() {
    assert!(
        Probe::<String>(PhantomData).is_ord(),
        "control: the probe does detect Ord"
    );
    assert!(
        !Probe::<Money>(PhantomData).is_ord(),
        "Money must not be orderable as a string — use helpers::compare_amounts"
    );
    // The trap the derives created, spelled out:
    assert!(
        Money("9.00".into()).0 > Money("10.00".into()).0,
        "strings order this way"
    );
    assert_eq!(
        compare_amounts("9.00", "10.00").unwrap(),
        std::cmp::Ordering::Less
    );
}

#[test]
fn money_still_supports_equality_and_hashing() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(Money("1.5".into()));
    assert!(set.contains(&Money("1.5".into())));
    assert_ne!(Money("25".into()), Money("25.000000".into()));
}

// --- the shared SDK error vocabulary ----------------------------------------------------------

#[test]
fn a_bad_amount_converts_to_the_shared_sdk_error_code() {
    let err: oblodai::Error = compare_amounts("nope", "1").unwrap_err().into();
    assert_eq!(err.code(), "sdk.bad_amount");
    assert_eq!(err.kind(), ErrorKind::Config);
    assert_eq!(err.field(), Some("amount"));
    assert_eq!(oblodai::helpers::AmountError::CODE, "sdk.bad_amount");
}

#[test]
fn a_client_side_idempotency_key_failure_is_a_config_error() {
    use oblodai::core::idempotency::assert_idempotency_key;
    for bad in ["", " key", &"k".repeat(256)] {
        let err = assert_idempotency_key(bad).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Config, "{bad:?}");
        assert_eq!(err.code(), "sdk.bad_idempotency_key", "{bad:?}");
    }
    assert!(assert_idempotency_key("order-1001").is_ok());
}
