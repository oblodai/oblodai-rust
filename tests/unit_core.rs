//! The pure core: envelope decoding, retry decisions, URL building, key validation. No I/O at all.

mod support;

use oblodai::core::envelope::{decode_envelope, parse_retry_after};
use oblodai::core::idempotency::{assert_idempotency_key, new_idempotency_key};
use oblodai::core::request::{fill_path, join_url, serialize_body};
use oblodai::core::retry::{retry_delay_ms, should_retry, RetryContext, RetryOptions};
use oblodai::core::util::{constant_time_eq, header_value, parse_http_date};
use oblodai::{Error, ErrorDetail, ErrorKind, Method};
use serde_json::json;

// --- envelopes ---------------------------------------------------------------------------------

#[test]
fn decodes_a_success_envelope() {
    let body = br#"{"state":0,"result":{"uuid":"u"}}"#;
    let result = decode_envelope(200, body, None, None).unwrap();
    assert_eq!(result["uuid"], "u");
}

#[test]
fn a_result_of_null_is_still_a_result() {
    let result = decode_envelope(200, br#"{"state":0,"result":null}"#, None, None).unwrap();
    assert!(result.is_null());
}

#[test]
fn decodes_an_error_envelope_with_the_cores_own_retryable_flag() {
    let body = br#"{"error":{"code":"payout.insufficient_funds","message":"no funds","retryable":true,"request_id":"rq-9"}}"#;
    let err = decode_envelope(409, body, None, None).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Conflict);
    assert_eq!(err.code(), "payout.insufficient_funds");
    assert!(err.retryable(), "the core said so");
    assert!(!err.synthetic());
    assert_eq!(err.request_id(), Some("rq-9"));
}

#[test]
fn an_answer_without_an_envelope_is_synthetic_and_judged_by_its_status() {
    let err = decode_envelope(502, b"<html>bad gateway</html>", None, None).unwrap_err();
    assert!(err.synthetic());
    assert!(err.retryable(), "502 is transient");
    assert_eq!(err.code(), "internal");
    assert!(err.message().contains("proxy or load balancer"));

    let err = decode_envelope(418, b"nope", None, None).unwrap_err();
    assert!(!err.retryable(), "418 is not transient");
}

#[test]
fn a_400_without_an_envelope_is_not_retryable() {
    let err = decode_envelope(400, b"", None, None).unwrap_err();
    assert!(err.synthetic());
    assert!(!err.retryable());
    assert!(err.message().contains("<empty body>"));
}

#[test]
fn a_redirect_names_where_it_points() {
    let err = decode_envelope(302, b"", None, Some("https://elsewhere/v1/x")).unwrap_err();
    assert_eq!(err.http_status(), 302);
    assert!(err.message().contains("https://elsewhere/v1/x"));
}

#[test]
fn a_non_json_success_body_is_a_contract_error() {
    let err = decode_envelope(200, b"not json at all", None, None).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Contract);
    assert_eq!(err.code(), "sdk.bad_envelope");
}

#[test]
fn parses_retry_after_as_seconds_or_a_date() {
    assert_eq!(parse_retry_after(Some("30"), 0), Some(30));
    assert_eq!(parse_retry_after(Some(" 7 "), 0), Some(7));
    assert_eq!(parse_retry_after(Some("garbage"), 0), None);
    assert_eq!(parse_retry_after(None, 0), None);
    let at = parse_http_date("Sun, 06 Nov 1994 08:49:37 GMT").unwrap();
    assert_eq!(
        parse_retry_after(Some("Sun, 06 Nov 1994 08:49:37 GMT"), at - 60),
        Some(60)
    );
    assert_eq!(
        parse_retry_after(Some("Sun, 06 Nov 1994 08:49:37 GMT"), at + 60),
        Some(0)
    );
}

#[test]
fn parses_the_three_http_date_forms() {
    // All three name the same instant (RFC 7231 §7.1.1.1).
    assert_eq!(
        parse_http_date("Sun, 06 Nov 1994 08:49:37 GMT"),
        Some(784_111_777)
    );
    assert_eq!(
        parse_http_date("Sunday, 06-Nov-94 08:49:37 GMT"),
        Some(784_111_777)
    );
    assert_eq!(
        parse_http_date("Sun Nov  6 08:49:37 1994"),
        Some(784_111_777)
    );
    assert_eq!(parse_http_date("not a date"), None);
    assert_eq!(parse_http_date("Sun, 06 Foo 1994 08:49:37 GMT"), None);
}

