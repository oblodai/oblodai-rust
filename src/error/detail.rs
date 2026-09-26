//! The error envelope as it arrives, decoded field by field.
//!
//! One malformed field must never cost the others — least of all `code`, which merchants branch
//! on, or `retryable`, which is the gateway's own classification of the failure.

use std::collections::BTreeMap;

/// The error envelope as the core writes it.
///
/// Decoded field by field by [`ErrorDetail::from_json`]: one malformed field never costs the
/// others, and in particular never costs `code` (the value merchants branch on) or the core's
/// authoritative `retryable`. The type deliberately has no `Deserialize` impl so that an
/// all-or-nothing `serde_json::from_value` cannot creep back in.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ErrorDetail {
    pub code: String,
    pub message: Option<String>,
    pub field: Option<String>,
    /// Machine-readable facts about the refusal (string values only), keys documented by its
    /// code; `None` when the core sent none.
    pub details: Option<BTreeMap<String, String>>,
    pub retryable: Option<bool>,
    pub retry_after: Option<u64>,
    pub request_id: Option<String>,
}

/// Ceiling applied to every `retry_after` the SDK accepts, from the envelope or from the
/// `Retry-After` header: 24 h. It only guards against overflow and absurd values — the delay the
/// SDK actually sleeps is additionally capped by [`crate::RetryOptions::max_retry_after_ms`].
pub const MAX_RETRY_AFTER_SECONDS: u64 = 86_400;

/// A `retry_after` slot: an integer, a float or a numeric string, in seconds. Anything else
/// (bool, object, `NaN`, `"soon"`) is absent, never zero and never a wrong number.
pub(crate) fn parse_retry_after_value(value: Option<&serde_json::Value>) -> Option<u64> {
    let value = value?;
    let secs = match value {
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i as f64
            } else if let Some(u) = n.as_u64() {
                u as f64
            } else {
                n.as_f64()?
            }
        }
        serde_json::Value::String(s) => s.trim().parse::<f64>().ok()?,
        _ => return None,
    };
    clamp_retry_after(secs)
}

/// Clamp a number of seconds into `[0, MAX_RETRY_AFTER_SECONDS]`. `NaN` is not a delay.
pub(crate) fn clamp_retry_after(secs: f64) -> Option<u64> {
    if secs.is_nan() {
        return None;
    }
    // The comparison happens in f64 so no cast can wrap; only the clamped value is converted.
    Some(secs.clamp(0.0, MAX_RETRY_AFTER_SECONDS as f64) as u64)
}

impl ErrorDetail {
    /// Decode the `error` object of an envelope, field by field.
    ///
    /// Returns `None` when the object carries no usable `code`: the answer then has no envelope
    /// the SDK can trust, and the caller reports a synthetic error built from the HTTP status —
    /// while still keeping a `request_id` that is a string (see [`ErrorDetail::request_id_of`]).
    pub fn from_json(value: &serde_json::Value) -> Option<Self> {
        let map = value.as_object()?;
        let code = map.get("code").and_then(|v| v.as_str())?.trim().to_string();
        if code.is_empty() {
            return None;
        }
        Some(Self {
            code,
            message: str_field(map.get("message")),
            field: str_field(map.get("field")),
            details: details_field(map.get("details")),
            // Only a literal boolean overrides the status-derived default: a string "false" or a
            // 0 must never be read as the core's classification.
            retryable: map.get("retryable").and_then(serde_json::Value::as_bool),
            retry_after: parse_retry_after_value(map.get("retry_after")),
            request_id: str_field(map.get("request_id")),
        })
    }

    /// The `request_id` of an envelope whose `code` is unusable — worth keeping for support even
    /// when nothing else in the object can be trusted.
    pub fn request_id_of(value: &serde_json::Value) -> Option<String> {
        str_field(value.as_object()?.get("request_id"))
    }
}

/// The string values of `details`; `None` when it is absent, not an object or has none.
fn details_field(value: Option<&serde_json::Value>) -> Option<BTreeMap<String, String>> {
    let out: BTreeMap<String, String> = value?
        .as_object()?
        .iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
        .collect();
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// A string field: present and a JSON string, or absent. An empty string is absent.
fn str_field(value: Option<&serde_json::Value>) -> Option<String> {
    let s = value?.as_str()?;
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}
