//! Request signing — the exact recipe the core verifies (`crypto.SignRequest`):
//!
//! ```text
//! canonical = ts "\n" METHOD "\n" requestURI "\n" idempotencyKey "\n" body
//! signature = hex(HMAC-SHA256(secret, canonical))
//! ```
//!
//! - `ts` is unix seconds; the core accepts ±300 s of skew.
//! - `requestURI` is path + raw query (`/v1/x?limit=1`), never the origin.
//! - The idempotency slot is the empty string when no `Idempotency-Key` header is sent.
//! - `body` is the byte-exact request body; GETs sign an empty body.
//!
//! Pure: no clock, no I/O. The vectors in `tests/unit_signing.rs` come from the core test suite.

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// What goes into a request signature.
#[derive(Clone, Debug)]
pub struct SignInput<'a> {
    /// Unix timestamp in seconds, as sent in `X-Timestamp`.
    pub ts: i64,
    /// Upper-case HTTP method.
    pub method: &'a str,
    /// Path plus raw query string, exactly as the request line carries it.
    pub request_uri: &'a str,
    /// Value of the `Idempotency-Key` header, or `None` when absent.
    pub idempotency_key: Option<&'a str>,
    /// Request body bytes (already-serialized JSON, or empty).
    pub body: &'a [u8],
}

/// The string the signature is computed over. Useful when debugging a 401 from the gateway.
pub fn canonical_string(input: &SignInput<'_>) -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}",
        input.ts,
        input.method.to_uppercase(),
        input.request_uri,
        input.idempotency_key.unwrap_or(""),
        String::from_utf8_lossy(input.body),
    )
}

/// Lower-case hex HMAC-SHA256 of the canonical string.
pub fn sign_request(secret: &str, input: &SignInput<'_>) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key size");
    mac.update(
        format!(
            "{}\n{}\n{}\n{}\n",
            input.ts,
            input.method.to_uppercase(),
            input.request_uri,
            input.idempotency_key.unwrap_or(""),
        )
        .as_bytes(),
    );
    mac.update(input.body);
    hex::encode(mac.finalize().into_bytes())
}

/// Webhook signature — `webhook.Sign` on the core side:
///
/// ```text
/// signature = hex(HMAC-SHA256(secret, "<unix ts>." + payload))
/// ```
///
/// The payload is signed verbatim, so verifiers must use the raw request bytes, never a
/// re-serialized parse of them.
pub fn sign_webhook(secret: &str, ts: i64, payload: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key size");
    mac.update(format!("{ts}.").as_bytes());
    mac.update(payload);
    hex::encode(mac.finalize().into_bytes())
}

/// Signed request headers as the core reads them.
pub const HEADER_PUBLIC_ID: &str = "X-Public-Id";
pub const HEADER_SIGNATURE: &str = "X-Signature";
pub const HEADER_TIMESTAMP: &str = "X-Timestamp";
pub const HEADER_IDEMPOTENCY_KEY: &str = "Idempotency-Key";
/// Admin gate of a self-hosted gateway, on `onboard` routes only.
pub const HEADER_ADMIN_TOKEN: &str = "X-Admin-Token";

/// Accepted clock skew on the core side, in seconds.
pub const SIGNATURE_SKEW_SECONDS: i64 = 300;
