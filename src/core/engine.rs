//! The call lifecycle, with the I/O taken out.
//!
//! [`Core`] owns everything a call needs to decide what to do next — serialize, sign, classify the
//! answer, correct the clock, choose whether and when to retry — and knows nothing about how bytes
//! reach the network. The async client and the `blocking` client are two loops around the very same
//! decisions, and the decisions themselves are testable without a socket.

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::Value;

use super::clock::{Clock, SkewCorrectingClock, SystemClock};
use super::envelope::decode_envelope;
use super::hooks::{Hooks, RequestInfo, ResponseInfo};
use super::idempotency::{assert_idempotency_key, new_idempotency_key};
use super::logger::{LogLevel, Logger, NoopLogger};
use super::request::{
    build_request, serialize_body, BuildInput, BuiltRequest, Credentials, HEADER_REQUEST_ID,
};
use super::retry::{jitter, retry_delay_ms, should_retry, RetryContext, RetryOptions};
use super::route::RouteSpec;
use super::signing::SIGNATURE_SKEW_SECONDS;
use crate::error::{Error, Result};

/// A response as it came off the wire, before any envelope is read.
#[derive(Clone, Debug, Default)]
pub struct RawResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl RawResponse {
    pub fn header(&self, name: &str) -> Option<&str> {
        super::util::header_value(&self.headers, name)
    }

    /// `Content-Type`, or `application/octet-stream` when the answer carried none.
    pub fn content_type(&self) -> &str {
        self.header("content-type")
            .unwrap_or("application/octet-stream")
    }
}

/// A successful (2xx) answer and the `X-Request-ID` of the call that got it.
#[derive(Clone, Debug, Default)]
pub struct Answer {
    pub response: RawResponse,
    /// The response's `X-Request-ID`, else the one the SDK sent with the call.
    pub request_id: String,
}

/// Everything one call is made of: the request itself and the caller's per-call options.
#[derive(Clone, Debug, Default)]
pub struct CallOptions {
    pub body: Option<Value>,
    pub query: Vec<(String, String)>,
    pub path_params: Vec<(&'static str, String)>,
    /// Supply your own key to make the call idempotent across process restarts.
    pub idempotency_key: Option<String>,
    /// Ruling 10: the route takes the key as the body field `idempotency_key`, not as a header.
    pub idempotency_key_in_body: bool,
    /// Extra headers for this call only. They are merged over the client-wide ones; names the SDK
    /// owns (signing, `Accept`, `Content-Type`, `User-Agent`, `X-Admin-Token`) still win.
    pub headers: Vec<(String, String)>,
    /// Per-attempt timeout.
    pub timeout: Option<Duration>,
    /// Overall budget including retries.
    pub deadline: Option<Duration>,
    /// Retries after the first attempt, overriding the client's retry policy for this call.
    pub max_retries: Option<u32>,
    /// `X-Request-ID` of this call (the same on every attempt); generated when absent.
    pub request_id: Option<String>,
}

/// Live state of one logical call, carried across its attempts.
#[derive(Debug)]
pub struct CallState {
    pub route: &'static RouteSpec,
    body: String,
    query: Vec<(String, String)>,
    path_params: Vec<(&'static str, String)>,
    idempotency_key: Option<String>,
    request_id: String,
    headers: Vec<(String, String)>,
    safe_to_repeat: bool,
    retry: RetryOptions,
    attempt: u32,
    skew_tried: bool,
    /// Offset that was applied when the current attempt was signed. Compared against a freshly
    /// measured one, so a concurrent correction by another call is not mistaken for drift.
    signed_offset: i64,
    /// Offset in force before this call corrected the clock, and the one it installed — a revert
    /// only happens when the shared offset is still the installed one.
    skew_before: i64,
    skew_installed: i64,
    deadline_at: Instant,
    pub attempt_timeout: Duration,
    /// The attempt in flight, as the hooks see it (only kept when a hook is installed).
    attempt_info: Option<RequestInfo>,
    sent_at: Instant,
}

impl CallState {
    /// The key sent on every attempt of this call (generated once, never regenerated).
    pub fn idempotency_key(&self) -> Option<&str> {
        self.idempotency_key.as_deref()
    }

