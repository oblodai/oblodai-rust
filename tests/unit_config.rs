//! Configuration resolution, and the money/status helpers every integration touches.

mod support;

use std::cmp::Ordering;
use std::sync::Arc;

use oblodai::helpers::{
    add_amounts, amounts_equal, compare_amounts, is_payment_final, is_payment_paid,
    is_payment_underpaid, is_payout_final, is_payout_succeeded, is_zero_amount, subtract_amounts,
};
use oblodai::{Client, ClientBuilder, HttpBackend, PaymentStatus, PayoutStatus};
use support::MockBackend;

fn empty_env() -> Vec<(String, String)> {
    Vec::new()
}

fn builder() -> ClientBuilder {
    let mock = MockBackend::new(vec![]);
    Client::builder().http_backend(mock as Arc<dyn HttpBackend>)
}

#[test]
fn reads_credentials_and_base_url_from_the_environment() {
    let client = builder()
        .env([
            ("OBLODAI_PUBLIC_ID", "pk"),
            ("OBLODAI_SECRET", "s"),
            ("OBLODAI_BASE_URL", "https://x.test/"),
        ])
        .build()
        .unwrap();
    let core = client.transport().core();
    assert_eq!(
        core.base_url, "https://x.test",
        "the trailing slash is trimmed"
    );
    let creds = core.credentials.as_ref().unwrap();
    assert_eq!(creds.public_id, "pk");
    assert_eq!(creds.secret, "s");
}

#[test]
fn explicit_options_win_over_the_environment() {
    let client = builder()
        .public_id("explicit")
        .secret("s")
        .env([("OBLODAI_PUBLIC_ID", "from-env"), ("OBLODAI_SECRET", "s")])
        .build()
        .unwrap();
    assert_eq!(
        client
            .transport()
            .core()
            .credentials
            .as_ref()
            .unwrap()
            .public_id,
        "explicit"
    );
}

#[test]
fn refuses_plain_http_except_for_loopback_or_when_allowed() {
    let err = builder()
        .base_url("http://api.oblodai.com")
        .env(empty_env())
        .build()
        .unwrap_err();
    assert_eq!(err.code(), "sdk.bad_config");
    assert!(err.message().contains("https"));

    for local in [
        "http://localhost:8095",
        "http://127.0.0.1:8095",
        "http://[::1]:8095",
    ] {
        builder()
            .base_url(local)
            .env(empty_env())
            .build()
            .unwrap_or_else(|e| {
                panic!("{local} should be allowed: {e}");
            });
    }
    builder()
        .base_url("http://10.0.0.1")
        .allow_insecure_base_url(true)
        .env(empty_env())
        .build()
        .unwrap();
    builder()
        .base_url("http://10.0.0.1")
        .env([("OBLODAI_ALLOW_INSECURE", "1")])
        .build()
        .unwrap();
}

#[test]
fn refuses_a_base_url_that_is_not_a_url() {
    let err = builder()
        .base_url("not a url")
        .env(empty_env())
        .build()
        .unwrap_err();
    assert_eq!(err.code(), "sdk.bad_config");
    assert_eq!(err.field(), Some("base_url"));
}

#[test]
fn refuses_half_a_key_pair() {
    let err = builder()
        .public_id("pk")
        .env(empty_env())
        .build()
        .unwrap_err();
    assert!(err.message().contains("together"));
    let err = builder()
        .payout_secret("s")
        .env(empty_env())
        .build()
        .unwrap_err();
    assert!(err.message().contains("together"));
}

#[test]
fn defaults_to_the_production_api() {
    let client = builder().env(empty_env()).build().unwrap();
    assert_eq!(
        client.transport().core().base_url,
        oblodai::DEFAULT_BASE_URL
    );
    assert!(client.transport().core().credentials.is_none());
}

#[test]
fn a_debug_dump_of_the_config_never_shows_a_secret() {
    let client = builder()
        .public_id("pk")
        .secret("super-secret")
        .env(empty_env())
        .build()
        .unwrap();
    let dump = format!("{:?}", client.transport().core());
    assert!(dump.contains("pk"));
    assert!(!dump.contains("super-secret"), "{dump}");
}

// --- money ---------------------------------------------------------------------------------------

#[test]
fn money_helpers_work_at_arbitrary_precision() {
    assert_eq!(add_amounts("0.1", "0.2").unwrap().as_str(), "0.3");
    assert_eq!(
        add_amounts("10.000000", "0.5").unwrap().as_str(),
        "10.500000"
    );
    assert_eq!(
        subtract_amounts("1", "1.000001").unwrap().as_str(),
        "-0.000001"
    );
    assert_eq!(compare_amounts("25", "25.000000").unwrap(), Ordering::Equal);
    assert_eq!(
        compare_amounts("0.000000000000000001", "0").unwrap(),
        Ordering::Greater
    );
    assert_eq!(compare_amounts("1", "2").unwrap(), Ordering::Less);
    assert!(amounts_equal("25", "25.00").unwrap());
    assert!(is_zero_amount("0.000000").unwrap());
    assert!(!is_zero_amount("0.000001").unwrap());
    // 18 decimals: the case an f64 would silently ruin.
    assert_eq!(
        add_amounts("0.000000000000000001", "0.000000000000000002")
            .unwrap()
            .as_str(),
        "0.000000000000000003"
    );
}

#[test]
fn money_helpers_refuse_what_is_not_a_decimal() {
    for bad in ["", "abc", "1.2.3", "1,2", "1.", "--1", " 1"] {
        assert!(add_amounts(bad, "1").is_err(), "{bad:?} should not parse");
    }
}

// --- statuses ------------------------------------------------------------------------------------

#[test]
fn status_helpers_follow_the_core_vocabulary() {
    assert!(is_payment_paid(&PaymentStatus::PaidOver));
    assert!(!is_payment_paid(&PaymentStatus::WrongAmount));
    assert!(is_payment_underpaid(&PaymentStatus::WrongAmount));
    assert!(!is_payment_final(&PaymentStatus::ConfirmCheck));
    assert!(is_payment_final(&PaymentStatus::Cancelled));
    assert!(!is_payout_final(&PayoutStatus::Sent));
    assert!(is_payout_final(&PayoutStatus::Confirmed));
    assert!(is_payout_succeeded(&PayoutStatus::Confirmed));
    assert!(!is_payout_succeeded(&PayoutStatus::Failed));
}

#[test]
fn an_unknown_status_from_a_newer_core_still_decodes() {
    let status: PaymentStatus = serde_json::from_str("\"time_travelling\"").unwrap();
    assert_eq!(status, PaymentStatus::Other("time_travelling".into()));
    assert_eq!(status.as_str(), "time_travelling");
    assert_eq!(
        serde_json::to_string(&status).unwrap(),
        "\"time_travelling\""
    );
    assert!(!is_payment_final(&status));
}
