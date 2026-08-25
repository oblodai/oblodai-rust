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

/// Which family a failure belongs to. Mirrors the reference SDK's error subclasses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorKind {
    /// 400 — the request is malformed or violates a business rule; see `field` and `code`.
    Validation,
    /// 401 — bad signature, unknown key, clock skew, IP not in the allow-list.
    Authentication,
    /// 403 — the key is valid but not allowed to do this (wrong key kind, feature disabled).
    Permission,
    /// 404 — the referenced object does not exist for this merchant.
    NotFound,
    /// 409 — state conflict.
    Conflict,
    /// 409 `idempotency.key_reused` — the same key was used with a different request body.
    IdempotencyConflict,
    /// 429 — rate limited; `retry_after` is set.
    RateLimit,
    /// 503 — an upstream dependency is down; safe to retry after a pause.
    Unavailable,
    /// 5xx other than 503.
    Internal,
    /// Any other status the API answered with.
    Api,
    /// The request never produced an HTTP response: DNS, TCP, TLS, timeout, abort, deadline.
    Transport,
    /// Raised before any request is sent: bad options, missing credentials, unusable arguments.
    Config,
    /// The response could not be interpreted as the documented envelope.
    Contract,
    /// Webhook verification failed (bad signature, stale timestamp, missing headers).
    Signature,
}

impl ErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorKind::Validation => "validation",
            ErrorKind::Authentication => "authentication",
            ErrorKind::Permission => "permission",
            ErrorKind::NotFound => "not_found",
            ErrorKind::Conflict => "conflict",
            ErrorKind::IdempotencyConflict => "idempotency_conflict",
            ErrorKind::RateLimit => "rate_limit",
            ErrorKind::Unavailable => "unavailable",
            ErrorKind::Internal => "internal",
            ErrorKind::Api => "api",
            ErrorKind::Transport => "transport",
            ErrorKind::Config => "config",
            ErrorKind::Contract => "contract",
            ErrorKind::Signature => "signature",
        }
    }
}

/// The error envelope as the core writes it.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
pub struct ErrorDetail {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub field: Option<String>,
    #[serde(default)]
    pub retryable: Option<bool>,
    #[serde(default)]
    pub retry_after: Option<u64>,
    #[serde(default)]
    pub request_id: Option<String>,
}

/// Every failure the SDK reports.
#[derive(Clone, thiserror::Error)]
#[error("{message}")]
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
