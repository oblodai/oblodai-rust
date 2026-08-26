//! Request and webhook signing against the vectors the core's own test suite exports.

mod support;

use oblodai::{canonical_string, sign_request, sign_webhook, SignInput};
use support::load_contract;

#[test]
fn matches_every_request_signing_vector() {
    let contract = load_contract();
    let vectors = contract["signing_vectors"]
        .as_array()
        .expect("signing_vectors");
    assert!(!vectors.is_empty());
    for v in vectors {
        let name = v["name"].as_str().unwrap();
        let key = v["idempotency_key"].as_str().unwrap_or("");
        let body = v["body"].as_str().unwrap_or("");
        let input = SignInput {
            ts: v["ts"].as_i64().unwrap(),
            method: v["method"].as_str().unwrap(),
            request_uri: v["request_uri"].as_str().unwrap(),
            idempotency_key: if key.is_empty() { None } else { Some(key) },
            body: body.as_bytes(),
        };
        assert_eq!(
            canonical_string(&input),
            v["canonical"].as_str().unwrap(),
            "canonical: {name}"
        );
        assert_eq!(
            sign_request(v["secret"].as_str().unwrap(), &input),
            v["signature"].as_str().unwrap(),
            "signature: {name}"
        );
    }
}

#[test]
fn the_idempotency_slot_is_empty_not_absent() {
    let base = SignInput {
        ts: 1,
        method: "POST",
        request_uri: "/v1/x",
        idempotency_key: None,
        body: b"{}",
    };
    assert_eq!(canonical_string(&base), "1\nPOST\n/v1/x\n\n{}");
    // An explicit empty key must hash the same as no key at all.
    let empty = SignInput {
        idempotency_key: Some(""),
        ..base.clone()
    };
    assert_eq!(sign_request("s", &base), sign_request("s", &empty));
}

#[test]
fn signs_the_body_bytes_so_non_ascii_survives() {
    // Deliberately non-ASCII: the signature covers the body BYTES, so a multi-byte character must
    // hash identically whether it arrives as a `&str` or as a `Vec<u8>`.
    let body = r#"{"additional_data":"café 日本語 🚀"}"#;
    let input = SignInput {
        ts: 5,
        method: "POST",
        request_uri: "/v1/payment",
        idempotency_key: None,
        body: body.as_bytes(),
    };
    let from_str = sign_request("s", &input);
    let bytes = body.as_bytes().to_vec();
    let from_bytes = sign_request(
        "s",
        &SignInput {
            body: &bytes,
            ..input.clone()
        },
    );
    assert_eq!(from_str, from_bytes);
    assert_eq!(from_str.len(), 64);
}

#[test]
fn the_method_is_upper_cased_before_signing() {
    let upper = SignInput {
        ts: 7,
        method: "POST",
        request_uri: "/v1/x",
        idempotency_key: None,
        body: b"",
    };
    let lower = SignInput {
        method: "post",
        ..upper.clone()
    };
    assert_eq!(sign_request("s", &upper), sign_request("s", &lower));
}

#[test]
fn matches_every_webhook_signing_vector() {
    let contract = load_contract();
    let vectors = contract["webhook_vectors"]
        .as_array()
        .expect("webhook_vectors");
    assert!(!vectors.is_empty());
    for v in vectors {
        assert_eq!(
            sign_webhook(
                v["secret"].as_str().unwrap(),
                v["ts"].as_i64().unwrap(),
                v["payload"].as_str().unwrap().as_bytes(),
            ),
            v["signature"].as_str().unwrap()
        );
    }
}
