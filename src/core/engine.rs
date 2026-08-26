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
use super::idempotency::{assert_idempotency_key, new_idempotency_key};
use super::logger::{LogLevel, Logger, NoopLogger};
use super::request::{build_request, serialize_body, BuildInput, BuiltRequest, Credentials};
use super::retry::{jitter, retry_delay_ms, should_retry, RetryContext, RetryOptions};
use super::signing::SIGNATURE_SKEW_SECONDS;
use crate::contract::types::{RouteAuth, RouteSpec};
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

/// Per-call knobs every resource method accepts.
#[derive(Clone, Debug, Default)]
pub struct CallOptions {
    pub body: Option<Value>,
    pub query: Vec<(String, String)>,
    pub path_params: Vec<(&'static str, String)>,
    /// Supply your own key to make the call idempotent across process restarts.
    pub idempotency_key: Option<String>,
    /// Prefer the payout key pair on a route that accepts either kind.
    pub prefer_payout_key: bool,
    /// Extra headers for this call only. They are merged over the client-wide ones; names the SDK
    /// owns (signing, `Accept`, `Content-Type`, `User-Agent`, `X-Admin-Token`) still win.
    pub headers: Vec<(String, String)>,
    /// Per-attempt timeout.
    pub timeout: Option<Duration>,
    /// Overall budget including retries.
    pub deadline: Option<Duration>,
}

/// Live state of one logical call, carried across its attempts.
#[derive(Debug)]
pub struct CallState {
    pub route: &'static RouteSpec,
    body: String,
    query: Vec<(String, String)>,
    path_params: Vec<(&'static str, String)>,
    idempotency_key: Option<String>,
    prefer_payout_key: bool,
    headers: Vec<(String, String)>,
    safe_to_repeat: bool,
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
}

impl CallState {
    /// The key sent on every attempt of this call (generated once, never regenerated).
    pub fn idempotency_key(&self) -> Option<&str> {
        self.idempotency_key.as_deref()
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

/// Everything shared by every call: credentials, policy, clock, logger.
pub struct Core {
    pub base_url: String,
    pub credentials: Option<Credentials>,
    pub payout_credentials: Option<Credentials>,
    pub timeout: Duration,
    pub deadline: Duration,
    pub retry: RetryOptions,
    pub logger: Arc<dyn Logger>,
    pub headers: Vec<(String, String)>,
    pub admin_token: Option<String>,
    pub user_agent: String,
    pub clock: SkewCorrectingClock,
}

impl std::fmt::Debug for Core {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Core")
            .field("base_url", &self.base_url)
            .field("credentials", &self.credentials)
            .field("payout_credentials", &self.payout_credentials)
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
            payout_credentials: None,
            timeout: Duration::from_secs(30),
            deadline: Duration::from_secs(90),
            retry: RetryOptions::default(),
            logger: Arc::new(NoopLogger),
            headers: Vec::new(),
            admin_token: None,
            user_agent,
            clock: SkewCorrectingClock::new(Arc::new(SystemClock) as Arc<dyn Clock>),
        }
    }

    /// Validate the call, choose the idempotency key and start the deadline.
    pub fn prepare(&self, route: &'static RouteSpec, opts: CallOptions) -> Result<CallState> {
        let body = serialize_body(opts.body.as_ref(), route.method);
        let mut key = opts.idempotency_key;
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
        let safe_to_repeat = route.safe || (route.idempotent && key.is_some());
        Ok(CallState {
            route,
            body,
            query: opts.query,
            path_params: opts.path_params,
            idempotency_key: key,
            prefer_payout_key: opts.prefer_payout_key,
            headers: opts.headers,
            safe_to_repeat,
            attempt: 0,
            skew_tried: false,
            signed_offset: 0,
            skew_before: 0,
            skew_installed: 0,
            deadline_at: Instant::now() + opts.deadline.unwrap_or(self.deadline),
            attempt_timeout: opts.timeout.unwrap_or(self.timeout),
        })
    }

    /// Which key pair signs a route. `any` routes take the payment key unless told otherwise.
    fn credentials_for(&self, route: &RouteSpec, prefer_payout: bool) -> Option<&Credentials> {
        if route.auth == RouteAuth::Payout || (route.auth == RouteAuth::Any && prefer_payout) {
            return self
                .payout_credentials
                .as_ref()
                .or(self.credentials.as_ref());
        }
        self.credentials.as_ref()
    }

    /// Build and sign the next attempt.
    pub fn build(&self, st: &mut CallState) -> Result<BuiltRequest> {
        // Read the shared offset once and remember it: what this attempt was signed with is what a
        // measured offset has to be compared against, not whatever another call installed since.
        let signed_offset = self.clock.offset();
        st.signed_offset = signed_offset;
        // Client-wide headers first, this call's on top: `build_request` keeps the last value for
        // a repeated name, so a per-call header overrides a client-wide one.
        let mut extra = self.headers.clone();
        extra.extend(st.headers.iter().cloned());
        let req = build_request(BuildInput {
            base_url: &self.base_url,
            route: st.route,
            path_params: &st.path_params,
            query: &st.query,
            body: &st.body,
            credentials: self.credentials_for(st.route, st.prefer_payout_key),
            idempotency_key: st.idempotency_key.as_deref(),
            ts: self.clock.raw_now() + signed_offset,
            user_agent: &self.user_agent,
            admin_token: self.admin_token.as_deref(),
            extra_headers: &extra,
        })?;
        self.log(
            LogLevel::Debug,
            "request",
            &[
                ("route", st.route.key.to_string()),
                ("attempt", st.attempt.to_string()),
                (
                    "idempotency_key",
                    st.idempotency_key.clone().unwrap_or_default(),
                ),
            ],
        );
        Ok(req)
    }

    /// Decide what to do with an answer.
    pub fn on_response(&self, st: &mut CallState, raw: RawResponse) -> Result<Step> {
        if (200..300).contains(&raw.status) {
            return Ok(Step::Return(raw));
        }
        let failure = self.classify(st.route, &raw);
        self.log(
            LogLevel::Debug,
            "response",
            &[
                ("route", st.route.key.to_string()),
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
                                ("route", st.route.key.to_string()),
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
        self.decide_retry(st, err)
    }

    fn decide_retry(&self, st: &mut CallState, err: Error) -> Result<Step> {
        let ctx = RetryContext {
            attempt: st.attempt,
            safe_to_repeat: st.safe_to_repeat,
        };
        if !should_retry(&err, ctx, &self.retry) {
            return Err(err);
        }
        let delay = Duration::from_millis(retry_delay_ms(&err, ctx, &self.retry, jitter()));
        if delay > st.remaining() {
            return Err(Error::transport(
                "transport.deadline",
                format!("retry would exceed the call deadline; last error: {err}"),
            ));
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
