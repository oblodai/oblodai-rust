//! Request signing — the exact recipe the core verifies (`crypto.SignRequest`), as the contract's
//! `x-oblodai-signing` states it ([`crate::generated::signing`]):
//!
//! ```text
//! canonical = the parts of REQUEST_CANONICAL_ORDER joined by REQUEST_CANONICAL_SEPARATOR
//!           = ts "\n" METHOD "\n" requestURI "\n" idempotencyKey "\n" body   (today)
//! signature = hex(HMAC-SHA256(secret, canonical))
//! ```
//!
//! - `ts` is unix seconds; the core accepts ±[`SIGNATURE_SKEW_SECONDS`] of skew.
//! - `requestURI` is path + raw query (`/v1/x?limit=1`), never the origin.
//! - The idempotency slot is the empty string when no [`HEADER_IDEMPOTENCY_KEY`] is sent.
//! - `body` is the byte-exact request body; GETs sign an empty body.
//!
//! The header names, the order of the parts and the separators are generated from the contract,
//! never written here: a header renamed in the core reaches this SDK with `make sdk`.
//!
//! Pure: no clock, no I/O. The vectors in `tests/unit_signing.rs` come from the core test suite.

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::generated::signing::{
    RequestPart, WebhookPart, REQUEST_CANONICAL_ORDER, REQUEST_CANONICAL_SEPARATOR,
    WEBHOOK_CANONICAL_ORDER, WEBHOOK_CANONICAL_SEPARATOR,
};
pub use crate::generated::signing::{
    HEADER_IDEMPOTENCY_KEY, HEADER_PUBLIC_ID, HEADER_SIGNATURE, HEADER_TIMESTAMP,
};

type HmacSha256 = Hmac<Sha256>;

/// What goes into a request signature.
#[derive(Clone, Debug)]
pub struct SignInput<'a> {
    /// Unix timestamp in seconds, as sent in [`HEADER_TIMESTAMP`].
    pub ts: i64,
    /// Upper-case HTTP method.
    pub method: &'a str,
    /// Path plus raw query string, exactly as the request line carries it.
    pub request_uri: &'a str,
    /// Value of the [`HEADER_IDEMPOTENCY_KEY`] header, or `None` when absent.
    pub idempotency_key: Option<&'a str>,
    /// Request body bytes (already-serialized JSON, or empty).
    pub body: &'a [u8],
}

/// The bytes of one canonical part of a request.
fn request_part(input: &SignInput<'_>, part: RequestPart, method: &str) -> Vec<u8> {
    match part {
        RequestPart::Ts => input.ts.to_string().into_bytes(),
        RequestPart::Method => method.as_bytes().to_vec(),
        RequestPart::RequestUri => input.request_uri.as_bytes().to_vec(),
        RequestPart::IdempotencyKey => input.idempotency_key.unwrap_or("").as_bytes().to_vec(),
        RequestPart::Body => input.body.to_vec(),
    }
}

/// The canonical bytes of a request: [`REQUEST_CANONICAL_ORDER`] joined by
/// [`REQUEST_CANONICAL_SEPARATOR`].
fn request_canonical(input: &SignInput<'_>) -> Vec<u8> {
    let method = input.method.to_uppercase();
    let mut out = Vec::with_capacity(input.body.len() + input.request_uri.len() + 64);
    for (i, part) in REQUEST_CANONICAL_ORDER.iter().enumerate() {
        if i > 0 {
            out.extend_from_slice(REQUEST_CANONICAL_SEPARATOR.as_bytes());
        }
        out.extend_from_slice(&request_part(input, *part, &method));
    }
    out
}

/// The string the signature is computed over. Useful when debugging a 401 from the gateway.
pub fn canonical_string(input: &SignInput<'_>) -> String {
    String::from_utf8_lossy(&request_canonical(input)).into_owned()
}

/// Lower-case hex HMAC-SHA256 of the canonical string.
pub fn sign_request(secret: &str, input: &SignInput<'_>) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key size");
    mac.update(&request_canonical(input));
    hex::encode(mac.finalize().into_bytes())
}

/// Webhook signature — `webhook.Sign` on the core side: lower-case hex HMAC-SHA256 of
/// [`WEBHOOK_CANONICAL_ORDER`] joined by [`WEBHOOK_CANONICAL_SEPARATOR`] (today
/// `"<unix ts>." + payload`).
///
/// The payload is signed verbatim, so verifiers must use the raw request bytes, never a
/// re-serialized parse of them.
pub fn sign_webhook(secret: &str, ts: i64, payload: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key size");
    for (i, part) in WEBHOOK_CANONICAL_ORDER.iter().enumerate() {
        if i > 0 {
            mac.update(WEBHOOK_CANONICAL_SEPARATOR.as_bytes());
        }
        match part {
            WebhookPart::Ts => mac.update(ts.to_string().as_bytes()),
            WebhookPart::Payload => mac.update(payload),
        }
    }
    hex::encode(mac.finalize().into_bytes())
}

/// Admin gate of a self-hosted gateway, on `onboard` routes only.
pub const HEADER_ADMIN_TOKEN: &str = "X-Admin-Token";

/// Accepted clock skew on the core side, in seconds (`skew_seconds` of the contract).
pub const SIGNATURE_SKEW_SECONDS: i64 = crate::generated::signing::SKEW_SECONDS;