// --- retry policy ------------------------------------------------------------------------------

fn envelope_error(status: u16, retryable: bool) -> Error {
    Error::from_envelope(
        status,
        ErrorDetail {
            code: "db.unavailable".into(),
            retryable: Some(retryable),
            ..Default::default()
        },
        None,
        false,
        None,
    )
}

fn synthetic_error(status: u16) -> Error {
    Error::from_envelope(status, ErrorDetail::default(), None, true, None)
}

#[test]
fn retries_stop_at_the_configured_ceiling() {
    let opts = RetryOptions::default();
    let err = envelope_error(503, true);
    assert!(should_retry(
        &err,
        RetryContext {
            attempt: 0,
            safe_to_repeat: false
        },
        &opts
    ));
    assert!(should_retry(
        &err,
        RetryContext {
            attempt: 1,
            safe_to_repeat: false
        },
        &opts
    ));
    assert!(!should_retry(
        &err,
        RetryContext {
            attempt: 2,
            safe_to_repeat: false
        },
        &opts
    ));
}

#[test]
fn a_non_retryable_error_is_never_retried_however_safe_the_route() {
    let opts = RetryOptions::default();
    let err = envelope_error(500, false);
    assert!(!should_retry(
        &err,
        RetryContext {
            attempt: 0,
            safe_to_repeat: true
        },
        &opts
    ));
}

#[test]
fn transport_and_synthetic_failures_need_the_route_to_be_safe_to_repeat() {
    let opts = RetryOptions::default();
    for err in [
        Error::transport("transport.network", "reset"),
        synthetic_error(503),
    ] {
        assert!(err.retryable());
        assert!(should_retry(
            &err,
            RetryContext {
                attempt: 0,
                safe_to_repeat: true
            },
            &opts
        ));
        assert!(
            !should_retry(
                &err,
                RetryContext {
                    attempt: 0,
                    safe_to_repeat: false
                },
                &opts
            ),
            "the core may already have done the work"
        );
    }
}

#[test]
fn an_enveloped_retryable_error_is_retried_even_on_an_unsafe_write() {
    let opts = RetryOptions::default();
    let err = envelope_error(409, true);
    assert!(should_retry(
        &err,
        RetryContext {
            attempt: 0,
            safe_to_repeat: false
        },
        &opts
    ));
}

#[test]
fn an_abort_or_deadline_is_never_retried() {
    let opts = RetryOptions::default();
    for code in ["transport.aborted", "transport.deadline"] {
        let err = Error::transport(code, "stopped");
        assert!(!err.retryable());
        assert!(!should_retry(
            &err,
            RetryContext {
                attempt: 0,
                safe_to_repeat: true
            },
            &opts
        ));
    }
}

#[test]
fn retry_after_wins_over_the_computed_backoff_and_is_capped() {
    let opts = RetryOptions::default();
    let ctx = RetryContext {
        attempt: 0,
        safe_to_repeat: true,
    };
    let with_hint = Error::from_envelope(
        429,
        ErrorDetail {
            code: "request.rate_limited".into(),
            retry_after: Some(5),
            ..Default::default()
        },
        None,
        false,
        None,
    );
    assert_eq!(retry_delay_ms(&with_hint, ctx, &opts, 0.0), 5_000);

    let huge = Error::from_envelope(
        429,
        ErrorDetail {
            code: "x".into(),
            retry_after: Some(9_999),
            ..Default::default()
        },
        None,
        false,
        None,
    );
    assert_eq!(
        retry_delay_ms(&huge, ctx, &opts, 0.0),
        opts.max_retry_after_ms
    );
}

#[test]
fn backoff_grows_exponentially_with_a_jitter_floor() {
    let opts = RetryOptions::default();
    let err = envelope_error(503, true);
    let full = |attempt| {
        retry_delay_ms(
            &err,
            RetryContext {
                attempt,
                safe_to_repeat: true,
            },
            &opts,
            1.0,
        )
    };
    assert_eq!(full(0), 250);
    assert_eq!(full(1), 500);
    assert_eq!(full(2), 1_000);
    assert_eq!(full(9), opts.max_delay_ms, "capped");
    // A zero random still waits a quarter of the window, so a burst never lands in one instant.
    let floor = retry_delay_ms(
        &err,
        RetryContext {
            attempt: 0,
            safe_to_repeat: true,
        },
        &opts,
        0.0,
    );
    assert_eq!(floor, 250 / 4);
}