    /// The `X-Request-ID` sent on every attempt of this call.
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// How long is left of the overall budget.
    pub fn remaining(&self) -> Duration {
        self.deadline_at.saturating_duration_since(Instant::now())
    }

    /// Timeout for the next attempt: the per-attempt timeout, capped by what the deadline allows.
    pub fn next_timeout(&self) -> Duration {
        self.attempt_timeout
            .min(self.remaining())
            .max(Duration::from_millis(1))
    }

    /// The successful answer of this call.
    pub fn answer(&self, response: RawResponse) -> Answer {
        let request_id = response
            .header(HEADER_REQUEST_ID)
            .filter(|v| !v.is_empty())
            .unwrap_or(&self.request_id)
            .to_string();
        Answer {
            response,
            request_id,
        }
    }
}

/// What the driver should do next.
#[derive(Debug)]
pub enum Step {
    /// The answer is final and successful (2xx).
    Return(RawResponse),
    /// Wait this long, then build and send again.
    Retry(Duration),
    /// Re-sign and send again immediately (clock-skew correction).
    Resign,
}

/// Error codes that mean the core rejected the signature because of the timestamp or MAC.
const SIGNATURE_FAILURE_CODES: [&str; 2] = ["merchant.bad_signature", "auth.bad_timestamp"];

/// Everything shared by every call: credentials, policy, clock, logger, hooks.
///
/// Cloning keeps the clock correction shared (it is an `Arc`), which is what
/// [`Client::with_options`](crate::Client::with_options) relies on.
#[derive(Clone)]
pub struct Core {
    pub base_url: String,
    pub credentials: Option<Credentials>,
    pub timeout: Duration,
    pub deadline: Duration,
    pub retry: RetryOptions,
    pub logger: Arc<dyn Logger>,
    pub headers: Vec<(String, String)>,
    pub admin_token: Option<String>,
    pub user_agent: String,
    pub clock: Arc<SkewCorrectingClock>,
    pub hooks: Hooks,
}

impl std::fmt::Debug for Core {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Core")
            .field("base_url", &self.base_url)
            .field("credentials", &self.credentials)
            .field("timeout", &self.timeout)
            .field("deadline", &self.deadline)
            .field("retry", &self.retry)
            .field("clock", &self.clock)
            .finish_non_exhaustive()
    }
}

impl Core {
    pub fn new(base_url: String, user_agent: String) -> Self {
        Self {
            base_url,
            credentials: None,
            timeout: Duration::from_secs(30),
            deadline: Duration::from_secs(90),
            retry: RetryOptions::default(),
            logger: Arc::new(NoopLogger),
            headers: Vec::new(),
            admin_token: None,
            user_agent,
            clock: Arc::new(SkewCorrectingClock::new(
                Arc::new(SystemClock) as Arc<dyn Clock>
            )),
            hooks: Hooks::default(),
        }
    }

