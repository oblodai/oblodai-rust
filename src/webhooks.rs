//! Webhook verification — usable on its own, no client and no API key required.
//!
//! Deliveries are signed as:
//!
//! ```text
//! X-Webhook-Timestamp:      <unix seconds>
//! X-Webhook-Signature:      hex(HMAC-SHA256(secret, "<ts>." + rawBody))
//! X-Webhook-Signature-Prev: same, with the previous secret — only during a rotation overlap
//! X-Webhook-Event:          invoice.<status> | payout.<status> | wallet.paid
//! X-Webhook-Id:             stable per delivery (identical across retries) — your dedup key
//! X-Webhook-Event-Time:     unix seconds when the state change committed (order events by it)
//! X-Webhook-Test:           "true" on a rehearsal delivery — the body also carries `test: true`
//! ```
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
//! println!("{} {}", delivery.event.event_kind(), delivery.event.uuid());
//! # Ok(()) }
//! ```

use crate::contract::models::WebhookEvent;
use crate::core::signing::sign_webhook;
use crate::core::util::{constant_time_eq, unix_now};
use crate::error::{Error, Result};

pub const HEADER_WEBHOOK_TIMESTAMP: &str = "X-Webhook-Timestamp";
pub const HEADER_WEBHOOK_SIGNATURE: &str = "X-Webhook-Signature";
pub const HEADER_WEBHOOK_SIGNATURE_PREV: &str = "X-Webhook-Signature-Prev";
pub const HEADER_WEBHOOK_EVENT: &str = "X-Webhook-Event";
pub const HEADER_WEBHOOK_ID: &str = "X-Webhook-Id";
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

    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.pairs.push((name.into(), value.into()));
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
    /// Reject deliveries older/newer than this, in seconds. 0 disables the check.
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

    /// Widen or disable (`0`) the freshness window.
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
#[derive(Clone, Debug, PartialEq)]
pub struct WebhookDeliveryInfo {
    pub event: WebhookEvent,
    /// `X-Webhook-Id` — stable across retries of the same delivery; use it as your dedup key.
    pub id: Option<String>,
    /// `X-Webhook-Event` — `invoice.<status>` | `payout.<status>` | `wallet.paid`.
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

/// Like [`verify_webhook`], and also returns the delivery id, event type and times.
pub fn verify_webhook_delivery(
    raw_body: &[u8],
    headers: &Headers,
    options: &VerifyOptions,
) -> Result<WebhookDeliveryInfo> {
    let (ts_raw, sig) = match (
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

    // A merchant who has not swapped the stored secret yet verifies the Prev header with it; one
    // who already swapped but kept the old copy verifies the main header with the new secret.
    // Both hold, so both are tried.
    let prev_sig = headers.get(HEADER_WEBHOOK_SIGNATURE_PREV);
    let mut candidates: Vec<(&str, &str)> = vec![(sig, options.secret.as_str())];
    if let Some(prev) = prev_sig {
        candidates.push((prev, options.secret.as_str()));
    }
    if let Some(previous) = &options.previous_secret {
        candidates.push((sig, previous.as_str()));
        if let Some(prev) = prev_sig {
            candidates.push((prev, previous.as_str()));
        }
    }
    let ok = candidates.into_iter().any(|(provided, secret)| {
        constant_time_eq(
            &provided.to_ascii_lowercase(),
            &sign_webhook(secret, ts, raw_body),
        )
    });
    if !ok {
        return Err(Error::signature(
            "webhook.bad_signature",
            "signature does not match the body",
        ));
    }

    let event = parse_webhook(raw_body)?;
    Ok(WebhookDeliveryInfo {
        is_test: headers.get(HEADER_WEBHOOK_TEST).map(str::trim) == Some("true") || event.is_test(),
        event,
        id: headers.get(HEADER_WEBHOOK_ID).map(str::to_string),
        event_type: headers.get(HEADER_WEBHOOK_EVENT).map(str::to_string),
        event_time: headers
            .get(HEADER_WEBHOOK_EVENT_TIME)
            .and_then(|v| v.trim().parse().ok()),
        sent_at: ts,
    })
}

/// Parse a (previously verified) delivery body into a typed event, discriminated by `type`.
pub fn parse_webhook(raw_body: &[u8]) -> Result<WebhookEvent> {
    let value: serde_json::Value = serde_json::from_slice(raw_body)
        .map_err(|_| Error::signature("webhook.bad_signature", "body is not JSON"))?;
    let kind = value.get("type").and_then(serde_json::Value::as_str);
    match kind {
        Some("payment") | Some("payout") | Some("wallet") => {}
        Some(other) => {
            return Err(Error::signature(
                "webhook.bad_signature",
                format!("unknown event type \"{other}\""),
            ))
        }
        None => {
            return Err(Error::signature(
                "webhook.bad_signature",
                "body lacks the type field every event carries",
            ))
        }
    }
    serde_json::from_value(value).map_err(|e| {
        Error::signature(
            "webhook.bad_signature",
            format!("body is not a known event: {e}"),
        )
    })
}

/// True for rehearsal deliveries (`webhooks.test`, sandbox) — never act on them as if money moved.
pub fn is_test_event(event: &WebhookEvent) -> bool {
    event.is_test()
}

/// Deliveries can arrive out of order (a retried `paid` after a `refund`). Keep the last
/// `sequence` you processed per object and skip anything not newer.
pub fn is_stale_event(event: &WebhookEvent, last_processed_sequence: Option<i64>) -> bool {
    matches!(last_processed_sequence, Some(last) if event.sequence() <= last)
}
