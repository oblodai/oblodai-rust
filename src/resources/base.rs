//! What every resource method is built from: a lazy request builder, the per-call knobs, and the
//! two reference shapes the API uses to name an object (a bare id, or `{uuid}` / `{order_id}`).

use std::future::{Future, IntoFuture};
use std::marker::PhantomData;
use std::pin::Pin;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::contract::types::RouteSpec;
use crate::core::engine::CallOptions;
use crate::core::envelope::PlainList;
use crate::core::transport::Transport;
use crate::error::Result;

/// A prepared call. Nothing is sent until it is awaited (or, on the blocking client, sent).
///
/// ```no_run
/// # async fn demo(client: &oblodai::Client) -> oblodai::Result<()> {
/// # let params = Default::default();
/// let invoice = client.payments().create(params).idempotency_key("order-1001").await?;
/// # Ok(()) }
/// ```
///
/// # Cancellation
///
/// Dropping the future cancels the call — and the auto-generated idempotency key lives in that
/// future, so it dies with it. A request already on the wire may still reach the gateway, and a
/// re-issued call would mint a *new* key that the gateway cannot deduplicate against it. Bound a
/// call with [`deadline`](Self::deadline) (which fails with `transport.deadline` while keeping the
/// key for the attempts it did make) rather than with `tokio::time::timeout` or `select!`; if a
/// retry has to survive a cancel or a process restart, supply your own
/// [`idempotency_key`](Self::idempotency_key).
#[must_use = "nothing is sent until this builder is awaited (or `.send()` is called)"]
pub struct RequestBuilder<Tr, T> {
    transport: Tr,
    route: &'static RouteSpec,
    opts: CallOptions,
    _out: PhantomData<fn() -> T>,
}

impl<Tr, T> std::fmt::Debug for RequestBuilder<Tr, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RequestBuilder")
            .field("route", &self.route.key)
            .finish_non_exhaustive()
    }
}

impl<Tr, T> RequestBuilder<Tr, T> {
    pub(crate) fn new(transport: Tr, route: &'static RouteSpec, opts: CallOptions) -> Self {
        Self {
            transport,
            route,
            opts,
            _out: PhantomData,
        }
    }

    /// Your own idempotency key, so a retry after a process restart is still deduplicated.
    /// Rejected with `sdk.idempotency_unsupported` on routes the core does not deduplicate.
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.opts.idempotency_key = Some(key.into());
        self
    }

    /// Per-attempt timeout. Default 30 s.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.opts.timeout = Some(timeout);
        self
    }

    /// Overall budget for the call, retries and pauses included. Default 90 s.
    pub fn deadline(mut self, deadline: Duration) -> Self {
        self.opts.deadline = Some(deadline);
        self
    }

    /// An extra header on this call only, merged over the client-wide ones. Names the SDK owns
    /// (the signing headers, `Accept`, `Content-Type`, `User-Agent`, `X-Admin-Token`) are never
    /// overridden; a CR/LF or non-ASCII value is a `sdk.bad_header` config error.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.opts.headers.push((name.into(), value.into()));
        self
    }
}

impl<T: DeserializeOwned + Send + 'static> RequestBuilder<Transport, T> {
    /// Send and decode.
    pub async fn send(self) -> Result<T> {
        self.transport.call::<T>(self.route, self.opts).await
    }

    /// Send and return the `result` payload as raw JSON — an escape hatch for a field this
    /// snapshot's models do not carry yet.
    pub async fn send_raw(self) -> Result<Value> {
        self.transport.call_value(self.route, self.opts).await
    }
}

impl<T: DeserializeOwned + Send + 'static> RequestBuilder<Transport, PlainList<T>> {
    /// The items of a plain (uncounted) list.
    pub async fn items(self) -> Result<Vec<T>> {
        Ok(self.send().await?.items)
    }
}

impl<T: DeserializeOwned + Send + 'static> IntoFuture for RequestBuilder<Transport, T> {
    type Output = Result<T>;
    type IntoFuture = Pin<Box<dyn Future<Output = Result<T>> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

#[cfg(feature = "blocking")]
impl<T: DeserializeOwned> RequestBuilder<crate::core::transport::BlockingTransport, T> {
    /// Send and decode.
    pub fn send(self) -> Result<T> {
        self.transport.call::<T>(self.route, self.opts)
    }

    /// Send and return the `result` payload as raw JSON.
    pub fn send_raw(self) -> Result<Value> {
        self.transport.call_value(self.route, self.opts)
    }
}

#[cfg(feature = "blocking")]
impl<T: DeserializeOwned> RequestBuilder<crate::core::transport::BlockingTransport, PlainList<T>> {
    /// The items of a plain (uncounted) list.
    pub fn items(self) -> Result<Vec<T>> {
        Ok(self.send()?.items)
    }
}

/// Serialize a request struct into the body value.
///
/// Every request type is a generated struct of strings, integers, floats, bools, vectors and
/// `serde_json::Value`, none of which can fail to serialize (a non-finite `f64` becomes `null`,
/// it does not error), so the fallback is unreachable. It is a `Null` rather than a panic because
/// an SDK that forbids unsafe and returns `Result` everywhere should not abort a payout call over
/// a serializer edge case; `serialize_body` then sends `{}` and the gateway refuses it with a
/// validation error naming the missing field.
pub(crate) fn to_value<T: serde::Serialize>(value: &T) -> Value {
    match serde_json::to_value(value) {
        Ok(value) => value,
        Err(_) => {
            debug_assert!(false, "a generated request body failed to serialize");
            Value::Null
        }
    }
}

/// Assembles the [`CallOptions`] of one resource method. Internal sugar so every method reads as
/// "this route, this body, these path parameters" and nothing else.
#[derive(Default)]
pub(crate) struct Call(CallOptions);

impl Call {
    pub(crate) fn new() -> Self {
        Self(CallOptions::default())
    }

    /// A typed request body.
    pub(crate) fn body<T: serde::Serialize>(mut self, body: &T) -> Self {
        self.0.body = Some(to_value(body));
        self
    }

    /// A body assembled on the spot (`{"uuid": …}` and friends).
    pub(crate) fn json(mut self, body: Value) -> Self {
        self.0.body = Some(body);
        self
    }

    pub(crate) fn path(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.0.path_params.push((name, value.into()));
        self
    }

    pub(crate) fn query(mut self, name: &str, value: impl Into<String>) -> Self {
        self.0.query.push((name.to_string(), value.into()));
        self
    }

    /// Flatten a typed query struct into `?a=1&b=2`, skipping absent fields.
    pub(crate) fn query_of<T: serde::Serialize>(mut self, value: &T) -> Self {
        if let Value::Object(map) = to_value(value) {
            for (k, v) in map {
                match v {
                    Value::Null => {}
                    Value::String(s) => self.0.query.push((k, s)),
                    other => self.0.query.push((k, other.to_string())),
                }
            }
        }
        self
    }

    pub(crate) fn done(self) -> CallOptions {
        self.0
    }
}