    /// Validate the call, choose the idempotency key and the request id, start the deadline.
    pub fn prepare(&self, route: &'static RouteSpec, opts: CallOptions) -> Result<CallState> {
        let mut body = opts.body;
        let mut key = opts.idempotency_key;
        if opts.idempotency_key_in_body {
            // Ruling 10: the key rides in the body field of the same name, and no header is sent.
            if let Some(k) = key.take() {
                assert_idempotency_key(&k)?;
                match &mut body {
                    Some(Value::Object(map)) => {
                        map.insert("idempotency_key".into(), Value::String(k));
                    }
                    _ => body = Some(serde_json::json!({ "idempotency_key": k })),
                }
            }
        }
        if let Some(b) = &body {
            super::money::reject_float_amounts(b, "")?;
        }
        let body = serialize_body(body.as_ref(), route.method);
        if let Some(k) = &key {
            assert_idempotency_key(k)?;
            if !route.idempotent {
                // The core ignores the header here, so a key would only make the SDK believe a
                // re-send is deduplicated when it is not — the one belief that turns a lost
                // response into a double spend.
                return Err(Error::config(
                    "sdk.idempotency_unsupported",
                    format!(
                        "{} {} does not deduplicate by Idempotency-Key; drop the idempotency key \
                         from this call",
                        route.method, route.path
                    ),
                    Some("idempotency_key"),
                ));
            }
        } else if route.idempotent {
            key = Some(new_idempotency_key());
        }
        // Client-wide headers first, this call's on top: `build_request` keeps the last value for
        // a repeated name, so a per-call header overrides a client-wide one.
        let mut headers = self.headers.clone();
        headers.extend(opts.headers);
        // One id for every attempt: it names the call, not the attempt.
        let request_id = match opts.request_id {
            Some(id) => id,
            None => super::util::header_value(&headers, HEADER_REQUEST_ID)
                .filter(|v| !v.is_empty())
                .map(str::to_string)
                .unwrap_or_else(new_idempotency_key),
        };
        let mut retry = self.retry;
        if let Some(n) = opts.max_retries {
            retry.max_retries = n;
        }
        let safe_to_repeat = route.safe || (route.idempotent && key.is_some());
        Ok(CallState {
            route,
            body,
            query: opts.query,
            path_params: opts.path_params,
            idempotency_key: key,
            request_id,
            headers,
            safe_to_repeat,
            retry,
            attempt: 0,
            skew_tried: false,
            signed_offset: 0,
            skew_before: 0,
            skew_installed: 0,
            deadline_at: Instant::now() + opts.deadline.unwrap_or(self.deadline),
            attempt_timeout: opts.timeout.unwrap_or(self.timeout),
            attempt_info: None,
            sent_at: Instant::now(),
        })
    }

    /// Build and sign the next attempt, and tell the request hook about it.
    pub fn build(&self, st: &mut CallState) -> Result<BuiltRequest> {
        // Read the shared offset once and remember it: what this attempt was signed with is what a
        // measured offset has to be compared against, not whatever another call installed since.
        let signed_offset = self.clock.offset();
        st.signed_offset = signed_offset;
        let req = build_request(BuildInput {
            base_url: &self.base_url,
            route: st.route,
            path_params: &st.path_params,
            query: &st.query,
            body: &st.body,
            credentials: self.credentials.as_ref(),
            idempotency_key: st.idempotency_key.as_deref(),
            ts: self.clock.raw_now() + signed_offset,
            user_agent: &self.user_agent,
            admin_token: self.admin_token.as_deref(),
            extra_headers: &st.headers,
            request_id: &st.request_id,
        })?;
        self.log(
            LogLevel::Debug,
            "request",
            &[
                ("route", st.route.operation_id.to_string()),
                ("attempt", st.attempt.to_string()),
                ("request_id", st.request_id.clone()),
                (
                    "idempotency_key",
                    st.idempotency_key.clone().unwrap_or_default(),
                ),
            ],
        );
        if self.hooks.is_set() {
            let info = RequestInfo::new(&req, st.attempt + 1, &st.request_id, st.route);
            if let Some(hook) = &self.hooks.on_request {
                hook(&info);
            }
            st.attempt_info = Some(info);
        }
        st.sent_at = Instant::now();
        Ok(req)
    }

    fn emit_response(
        &self,
        st: &CallState,
        status: u16,
        headers: &[(String, String)],
        error: Option<&Error>,
    ) {
        let (Some(hook), Some(request)) = (&self.hooks.on_response, &st.attempt_info) else {
            return;
        };
        hook(&ResponseInfo {
            request: request.clone(),
            status,
            headers: headers.to_vec(),
            elapsed: st.sent_at.elapsed(),
            error: error.cloned(),
        });
    }

