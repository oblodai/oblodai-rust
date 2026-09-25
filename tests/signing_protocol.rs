//! The signing protocol has one source: `x-oblodai-signing` of the contract, generated into
//! `src/generated/signing.rs`. The runtime reads the header names, the canonical strings and the
//! limits from there, so a header the core renames reaches this SDK by regeneration alone.

use std::path::{Path, PathBuf};

use oblodai::generated::signing as gen;
use serde_json::Value;

fn backend_root() -> PathBuf {
    match std::env::var_os("OBLODAI_BACKEND") {
        Some(dir) => PathBuf::from(dir),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("../oblodai-backend"),
    }
}

/// `x-oblodai-signing` of the backend's openapi.json; `None` (skip, loudly) without a backend
/// checkout, a failure when `OBLODAI_BACKEND` names one that has no spec.
fn signing() -> Option<Value> {
    let path = backend_root().join("services/core/api/openapi.json");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        assert!(
            std::env::var_os("OBLODAI_BACKEND").is_none(),
            "no spec at {}",
            path.display()
        );
        eprintln!(
            "signing protocol: skipped — no spec at {}; set OBLODAI_BACKEND",
            path.display()
        );
        return None;
    };
    let spec: Value = serde_json::from_str(&raw).expect("openapi.json");
    Some(spec["x-oblodai-signing"].clone())
}

fn names(list: &Value) -> Vec<String> {
    list.as_array()
        .expect("a list of header names")
        .iter()
        .map(|v| v.as_str().expect("a header name").to_string())
        .collect()
}

#[test]
fn the_generated_constants_are_the_contracts() {
    let Some(s) = signing() else { return };
    assert_eq!(
        names(&s["headers"]),
        [
            gen::HEADER_PUBLIC_ID,
            gen::HEADER_SIGNATURE,
            gen::HEADER_TIMESTAMP,
            gen::HEADER_IDEMPOTENCY_KEY
        ]
    );
    assert_eq!(
        names(&s["webhook"]["headers"]),
        [
            gen::HEADER_WEBHOOK_TIMESTAMP,
            gen::HEADER_WEBHOOK_SIGNATURE,
            gen::HEADER_WEBHOOK_SIGNATURE_PREV,
            gen::HEADER_WEBHOOK_EVENT,
            gen::HEADER_WEBHOOK_ID,
            gen::HEADER_WEBHOOK_EVENT_ID,
            gen::HEADER_WEBHOOK_EVENT_TIME,
        ]
    );
    assert_eq!(s["webhook"]["test_header"], gen::HEADER_WEBHOOK_TEST);
    assert_eq!(s["skew_seconds"], gen::SKEW_SECONDS);
    assert_eq!(s["max_body"], gen::MAX_BODY as u64);
    assert_eq!(
        s["max_idempotency_key_length"],
        gen::MAX_IDEMPOTENCY_KEY_LENGTH as u64
    );
    assert_eq!(s["algorithm"], gen::SIGNATURE_ALGORITHM);
}

#[test]
fn the_public_names_are_the_generated_values() {
    assert_eq!(
        oblodai::webhooks::HEADER_WEBHOOK_TEST,
        gen::HEADER_WEBHOOK_TEST
    );
    assert_eq!(
        oblodai::webhooks::DEFAULT_TOLERANCE_SECONDS,
        gen::SKEW_SECONDS
    );
    assert_eq!(
        oblodai::core::signing::SIGNATURE_SKEW_SECONDS,
        gen::SKEW_SECONDS
    );
    assert_eq!(
        oblodai::core::signing::HEADER_SIGNATURE,
        gen::HEADER_SIGNATURE
    );
    assert_eq!(
        oblodai::core::idempotency::MAX_IDEMPOTENCY_KEY_LENGTH,
        gen::MAX_IDEMPOTENCY_KEY_LENGTH
    );
}

/// No header name of the signing protocol — request, webhook delivery or rehearsal — is spelled in
/// a hand-written source file: every one comes from `src/generated`.
#[test]
fn no_signing_header_is_spelled_outside_generated() {
    let Some(s) = signing() else { return };
    let mut wanted = names(&s["headers"]);
    wanted.extend(names(&s["webhook"]["headers"]));
    wanted.push(
        s["webhook"]["test_header"]
            .as_str()
            .expect("test_header")
            .to_string(),
    );
    let wanted: Vec<String> = wanted.iter().map(|n| n.to_lowercase()).collect();
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    let mut stack = vec![src.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read src") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                if path.file_name() != Some("generated".as_ref()) {
                    stack.push(path);
                }
                continue;
            }
            let text = std::fs::read_to_string(&path)
                .expect("read source")
                .to_lowercase();
            for name in &wanted {
                if text.contains(name.as_str()) {
                    offenders.push(format!(
                        "{}: {name}",
                        path.strip_prefix(&src).unwrap().display()
                    ));
                }
            }
        }
    }
    offenders.sort();
    assert!(
        offenders.is_empty(),
        "signing header names outside src/generated: {offenders:?}"
    );
}
