//! Response envelopes, as `httpx`/`apiutil` on the core write them:
//!
//! ```text
//! success : { "state": 0, "result": <payload> }
//! list    : result = { "items": [...], "paginate": { total, per_page, offset, has_pages } }
//! error   : { "error": { code, message, field?, retryable, retry_after?, request_id? } }
//! ```
//!
//! Every non-`bare` route uses these; bare routes (PDF/CSV documents) bypass this module.

use serde::de::DeserializeOwned;
use serde_json::Value;

use super::util::parse_http_date;
use crate::error::{Error, ErrorDetail, Result};

/// Offset-pagination counters the core returns with every paged list.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Paginate {
    /// Total number of matching objects.
    pub total: i64,
    /// Page size actually applied.
    pub per_page: i64,
    /// Offset of this page.
    pub offset: i64,
    /// The core's own "there is more" flag.
    pub has_pages: bool,
}

/// One page of a list route.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub paginate: Paginate,
}

/// A list the core caps by catalog size rather than paginating (`list: plain` routes).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PlainList<T> {
    pub items: Vec<T>,
}

/// Interpret a response body. `body` is the raw bytes so non-JSON failures keep their evidence.
pub fn decode_envelope(
    http_status: u16,
    body: &[u8],
    retry_after: Option<&str>,
    location: Option<&str>,
) -> Result<Value> {
    let text = String::from_utf8_lossy(body).into_owned();
    let retry_after_header = parse_retry_after(retry_after, crate::core::util::unix_now());

    if (300..400).contains(&http_status) {
        let where_to = location.map(|l| format!(" to {l}")).unwrap_or_default();
        return Err(Error::from_envelope(
            http_status,
            ErrorDetail {
                code: "internal".into(),
                message: Some(format!(
                    "unexpected redirect (HTTP {http_status}){where_to}; check base_url"
                )),
                ..Default::default()
            },
            Some(text),
            true,
            retry_after_header,
        ));
    }

    let parsed: Option<Value> = if text.is_empty() {
        None
    } else {
        match serde_json::from_str(&text) {
            Ok(v) => Some(v),
            Err(_) => {
                if http_status >= 400 {
                    return Err(no_envelope(http_status, &text, retry_after_header, None));
                }
                return Err(Error::contract(
                    format!("expected a JSON envelope, got {}", describe(&text)),
                    http_status,
                    Some(text),
                ));
            }
        }
    };

    if let Some(Value::Object(map)) = &parsed {
        if let Some(err) = map.get("error").filter(|e| e.is_object()) {
            // Field by field: a malformed `retry_after` must not cost the `code` merchants branch
            // on, nor flip the core's authoritative `retryable`.
            match ErrorDetail::from_json(err) {
                Some(detail) => {
                    return Err(Error::from_envelope(
                        http_status,
                        detail,
                        Some(text),
                        false,
                        retry_after_header,
                    ));
                }
                None => {
                    // No usable `code`: nothing here is the core's classification, but a
                    // `request_id` is still worth quoting to support.
                    let request_id = ErrorDetail::request_id_of(err);
                    return Err(no_envelope(
                        http_status,
                        &text,
                        retry_after_header,
                        request_id,
                    ));
                }
            }
        }
    }
    if http_status >= 400 {
        return Err(no_envelope(http_status, &text, retry_after_header, None));
    }
    if let Some(Value::Object(map)) = &parsed {
        if map.get("state").and_then(Value::as_i64) == Some(0) {
            if let Some(result) = map.get("result") {
                return Ok(result.clone());
            }
        }
    }
    Err(Error::contract(
        format!(
            "response is not a {{state:0,result}} envelope: {}",
            describe(&text)
        ),
        http_status,
        Some(text),
    ))
}

/// Decode a `result` payload into a model, reporting a mismatch as a contract error rather than a
/// bare serde message.
pub fn decode_result<T: DeserializeOwned>(result: Value, route: &str) -> Result<T> {
    let raw = result.to_string();
    serde_json::from_value(result).map_err(|e| {
        Error::contract(
            format!("{route}: response body does not match the documented model: {e}"),
            200,
            Some(raw),
        )
    })
}

/// `Retry-After` as delta-seconds or an HTTP-date; `None` when absent or unparsable.
///
/// The result is clamped into `[0, MAX_RETRY_AFTER_SECONDS]`: a date in the year 9999 is a capped
/// delay, never an overflowing one, and a date in the past is 0, never negative.
pub fn parse_retry_after(value: Option<&str>, now: i64) -> Option<u64> {
    let v = value?.trim();
    if v.is_empty() {
        return None;
    }
    if v.bytes().all(|b| b.is_ascii_digit()) {
        // Parse wide, clamp, then narrow: a 30-digit header is capped, not rejected as garbage.
        return match v.parse::<u64>() {
            Ok(secs) => Some(secs.min(crate::error::MAX_RETRY_AFTER_SECONDS)),
            Err(_) => Some(crate::error::MAX_RETRY_AFTER_SECONDS),
        };
    }
    let at = parse_http_date(v)?;
    // i64 subtraction, then clamp in f64 so neither end can wrap.
    crate::error::clamp_retry_after((at - now) as f64)
}

fn no_envelope(
    status: u16,
    text: &str,
    retry_after_header: Option<u64>,
    request_id: Option<String>,
) -> Error {
    Error::from_envelope(
        status,
        ErrorDetail {
            code: "internal".into(),
            message: Some(format!(
                "HTTP {status} without an Oblodai error envelope ({}) — the answer came from a \
                 proxy or load balancer, not the API",
                describe(text)
            )),
            request_id,
            ..Default::default()
        },
        Some(text.to_string()),
        true,
        retry_after_header,
    )
}

fn describe(text: &str) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.is_empty() {
        return "<empty body>".to_string();
    }
    if flat.chars().count() > 120 {
        let head: String = flat.chars().take(120).collect();
        format!("{head}…")
    } else {
        flat
    }
}
