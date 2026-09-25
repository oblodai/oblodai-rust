//! Webhook verification — usable on its own, no client and no API key required.
//!
//! Deliveries are signed as:
//!
//! ```text
//! X-Webhook-Timestamp:      <unix seconds>
//! X-Webhook-Signature:      hex(HMAC-SHA256(secret, "<ts>." + rawBody))
//! X-Webhook-Signature-Prev: same, with the previous secret — only during a rotation overlap
//! X-Webhook-Event:          the event name, one of WEBHOOK_EVENTS (invoice.paid, payout.failed, …)
//! X-Webhook-Id:             the delivery — identical across its retries; a resend gets a new one
//! X-Webhook-Event-Id:       the state — identical across retries AND resends; your dedup key
//! X-Webhook-Event-Time:     unix seconds when the state change committed (order events by it)
//! X-Webhook-Test:           "true" on a rehearsal delivery — the body also carries `test: true`
//! ```
//!
//! The event names, their kinds (the body's `type`) and the models of the kinds come from the
//! contract: [`WebhookEvent`], [`KNOWN_EVENT_KINDS`] and [`WEBHOOK_EVENTS`] are generated
//! (`crate::generated::webhooks`).
//!
//! A rehearsal delivery (`webhooks.test`, sandbox) is signed exactly like a live one. Check
//! [`WebhookDeliveryInfo::is_test`] (or [`is_test_event`]) and never act on one as if money moved.
//!
//! Always verify over the **raw** request bytes; a re-serialized parse will not match.
//!
//! ```no_run
//! use oblodai::webhooks::{verify_webhook_delivery, Headers, VerifyOptions};
//!
//! # fn handler(raw_body: &[u8], header_pairs: Vec<(String, String)>) -> oblodai::Result<()> {
//! let headers = Headers::from_pairs(header_pairs);
//! let delivery = verify_webhook_delivery(
//!     raw_body,
//!     &headers,
//!     &VerifyOptions::new(std::env::var("OBLODAI_WEBHOOK_SECRET").unwrap()),
//! )?;
//! println!("{} {}", delivery.event.event_kind(), delivery.event.object_id());
//! # Ok(()) }
//! ```

use crate::core::signing::sign_webhook;
use crate::core::util::{constant_time_eq, unix_now};
use crate::error::{Error, Result};

pub use crate::generated::webhooks::{WebhookEvent, KNOWN_EVENT_KINDS, WEBHOOK_EVENTS};

pub const HEADER_WEBHOOK_TIMESTAMP: &str = "X-Webhook-Timestamp";
pub const HEADER_WEBHOOK_SIGNATURE: &str = "X-Webhook-Signature";
pub const HEADER_WEBHOOK_SIGNATURE_PREV: &str = "X-Webhook-Signature-Prev";
pub const HEADER_WEBHOOK_EVENT: &str = "X-Webhook-Event";
pub const HEADER_WEBHOOK_ID: &str = "X-Webhook-Id";
pub const HEADER_WEBHOOK_EVENT_ID: &str = "X-Webhook-Event-Id";
pub const HEADER_WEBHOOK_EVENT_TIME: &str = "X-Webhook-Event-Time";
pub const HEADER_WEBHOOK_TEST: &str = "X-Webhook-Test";

/// Default freshness window, seconds.
pub const DEFAULT_TOLERANCE_SECONDS: i64 = 300;

/// The delivery headers, however your web framework spells them.
///
/// Lookup is case-insensitive, so `hyper`, `axum`, `actix` and a plain `Vec` all work.
#[derive(Clone, Debug, Default)]
pub struct Headers {
    pairs: Vec<(String, String)>,
}

impl Headers {
    pub fn new() -> Self {
        Self::default()
    }

    /// Collect from anything that yields name/value pairs.
    pub fn from_pairs<I, K, V>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        Self {
            pairs: pairs
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }

    /// Set a header, replacing any existing one with the same name (case-insensitively) — the
    /// same thing a map does, so the value you set last is the value [`get`](Self::get) returns.
    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<String>) -> &mut Self {
        let name = name.into();
        let value = value.into();
        match self
            .pairs
            .iter_mut()
            .find(|(k, _)| k.eq_ignore_ascii_case(&name))
        {
            Some(slot) => slot.1 = value,
            None => self.pairs.push((name, value)),
        }
        self
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        crate::core::util::header_value(&self.pairs, name)
    }
}

impl<K: Into<String>, V: Into<String>> FromIterator<(K, V)> for Headers {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Headers::from_pairs(iter)
    }
}

/// How to verify a delivery.
#[derive(Clone, Debug)]
pub struct VerifyOptions {
    /// The endpoint secret from `webhooks().register()` / `rotate_secret()`.
    pub secret: String,
    /// During a rotation keep the outgoing secret here. Deliveries queued before the rotation stay
    /// signed with it for their whole retry life (~26 h), so keep it at least that long.
    pub previous_secret: Option<String>,
    /// Reject deliveries older/newer than this, in seconds. `0` disables the freshness check
    /// entirely (replaying a recorded delivery, a queue that may sit for hours). A negative value
    /// is a configuration error, not "disabled".
    pub tolerance_seconds: i64,
    /// Current unix time; override in tests.
    pub now: Option<i64>,
}

