//! Error model. One type, [`Error`], mirrors the core's error envelope:
//!
//! ```json
//! { "error": { "code": "…", "message": "…", "field": "…", "retryable": false,
//!              "retry_after": 30, "request_id": "…" } }
//! ```
//!
//! `retryable` is authoritative when the core wrote the envelope: it is the core's own
//! classification of the failure. A response without an envelope (a proxy 502, an HTML 503) is
//! [`Error::synthetic`] — the core never saw or never answered the request — and is retried only
//! when repeating is safe. [`ErrorKind`] exists for matching; the discriminator is always `code`.

use serde::ser::{Serialize, SerializeStruct, Serializer};

mod detail;
mod kind;

pub(crate) use detail::clamp_retry_after;
pub use detail::{ErrorDetail, MAX_RETRY_AFTER_SECONDS};
pub use kind::ErrorKind;

/// Every failure the SDK reports.
///
/// The type is `Clone`, so an underlying `reqwest::Error` is folded into [`message`](Self::message)
/// rather than kept as a source: `std::error::Error::source` is always `None`, and `anyhow`'s chain
/// rendering therefore adds nothing beyond the message. Everything worth acting on is already a
/// method here.
///
/// `Display` is `[code] message (request_id=…)` — the suffix only when there is an id — so a log
/// line alone is enough to find the call on the gateway's side. It (and only it) may quote up to
/// 120 characters of an unparsable answer, because that is what tells you a proxy answered instead
/// of the gateway. [`Debug`] and the `Serialize` impl never carry the body — use those in
/// structured logs.
#[derive(Clone)]
pub struct Error {
    kind: ErrorKind,
    code: String,
    message: String,
    http_status: u16,
    retryable: bool,
    retry_after: Option<u64>,
    request_id: Option<String>,
    field: Option<String>,
    synthetic: bool,
    /// The raw body. Never printed by `Debug` and never serialized, so a log cannot leak it.
    raw: Option<String>,
}

impl Error {
    /// Which family the failure belongs to.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Stable machine code (`family.reason`), e.g. `payout.insufficient_funds`.
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Code family (`payout` in `payout.insufficient_funds`).
    pub fn family(&self) -> &str {
        match self.code.find('.') {
            Some(i) => &self.code[..i],
            None => &self.code,
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    /// HTTP status, or 0 when no response was received.
    pub fn http_status(&self) -> u16 {
        self.http_status
    }

    /// Whether repeating the identical request can succeed later. The SDK already retried what it
    /// safely could, so a `true` here means "retry later", not "retry now".
    pub fn retryable(&self) -> bool {
        self.retryable
    }

    /// Seconds to wait before retrying, when the core (or a `Retry-After` header) gave a hint.
    pub fn retry_after(&self) -> Option<u64> {
        self.retry_after
    }

    /// Server-side request id — quote it when contacting support.
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    /// The request field the error refers to, for validation failures.
    pub fn field(&self) -> Option<&str> {
        self.field.as_deref()
    }

    /// No core envelope: the answer came from something in front of the core.
    pub fn synthetic(&self) -> bool {
        self.synthetic
    }

    /// The raw body that produced this error, when one was read.
    pub fn raw_body(&self) -> Option<&str> {
        self.raw.as_deref()
    }

    /// The request id of the call, when the answer did not carry one of its own.
    pub(crate) fn with_request_id(mut self, request_id: &str) -> Self {
        if self.request_id.is_none() && !request_id.is_empty() {
            self.request_id = Some(request_id.to_string());
        }
        self
    }

    /// A long-running operation did not finish within the time [`wait`](crate::Job::wait) was
    /// given. Not retryable as such: the job keeps running, wait on it again.
    pub fn job_timeout(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Transport,
            code: "sdk.job_timeout".to_string(),
            message: message.into(),
            http_status: 0,
            retryable: false,
            retry_after: None,
            request_id: None,
            field: None,
            synthetic: false,
            raw: None,
        }
    }

    /// Raised before any request is sent.
    pub fn config(code: &str, message: impl Into<String>, field: Option<&str>) -> Self {
        Self {
            kind: ErrorKind::Config,
            code: code.to_string(),
            message: message.into(),
            http_status: 0,
            retryable: false,
            retry_after: None,
            request_id: None,
            field: field.map(str::to_string),
            synthetic: false,
            raw: None,
        }
    }

    /// The request never produced an HTTP response. `transport.timeout` and `transport.network`
    /// are retryable (subject to the safe-to-repeat rule); aborts and deadlines are not.
    pub fn transport(code: &str, message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Transport,
            code: code.to_string(),
            message: message.into(),
            http_status: 0,
            retryable: code == "transport.timeout" || code == "transport.network",
            retry_after: None,
            request_id: None,
            field: None,
            synthetic: false,
            raw: None,
        }
    }