// --- request building --------------------------------------------------------------------------

#[test]
fn keeps_a_path_prefix_and_drops_any_query_of_the_base_url() {
    assert_eq!(
        join_url("https://gw.corp/oblodai", "/v1/payment")
            .unwrap()
            .as_str(),
        "https://gw.corp/oblodai/v1/payment"
    );
    assert_eq!(
        join_url("https://api.test/?a=1", "/v1/payment")
            .unwrap()
            .as_str(),
        "https://api.test/v1/payment"
    );
}

#[test]
fn fills_and_percent_encodes_path_parameters() {
    assert_eq!(
        fill_path("/v1/pay/{id}", &[("id", "abc".into())]).unwrap(),
        "/v1/pay/abc"
    );
    assert_eq!(
        fill_path("/v1/pay/{id}", &[("id", "a b".into())]).unwrap(),
        "/v1/pay/a%20b"
    );
    assert_eq!(
        fill_path(
            "/v1/documents/{kind}/{id}",
            &[("kind", "invoice".into()), ("id", "i1".into())]
        )
        .unwrap(),
        "/v1/documents/invoice/i1"
    );
}

#[test]
fn refuses_a_path_parameter_that_would_rewrite_the_url() {
    for bad in ["", ".", "..", "a/b"] {
        let err = fill_path("/v1/pay/{id}", &[("id", bad.into())]).unwrap_err();
        assert_eq!(err.code(), "sdk.bad_path_param", "for {bad:?}");
    }
    // A missing parameter is the empty case, and is refused too.
    assert!(fill_path("/v1/pay/{id}", &[]).is_err());
}

#[test]
fn serializes_a_missing_post_body_as_an_empty_object_and_a_get_body_as_nothing() {
    assert_eq!(serialize_body(None, Method::Post), "{}");
    assert_eq!(serialize_body(Some(&json!(null)), Method::Post), "{}");
    assert_eq!(
        serialize_body(Some(&json!({"a": 1})), Method::Post),
        r#"{"a":1}"#
    );
    assert_eq!(serialize_body(Some(&json!({"a": 1})), Method::Get), "");
}

// --- idempotency keys ---------------------------------------------------------------------------

#[test]
fn generated_keys_are_unique_v4_uuids() {
    let a = new_idempotency_key();
    let b = new_idempotency_key();
    assert_ne!(a, b);
    assert_eq!(a.len(), 36);
    assert_idempotency_key(&a).unwrap();
}

#[test]
fn refuses_a_key_that_would_not_survive_a_header() {
    assert!(assert_idempotency_key("").is_err());
    assert!(assert_idempotency_key("has space").is_err());
    assert!(assert_idempotency_key("tab\there").is_err());
    assert!(assert_idempotency_key(&"x".repeat(256)).is_err());
    assert!(assert_idempotency_key(&"x".repeat(255)).is_ok());
    assert!(assert_idempotency_key("order-1001:retry#2").is_ok());
}

// --- small helpers -------------------------------------------------------------------------------

#[test]
fn constant_time_equality_still_compares_correctly() {
    assert!(constant_time_eq("abcd", "abcd"));
    assert!(!constant_time_eq("abcd", "abce"));
    assert!(!constant_time_eq("abcd", "abc"));
}

#[test]
fn header_lookup_ignores_case() {
    let headers = vec![("Content-Type".to_string(), "application/json".to_string())];
    assert_eq!(
        header_value(&headers, "content-type"),
        Some("application/json")
    );
    assert_eq!(
        header_value(&headers, "CONTENT-TYPE"),
        Some("application/json")
    );
    assert_eq!(header_value(&headers, "other"), None);
}

#[test]
fn a_logger_redacts_what_looks_like_a_secret() {
    use oblodai::core::logger::redact;
    assert_eq!(redact("secret", "abc"), "[redacted]");
    assert_eq!(redact("X-Signature", "abc"), "[redacted]");
    assert_eq!(redact("claim_passcode", "1234"), "[redacted]");
    assert_eq!(redact("order_id", "o-1"), "o-1");
}