impl VerifyOptions {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            previous_secret: None,
            tolerance_seconds: DEFAULT_TOLERANCE_SECONDS,
            now: None,
        }
    }

    /// Also accept the previous secret, for the rotation overlap.
    pub fn previous_secret(mut self, secret: impl Into<String>) -> Self {
        self.previous_secret = Some(secret.into());
        self
    }

    /// Widen or disable (`0`) the freshness window. Negative values are refused at verify time
    /// with a config error.
    pub fn tolerance_seconds(mut self, seconds: i64) -> Self {
        self.tolerance_seconds = seconds;
        self
    }

    /// Pin the clock (tests, replaying a recorded delivery).
    pub fn now(mut self, now: i64) -> Self {
        self.now = Some(now);
        self
    }
}

/// A verified delivery: the event plus the advisory headers worth keeping.
///
/// `#[non_exhaustive]`: the gateway can add a delivery header without that being a breaking change
/// here. Read the fields; do not construct one.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct WebhookDeliveryInfo {
    pub event: WebhookEvent,
    /// `X-Webhook-Id` — the delivery: identical across its retries, but a resend
    /// (`webhooks().resend_payment()`, a sandbox replay) is a new delivery with a new id.
    pub id: Option<String>,
    /// `X-Webhook-Event-Id` — the state the delivery carries: identical for the original, every
    /// retry and every resend of the same state, different once the state changes. Deduplicate on it.
    pub event_id: Option<String>,
    /// `X-Webhook-Event` — the event name, one of [`WEBHOOK_EVENTS`].
    pub event_type: Option<String>,
    /// `X-Webhook-Event-Time` — unix seconds when the state change committed.
    pub event_time: Option<i64>,
    /// `X-Webhook-Timestamp` — unix seconds when this attempt was sent.
    pub sent_at: i64,
    /// A rehearsal delivery (`X-Webhook-Test: true` / body `test: true`): signed like a live one,
    /// but no money moved.
    pub is_test: bool,
}

/// Verify the signature and freshness, then parse. Never returns an unverified body.
pub fn verify_webhook(
    raw_body: &[u8],
    headers: &Headers,
    options: &VerifyOptions,
) -> Result<WebhookEvent> {
    Ok(verify_webhook_delivery(raw_body, headers, options)?.event)
}

/// Like [`verify_webhook`], and also returns the delivery and event ids, event type and times.
pub fn verify_webhook_delivery(
    raw_body: &[u8],
    headers: &Headers,
    options: &VerifyOptions,
) -> Result<WebhookDeliveryInfo> {
    // Configuration first: an empty secret would "verify" with the empty key, and a negative
    // tolerance is a typo for "disabled" that would reject every delivery.
    assert_options(options)?;

    let (ts_raw, sig_raw) = match (
        headers.get(HEADER_WEBHOOK_TIMESTAMP),
        headers.get(HEADER_WEBHOOK_SIGNATURE),
    ) {
        (Some(ts), Some(sig)) => (ts, sig),
        _ => {
            return Err(Error::signature(
                "webhook.missing_header",
                format!("missing {HEADER_WEBHOOK_TIMESTAMP} or {HEADER_WEBHOOK_SIGNATURE}"),
            ))
        }
    };
    let ts: i64 = ts_raw.trim().parse().map_err(|_| {
        Error::signature(
            "webhook.bad_signature",
            "timestamp header is not an integer",
        )
    })?;
    let sig = normalize_signature(sig_raw)?;
    let prev_sig = headers
        .get(HEADER_WEBHOOK_SIGNATURE_PREV)
        .map(normalize_signature)
        .transpose()?;

    // A merchant who has not swapped the stored secret yet verifies the Prev header with it; one
    // who already swapped but kept the old copy verifies the main header with the new secret.
    // Both hold, so both are tried.
    let mut candidates: Vec<(&str, &str)> = vec![(sig.as_str(), options.secret.as_str())];
    if let Some(prev) = &prev_sig {
        candidates.push((prev.as_str(), options.secret.as_str()));
    }
    if let Some(previous) = &options.previous_secret {
        candidates.push((sig.as_str(), previous.as_str()));
        if let Some(prev) = &prev_sig {
            candidates.push((prev.as_str(), previous.as_str()));
        }
    }
    let ok = candidates
        .into_iter()
        .any(|(provided, secret)| constant_time_eq(provided, &sign_webhook(secret, ts, raw_body)));
    if !ok {
        return Err(Error::signature(
            "webhook.bad_signature",
            "signature does not match the body",
        ));
    }

    // Only now: the freshness window must not answer questions about a body whose MAC has not
    // been checked, or it becomes a pre-authentication oracle.
    if options.tolerance_seconds > 0 {
        let now = options.now.unwrap_or_else(unix_now);
        if (now - ts).abs() > options.tolerance_seconds {
            return Err(Error::signature(
                "webhook.stale_timestamp",
                format!(
                    "delivery timestamp {ts} is outside the ±{}s window",
                    options.tolerance_seconds
                ),
            ));
        }
    }

    let event = parse_webhook(raw_body)?;
    Ok(WebhookDeliveryInfo {
        is_test: headers.get(HEADER_WEBHOOK_TEST).map(str::trim) == Some("true") || event.is_test(),
        event,
        id: headers.get(HEADER_WEBHOOK_ID).map(str::to_string),
        event_id: headers.get(HEADER_WEBHOOK_EVENT_ID).map(str::to_string),
        event_type: headers.get(HEADER_WEBHOOK_EVENT).map(str::to_string),
        event_time: headers
            .get(HEADER_WEBHOOK_EVENT_TIME)
            .and_then(|v| v.trim().parse().ok()),
        sent_at: ts,
    })
}

