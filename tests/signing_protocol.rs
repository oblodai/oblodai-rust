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
/// a hand-written source file, library or example: every one comes from `src/generated`.
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
    let mut offenders = Vec::new();
    for (path, text) in hand_written_sources() {
        let text = text.to_lowercase();
        for name in &wanted {
            if text.contains(name.as_str()) {
                offenders.push(format!("{path}: {name}"));
            }
        }
    }
    offenders.sort();
    assert!(
        offenders.is_empty(),
        "signing header names outside src/generated: {offenders:?}"
    );
}

/// Every hand-written Rust file that ships or is run — `src/` outside `src/generated`, and the
/// examples (the README points at them and `cargo test` builds them) — with its path relative to the
/// crate root.
fn hand_written_sources() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut out = Vec::new();
    let mut stack = vec![root.join("src"), root.join("examples")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read sources") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                if path.file_name() != Some("generated".as_ref()) {
                    stack.push(path);
                }
                continue;
            }
            if path.extension() != Some("rs".as_ref()) {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("read source");
            out.push((path.strip_prefix(root).unwrap().display().to_string(), text));
        }
    }
    assert!(
        out.iter().any(|(p, _)| p.starts_with("examples")),
        "no examples scanned"
    );
    out
}

/// `needle` in `text` as a whole token: no word character or `.` on either side.
fn has_token(text: &str, needle: &str) -> bool {
    let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.');
    text.match_indices(needle).any(|(i, _)| {
        !word(text[..i].chars().next_back()) && !word(text[i + needle.len()..].chars().next())
    })
}

/// No hand-written source spells a literal of the body or idempotency-key limit — decimal, or
/// `1 << n` for a power of two; digit separators (`1_048_576`) do not hide one. Both are read from
/// `src/generated`, so a changed limit reaches the SDK by regeneration alone. The skew is not
/// scanned for: its value is also an HTTP status class (`< 300`); the alias assertions hold it.
#[test]
fn no_signing_limit_is_spelled_outside_generated() {
    let mut needles = vec![
        gen::MAX_BODY.to_string(),
        gen::MAX_IDEMPOTENCY_KEY_LENGTH.to_string(),
    ];
    if gen::MAX_BODY.is_power_of_two() {
        needles.push(format!("1<<{}", gen::MAX_BODY.trailing_zeros()));
    }
    let mut offenders = Vec::new();
    for (path, text) in hand_written_sources() {
        // Digit separators out, `<<` without spaces: `1_048_576` reads as `1048576`, `1 << 20` as `1<<20`.
        let chars: Vec<char> = text.chars().collect();
        let text: String = chars
            .iter()
            .enumerate()
            .filter(|&(i, &c)| {
                c != '_'
                    || i == 0
                    || i + 1 == chars.len()
                    || !chars[i - 1].is_ascii_digit()
                    || !chars[i + 1].is_ascii_digit()
            })
            .map(|(_, &c)| c)
            .collect();
        let text = text
            .replace(" << ", "<<")
            .replace("<< ", "<<")
            .replace(" <<", "<<");
        for needle in &needles {
            if has_token(&text, needle) {
                offenders.push(format!("{path}: {needle}"));
            }
        }
    }
    offenders.sort();
    assert!(
        offenders.is_empty(),
        "signing limits spelled outside src/generated: {offenders:?}"
    );
}
