//! Spec §3 items 3–4: amounts are decimal strings (a float is refused before the network), and
//! the generated models read what this SDK version does not know yet.

use oblodai::enums::PaymentStatus;
use oblodai::models::{PaymentRequest, PaymentView};
use oblodai::{from_json, ErrorKind, Money};
use serde_json::json;

mod support;

#[test]
fn a_float_amount_is_refused_with_its_own_code() {
    let err = from_json::<PaymentRequest>(json!({"amount": 25.5, "currency": "USDT"})).unwrap_err();
    assert_eq!(err.code(), "sdk.float_amount");
    assert_eq!(err.kind(), ErrorKind::Config);
    assert_eq!(err.field(), Some("amount"));
    // An exponent is a float too.
    let err = from_json::<PaymentRequest>(json!({"amount": 1e3, "currency": "USDT"})).unwrap_err();
    assert_eq!(err.code(), "sdk.float_amount");
}

#[test]
fn a_decimal_string_goes_to_the_wire_verbatim() {
    let req: PaymentRequest =
        from_json(json!({"amount": "25.10", "currency": "USDT", "order_id": "o1"})).unwrap();
    assert_eq!(req.amount, Money::from("25.10"));
    let wire = serde_json::to_value(&req).unwrap();
    assert_eq!(
        wire["amount"], "25.10",
        "trailing zero kept, still a string"
    );
    // An integer is exact: accepted, and sent as the same digits in a string.
    let req: PaymentRequest = from_json(json!({"amount": 25, "currency": "USDT"})).unwrap();
    assert_eq!(req.amount.as_str(), "25");
}

#[test]
fn other_mismatches_are_bad_params() {
    let err = from_json::<PaymentRequest>(json!({"currency": "USDT"})).unwrap_err();
    assert_eq!(err.code(), "sdk.bad_params");
    assert!(err.message().contains("amount"), "{err}");
}

#[test]
fn unknown_fields_and_enum_values_parse() {
    let mut body = support::sample("payment");
    body["status"] = json!("teleported");
    body["brand_new_field"] = json!({"nested": true});
    let v: PaymentView = serde_json::from_value(body).unwrap();
    assert_eq!(v.status, PaymentStatus::Other("teleported".into()));
    assert_eq!(v.status.as_str(), "teleported");
    assert_eq!(v.extra["brand_new_field"], json!({"nested": true}));
    // And back: the unknown field travels on.
    let again = serde_json::to_value(&v).unwrap();
    assert_eq!(again["brand_new_field"], json!({"nested": true}));
    assert_eq!(again["status"], "teleported");
}

#[test]
fn debug_prints_the_model_without_secret_values() {
    let mut v: PaymentView = serde_json::from_value(support::sample("payment")).unwrap();
    v.extra.insert("api_token".into(), json!("tok-123"));
    let shown = format!("{v:?}");
    assert!(shown.starts_with("PaymentView {"), "{shown}");
    assert!(shown.contains("uuid: \"x\""), "{shown}");
    assert!(!shown.contains("tok-123"), "{shown}");
}
