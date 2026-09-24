//! The HTTP engine every resource goes through: serialize → sign → send (with timeout) → decode
//! envelope → classify → retry per policy. `execute` serves the few `bare` routes that return bytes
//! instead of JSON (PDF/CSV documents).

use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::Value;

use super::engine::{Answer, CallOptions, Core, RawResponse, Step};
use super::envelope::{decode_envelope, decode_result};
use super::http::{HttpBackend, HttpRequest, MAX_FILE_BODY_BYTES, MAX_JSON_BODY_BYTES};
use super::route::RouteSpec;
use crate::error::{Error, Result};

/// The async transport. Cheap to clone; share one per key pair.
#[derive(Clone)]
pub struct Transport {
    core: Arc<Core>,
    backend: Arc<dyn HttpBackend>,
}

impl std::fmt::Debug for Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transport")
            .field("core", &self.core)
            .finish_non_exhaustive()
    }
}

impl Transport {
    pub fn new(core: Arc<Core>, backend: Arc<dyn HttpBackend>) -> Self {
        Self { core, backend }
    }

    pub fn core(&self) -> &Core {
        &self.core
    }

    /// A transport over a copy of this one's settings, changed by `change` (same HTTP backend,
    /// same clock correction).
    pub fn with_core(&self, change: impl FnOnce(&mut Core)) -> Self {
        let mut core = (*self.core).clone();
        change(&mut core);
        Self {
            core: Arc::new(core),
            backend: self.backend.clone(),
        }
    }

    /// Run the whole lifecycle and return the 2xx answer with the call's request id.
    pub async fn execute(&self, route: &'static RouteSpec, opts: CallOptions) -> Result<Answer> {
        let mut st = self.core.prepare(route, opts)?;
        loop {
            let req = self.core.build(&mut st)?;
            let outcome = self
                .backend
                .send(HttpRequest {
                    url: req.url,
                    method: req.method,
                    headers: req.headers,
                    body: req.body,
                    timeout: st.next_timeout(),
                    max_body_bytes: body_cap(route),
                })
                .await;
            let step = match outcome {
                Ok(raw) => self.core.on_response(&mut st, raw)?,
                Err(err) => self.core.on_transport_error(&mut st, err)?,
            };
            match step {
                Step::Return(raw) => return Ok(st.answer(raw)),
                Step::Resign => continue,
                Step::Retry(delay) => {
                    tokio::time::sleep(delay).await;
                    continue;
                }
            }
        }
    }

    /// Call an envelope route and return its `result` as raw JSON.
    pub async fn call_value(&self, route: &'static RouteSpec, opts: CallOptions) -> Result<Value> {
        let answer = self.execute(route, opts).await?;
        finish(route, answer.response)
    }

    /// Call an envelope route and decode its `result` into a model.
    pub async fn call<T: DeserializeOwned>(
        &self,
        route: &'static RouteSpec,
        opts: CallOptions,
    ) -> Result<T> {
        decode_result(self.call_value(route, opts).await?, route.operation_id)
    }
}

/// How much of an answer the SDK is willing to hold: documents are large, envelopes are not.
fn body_cap(route: &RouteSpec) -> usize {
    if route.bare {
        MAX_FILE_BODY_BYTES
    } else {
        MAX_JSON_BODY_BYTES
    }
}

/// Read the envelope of a completed 2xx answer.
pub(crate) fn finish(route: &'static RouteSpec, raw: RawResponse) -> Result<Value> {
    let result = decode_envelope(raw.status, &raw.body, None, None)?;
    // The core replays a cached response by Idempotency-Key; when the original was too large to
    // cache it answers {ok, idempotent_replay: true, detail} instead of the object — surface that
    // rather than handing back an object that is not the one the caller asked for.
    if result.get("idempotent_replay").and_then(Value::as_bool) == Some(true) {
        let detail = result
            .get("detail")
            .and_then(Value::as_str)
            .unwrap_or_default();
        return Err(Error::contract(
            format!(
                "{}: the request was already processed but its response was too large to replay \
                 — fetch the result by order_id/reference ({detail})",
                route.operation_id
            ),
            raw.status,
            Some(result.to_string()),
        ));
    }
    Ok(result)
}

/// The synchronous transport, driving the same [`Core`] decisions over a blocking backend.
#[cfg(feature = "blocking")]
#[derive(Clone)]
pub struct BlockingTransport {
    core: Arc<Core>,
    backend: Arc<dyn super::http::BlockingHttpBackend>,
}

#[cfg(feature = "blocking")]
impl std::fmt::Debug for BlockingTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockingTransport")
            .field("core", &self.core)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "blocking")]
impl BlockingTransport {
    pub fn new(core: Arc<Core>, backend: Arc<dyn super::http::BlockingHttpBackend>) -> Self {
        Self { core, backend }
    }

    pub fn core(&self) -> &Core {
        &self.core
    }

    /// A transport over a copy of this one's settings, changed by `change`.
    pub fn with_core(&self, change: impl FnOnce(&mut Core)) -> Self {
        let mut core = (*self.core).clone();
        change(&mut core);
        Self {
            core: Arc::new(core),
            backend: self.backend.clone(),
        }
    }

    /// Run the whole lifecycle and return the 2xx answer with the call's request id.
    pub fn execute(&self, route: &'static RouteSpec, opts: CallOptions) -> Result<Answer> {
        let mut st = self.core.prepare(route, opts)?;
        loop {
            let req = self.core.build(&mut st)?;
            let outcome = self.backend.send(HttpRequest {
                url: req.url,
                method: req.method,
                headers: req.headers,
                body: req.body,
                timeout: st.next_timeout(),
                max_body_bytes: body_cap(route),
            });
            let step = match outcome {
                Ok(raw) => self.core.on_response(&mut st, raw)?,
                Err(err) => self.core.on_transport_error(&mut st, err)?,
            };
            match step {
                Step::Return(raw) => return Ok(st.answer(raw)),
                Step::Resign => continue,
                Step::Retry(delay) => {
                    std::thread::sleep(delay);
                    continue;
                }
            }
        }
    }

    pub fn call_value(&self, route: &'static RouteSpec, opts: CallOptions) -> Result<Value> {
        finish(route, self.execute(route, opts)?.response)
    }

    pub fn call<T: DeserializeOwned>(
        &self,
        route: &'static RouteSpec,
        opts: CallOptions,
    ) -> Result<T> {
        decode_result(self.call_value(route, opts)?, route.operation_id)
    }
}
