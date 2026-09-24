//! Request and response hooks: plain closures the client calls once per attempt.
//!
//! Hooks are for observability — metrics, tracing, structured logs. They run synchronously on the
//! task (or thread) that makes the call, so keep them cheap. A panic in a hook propagates.
//!
//! ```
//! use oblodai::Hooks;
//!
//! let hooks = Hooks::new()
//!     .on_request(|r| println!("-> {} {} #{} {}", r.method, r.url, r.attempt, r.request_id))
//!     .on_response(|r| println!("<- {} in {:?}", r.status, r.elapsed));
//! let client = oblodai::Client::builder()
//!     .public_id("oblodai_…")
//!     .secret("oblodai_live_…")
//!     .hooks(hooks)
//!     .build();
//! # let _ = client;
//! ```

use std::sync::Arc;
use std::time::Duration;

use super::request::BuiltRequest;
use super::route::RouteSpec;
use super::signing::{HEADER_ADMIN_TOKEN, HEADER_SIGNATURE};
use crate::error::Error;

/// A hook on the attempt about to be sent.
pub type RequestHook = Arc<dyn Fn(&RequestInfo) + Send + Sync>;
/// A hook on how an attempt ended.
pub type ResponseHook = Arc<dyn Fn(&ResponseInfo) + Send + Sync>;

/// One attempt about to be sent.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct RequestInfo {
    pub method: &'static str,
    pub url: String,
    /// The headers as sent, with the signature and the admin token replaced by `[redacted]`.
    pub headers: Vec<(String, String)>,
    /// 1 for the first attempt, 2 for the first retry, and so on.
    pub attempt: u32,
    /// `X-Request-ID` of the call; the same on every attempt.
    pub request_id: String,
    /// The route's OpenAPI `operationId`.
    pub operation_id: &'static str,
}

impl RequestInfo {
    pub(crate) fn new(
        req: &BuiltRequest,
        attempt: u32,
        request_id: &str,
        route: &RouteSpec,
    ) -> Self {
        Self {
            method: req.method,
            url: req.url.clone(),
            headers: redact_headers(&req.headers),
            attempt,
            request_id: request_id.to_string(),
            operation_id: route.operation_id,
        }
    }
}

/// How one attempt ended: an HTTP response, or `status == 0` and a transport `error`.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ResponseInfo {
    pub request: RequestInfo,
    /// HTTP status, or 0 when the attempt produced no response (timeout, network error).
    pub status: u16,
    pub headers: Vec<(String, String)>,
    /// From sending the attempt to this point.
    pub elapsed: Duration,
    /// The error this attempt ended with (an error status or a transport failure).
    pub error: Option<Error>,
}

/// `ClientBuilder::hooks(Hooks::new().on_request(..).on_response(..))`; both are optional.
#[derive(Clone, Default)]
pub struct Hooks {
    pub on_request: Option<RequestHook>,
    pub on_response: Option<ResponseHook>,
}

impl std::fmt::Debug for Hooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hooks")
            .field("on_request", &self.on_request.is_some())
            .field("on_response", &self.on_response.is_some())
            .finish()
    }
}

impl Hooks {
    pub fn new() -> Self {
        Self::default()
    }

    /// Called before every attempt is sent.
    pub fn on_request(mut self, hook: impl Fn(&RequestInfo) + Send + Sync + 'static) -> Self {
        self.on_request = Some(Arc::new(hook));
        self
    }

    /// Called when every attempt ends, successful or not.
    pub fn on_response(mut self, hook: impl Fn(&ResponseInfo) + Send + Sync + 'static) -> Self {
        self.on_response = Some(Arc::new(hook));
        self
    }

    pub(crate) fn is_set(&self) -> bool {
        self.on_request.is_some() || self.on_response.is_some()
    }
}

/// A copy with the signature and the admin token replaced by `[redacted]`.
pub fn redact_headers(headers: &[(String, String)]) -> Vec<(String, String)> {
    headers
        .iter()
        .map(|(k, v)| {
            let secret = k.eq_ignore_ascii_case(HEADER_SIGNATURE)
                || k.eq_ignore_ascii_case(HEADER_ADMIN_TOKEN);
            (
                k.clone(),
                if secret {
                    "[redacted]".to_string()
                } else {
                    v.clone()
                },
            )
        })
        .collect()
}
