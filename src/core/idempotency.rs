//! Idempotency keys.
//!
//! On create-type routes the core caches the first response per key for the merchant and replays
//! it on retries; a different body under the same key is a 409 `idempotency.key_reused`. The SDK
//! generates a key once per logical call and reuses it on every retry, so a timeout never turns
//! into a double payout.

use crate::error::{Error, Result};

pub const MAX_IDEMPOTENCY_KEY_LENGTH: usize = 255;

/// A fresh v4 UUID from the platform CSPRNG.
pub fn new_idempotency_key() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Validate a caller-supplied key before it is signed and sent.
pub fn assert_idempotency_key(key: &str) -> Result<()> {
    if key.is_empty() {
        return Err(bad_key("idempotency_key must not be empty"));
    }
    if key.len() > MAX_IDEMPOTENCY_KEY_LENGTH {
        return Err(bad_key(format!(
            "idempotency_key is too long (max {MAX_IDEMPOTENCY_KEY_LENGTH} chars)"
        )));
    }
    // Header values must be visible ASCII: the key is signed verbatim, so a stray control char or
    // surrounding whitespace would silently change the MAC on one side only.
    if !key.bytes().all(|b| (0x21..=0x7e).contains(&b)) {
        return Err(bad_key(
            "idempotency_key must be printable ASCII without spaces",
        ));
    }
    Ok(())
}

fn bad_key(message: impl Into<String>) -> Error {
    Error::config("sdk.bad_idempotency_key", message, Some("idempotency_key"))
}