    /// Decide what to do with an answer.
    pub fn on_response(&self, st: &mut CallState, raw: RawResponse) -> Result<Step> {
        if (200..300).contains(&raw.status) {
            self.emit_response(st, raw.status, &raw.headers, None);
            return Ok(Step::Return(raw));
        }
        let failure = self
            .classify(st.route, &raw)
            .with_request_id(raw.header(HEADER_REQUEST_ID).unwrap_or(&st.request_id));
        self.emit_response(st, raw.status, &raw.headers, Some(&failure));
        self.log(
            LogLevel::Debug,
            "response",
            &[
                ("route", st.route.operation_id.to_string()),
                ("status", raw.status.to_string()),
                ("code", failure.code().to_string()),
                (
                    "request_id",
                    failure.request_id().unwrap_or_default().to_string(),
                ),
            ],
        );

        // Clock skew: the core rejected the timestamp/MAC. Learn its time from the `Date` header,
        // re-sign once, and keep the offset only if that attempt got past authentication.
        if raw.status == 401 && SIGNATURE_FAILURE_CODES.contains(&failure.code()) {
            if !st.skew_tried {
                if let Some(offset) = self.clock.observe_server_date(raw.header("date")) {
                    // Against the offset THIS attempt was signed with: another call may already
                    // have installed the very correction we are about to make.
                    if (offset - st.signed_offset).abs() > SIGNATURE_SKEW_SECONDS / 2 {
                        self.log(
                            LogLevel::Warn,
                            "clock skew detected; re-signing with server time",
                            &[
                                ("route", st.route.operation_id.to_string()),
                                ("offset_sec", offset.to_string()),
                            ],
                        );
                        st.skew_tried = true;
                        st.skew_before = st.signed_offset;
                        st.skew_installed = offset;
                        self.clock.correct(offset);
                        return Ok(Step::Resign);
                    }
                }
            } else {
                // The corrected timestamp did not help: it was not skew. Roll back only if the
                // shared offset is still the one this call installed.
                self.clock.revert(st.skew_installed, st.skew_before);
            }
        }
        self.decide_retry(st, failure)
    }

    /// Decide what to do when no answer arrived at all.
    pub fn on_transport_error(&self, st: &mut CallState, err: Error) -> Result<Step> {
        let err = err.with_request_id(&st.request_id);
        self.emit_response(st, 0, &[], Some(&err));
        self.decide_retry(st, err)
    }

    fn decide_retry(&self, st: &mut CallState, err: Error) -> Result<Step> {
        let ctx = RetryContext {
            attempt: st.attempt,
            safe_to_repeat: st.safe_to_repeat,
        };
        if !should_retry(&err, ctx, &st.retry) {
            return Err(err);
        }
        let delay = Duration::from_millis(retry_delay_ms(&err, ctx, &st.retry, jitter()));
        if delay > st.remaining() {
            return Err(Error::transport(
                "transport.deadline",
                format!("retry would exceed the call deadline; last error: {err}"),
            )
            .with_request_id(&st.request_id));
        }
        st.attempt += 1;
        Ok(Step::Retry(delay))
    }

    /// Log through the configured sink, redacting sensitive-looking values FIRST — a
    /// caller-supplied [`Logger`] never sees a secret, a signature or a passcode.
    fn log(&self, level: LogLevel, message: &str, fields: &[(&str, String)]) {
        let safe: Vec<(&str, String)> = fields
            .iter()
            .map(|(k, v)| (*k, super::logger::redact(k, v).to_string()))
            .collect();
        self.logger.log(level, message, &safe);
    }

    fn classify(&self, route: &RouteSpec, raw: &RawResponse) -> Error {
        match decode_envelope(
            raw.status,
            &raw.body,
            raw.header("retry-after"),
            raw.header("location"),
        ) {
            Err(err) => err,
            Ok(_) => Error::contract(
                format!(
                    "{} {}: HTTP {} with a success envelope",
                    route.method, route.path, raw.status
                ),
                raw.status,
                Some(String::from_utf8_lossy(&raw.body).into_owned()),
            ),
        }
    }
}