/// Reject a configuration that cannot verify anything, before a single byte is hashed.
fn assert_options(options: &VerifyOptions) -> Result<()> {
    if options.secret.is_empty() {
        return Err(Error::config(
            "sdk.bad_config",
            "webhook secret is empty; verifying with the empty key would accept forged deliveries",
            Some("secret"),
        ));
    }
    if options
        .previous_secret
        .as_ref()
        .is_some_and(|p| p.is_empty())
    {
        return Err(Error::config(
            "sdk.bad_config",
            "previous_secret was supplied but is empty; drop it instead",
            Some("previous_secret"),
        ));
    }
    if options.tolerance_seconds < 0 {
        return Err(Error::config(
            "sdk.bad_config",
            format!(
                "tolerance_seconds must not be negative (got {}); use 0 to disable the freshness \
                 check",
                options.tolerance_seconds
            ),
            Some("tolerance_seconds"),
        ));
    }
    Ok(())
}

/// Normalize a signature header: surrounding whitespace is tolerated and upper-case hex is
/// accepted, but a `0x` prefix is not hex the core ever writes and is refused rather than guessed.
fn normalize_signature(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.len() >= 2 && trimmed[..2].eq_ignore_ascii_case("0x") {
        return Err(Error::signature(
            "webhook.bad_signature",
            "signature header carries a \"0x\" prefix; the gateway sends bare hex",
        ));
    }
    Ok(trimmed.to_ascii_lowercase())
}

/// Parse a (previously verified) delivery body into a typed event, discriminated by `type`.
///
/// A `type` this snapshot does not know yields [`WebhookEvent::Other`] rather than an error: a new
/// event kind must never make a receiver reject an authentic delivery. A body that is not JSON, or
/// that claims a known type with fields of the wrong shape, is `webhook.bad_payload`
/// ([`ErrorKind::Contract`](crate::ErrorKind::Contract)) — never a signature failure.
pub fn parse_webhook(raw_body: &[u8]) -> Result<WebhookEvent> {
    let value: serde_json::Value = serde_json::from_slice(raw_body)
        .map_err(|e| Error::bad_payload(format!("delivery body is not JSON: {e}")))?;
    let kind = match value.get("type").and_then(serde_json::Value::as_str) {
        Some(kind) => kind.to_string(),
        None => {
            return Err(Error::bad_payload(
                "delivery body lacks the string `type` field every event carries",
            ))
        }
    };
    if !KNOWN_EVENT_KINDS.contains(&kind.as_str()) {
        // Unknown to this snapshot: hand it over raw so the receiver can 200 it and move on.
        return Ok(WebhookEvent::Other(value));
    }
    serde_json::from_value(value)
        .map_err(|e| Error::bad_payload(format!("delivery body is not a valid {kind} event: {e}")))
}

/// Whether this snapshot models the event's `type`.
///
/// The counterpart of the reference SDK's `isKnownEvent()`: `false` means the gateway sent a `type`
/// newer than this contract snapshot. The delivery verified and is authentic — acknowledge it.
pub fn is_known_event(event: &WebhookEvent) -> bool {
    event.is_known()
}

/// True for rehearsal deliveries (`webhooks.test`, sandbox) — never act on them as if money moved.
pub fn is_test_event(event: &WebhookEvent) -> bool {
    event.is_test()
}

/// Deliveries can arrive out of order (a retried `paid` after a `refund`). Keep the last
/// `sequence` you processed per object and skip anything not newer.
/// An event without a usable `sequence` (an unknown type, a field the gateway omitted) is never
/// reported stale: dropping a delivery you cannot order is worse than handling it twice.
pub fn is_stale_event(event: &WebhookEvent, last_processed_sequence: Option<i64>) -> bool {
    match (last_processed_sequence, event.sequence()) {
        (Some(last), Some(sequence)) => sequence <= last,
        _ => false,
    }
}
