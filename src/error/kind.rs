//! [`ErrorKind`] — the family a failure belongs to, for matching. The value a caller should
//! branch on is always the `code`; the kind is for a `match` on the shape of the failure.

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