    /// The response is not the documented envelope.
    pub fn contract(message: impl Into<String>, http_status: u16, raw: Option<String>) -> Self {
        Self {
            kind: ErrorKind::Contract,
            code: "sdk.bad_envelope".to_string(),
            message: message.into(),
            http_status,
            retryable: false,
            retry_after: None,
            request_id: None,
            field: None,
            synthetic: false,
            raw,
        }
    }

    /// The answer was larger than the SDK is willing to buffer.
    pub fn response_too_large(message: impl Into<String>, http_status: u16) -> Self {
        Self {
            kind: ErrorKind::Contract,
            code: "sdk.response_too_large".to_string(),
            message: message.into(),
            http_status,
            retryable: false,
            retry_after: None,
            request_id: None,
            field: None,
            synthetic: false,
            raw: None,
        }
    }

    /// A delivery whose signature verified but whose body the SDK cannot read.
    ///
    /// Deliberately NOT in the signature family: a receiver that answers 401 to a forged delivery
    /// must not answer 401 to an authentic one, or the gateway retries it for ~26 h. The code is
    /// `webhook.bad_payload` and the kind is [`ErrorKind::Contract`].
    pub fn bad_payload(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Contract,
            code: "webhook.bad_payload".to_string(),
            message: message.into(),
            http_status: 0,
            retryable: false,
            retry_after: None,
            request_id: None,
            field: None,
            synthetic: false,
            raw: None,
        }
    }

    /// Webhook verification failed.
    pub fn signature(code: &str, message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Signature,
            code: code.to_string(),
            message: message.into(),
            http_status: 0,
            retryable: false,
            retry_after: None,
            request_id: None,
            field: None,
            synthetic: false,
            raw: None,
        }
    }

    /// Build the error an API answer describes.
    ///
    /// `synthetic` marks an answer that carried no `{error}` envelope: then `retryable` is decided
    /// by the status alone, because the core's own classification is not available.
    pub fn from_envelope(
        http_status: u16,
        detail: ErrorDetail,
        raw: Option<String>,
        synthetic: bool,
        retry_after_header: Option<u64>,
    ) -> Self {
        let code = if detail.code.is_empty() {
            "internal".to_string()
        } else {
            detail.code.clone()
        };
        let message = detail
            .message
            .clone()
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| {
                format!(
                    "request failed with HTTP {http_status} ({})",
                    if detail.code.is_empty() {
                        "no envelope"
                    } else {
                        &detail.code
                    }
                )
            });
        let retryable = if synthetic {
            TRANSIENT_STATUSES.contains(&http_status)
        } else {
            detail
                .retryable
                .unwrap_or(http_status == 429 || http_status == 503)
        };
        let kind = if code == "idempotency.key_reused" {
            ErrorKind::IdempotencyConflict
        } else {
            match http_status {
                400 => ErrorKind::Validation,
                401 => ErrorKind::Authentication,
                403 => ErrorKind::Permission,
                404 => ErrorKind::NotFound,
                409 => ErrorKind::Conflict,
                429 => ErrorKind::RateLimit,
                503 => ErrorKind::Unavailable,
                s if s >= 500 => ErrorKind::Internal,
                _ => ErrorKind::Api,
            }
        };
        Self {
            kind,
            code,
            message,
            http_status,
            retryable,
            retry_after: detail.retry_after.or(retry_after_header),
            request_id: detail.request_id,
            field: detail.field,
            synthetic,
            raw,
        }
    }
}

/// `[code] message (request_id=…)`; the suffix only when there is an id.
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)?;
        if let Some(id) = &self.request_id {
            write!(f, " (request_id={id})")?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

/// Statuses a response without an envelope may carry transiently (LB/proxy/timeouts).
const TRANSIENT_STATUSES: [u16; 7] = [408, 425, 429, 500, 502, 503, 504];

/// Structured-logger friendly: keeps the message, drops the raw body.
impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OblodaiError")
            .field("kind", &self.kind)
            .field("code", &self.code)
            .field("message", &self.message)
            .field("http_status", &self.http_status)
            .field("retryable", &self.retryable)
            .field("retry_after", &self.retry_after)
            .field("request_id", &self.request_id)
            .field("field", &self.field)
            .field("synthetic", &self.synthetic)
            .finish_non_exhaustive()
    }
}

/// Serializes what a support ticket needs and nothing that could carry a secret.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("OblodaiError", 9)?;
        s.serialize_field("kind", self.kind.as_str())?;
        s.serialize_field("code", &self.code)?;
        s.serialize_field("message", &self.message)?;
        s.serialize_field("http_status", &self.http_status)?;
        s.serialize_field("retryable", &self.retryable)?;
        s.serialize_field("retry_after", &self.retry_after)?;
        s.serialize_field("request_id", &self.request_id)?;
        s.serialize_field("field", &self.field)?;
        s.serialize_field("synthetic", &self.synthetic)?;
        s.end()
    }
}

/// The SDK's result type.
pub type Result<T> = std::result::Result<T, Error>;
