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

    /// Sign with the payout key on a route that accepts either key kind.
    pub fn prefer_payout_key(mut self, prefer: bool) -> Self {
        self.opts.prefer_payout_key = prefer;
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

/// A document the gateway rendered: the bytes plus what they are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileResult {
    pub bytes: Vec<u8>,
    pub content_type: String,
    /// From `Content-Disposition`, when the gateway named the file.
    pub filename: Option<String>,
}

/// A prepared call to a `bare` route: it answers with bytes, not an envelope.
pub struct FileBuilder<Tr> {
    transport: Tr,
    route: &'static RouteSpec,
    opts: CallOptions,
}

impl<Tr> std::fmt::Debug for FileBuilder<Tr> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileBuilder")
            .field("route", &self.route.key)
            .finish_non_exhaustive()
    }
}

impl<Tr> FileBuilder<Tr> {
    pub(crate) fn new(transport: Tr, route: &'static RouteSpec, opts: CallOptions) -> Self {
        Self {
            transport,
            route,
            opts,
        }
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.opts.timeout = Some(timeout);
        self
    }

    pub fn deadline(mut self, deadline: Duration) -> Self {
        self.opts.deadline = Some(deadline);
        self
    }

    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.opts.idempotency_key = Some(key.into());
        self
    }
}

fn file_of(raw: crate::core::engine::RawResponse) -> FileResult {
    FileResult {
        content_type: raw.content_type().to_string(),
        filename: filename_from(raw.header("content-disposition")),
        bytes: raw.body,
    }
}

impl FileBuilder<Transport> {
    /// Fetch the document.
    pub async fn send(self) -> Result<FileResult> {
        Ok(file_of(
            self.transport.execute(self.route, self.opts).await?,
        ))
    }
}

impl IntoFuture for FileBuilder<Transport> {
    type Output = Result<FileResult>;
    type IntoFuture = Pin<Box<dyn Future<Output = Result<FileResult>> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

#[cfg(feature = "blocking")]
impl FileBuilder<crate::core::transport::BlockingTransport> {
    /// Fetch the document.
    pub fn send(self) -> Result<FileResult> {
        Ok(file_of(self.transport.execute(self.route, self.opts)?))
    }
}

/// `filename*=UTF-8''…` wins over `filename="…"`, as RFC 6266 asks.
pub(crate) fn filename_from(disposition: Option<&str>) -> Option<String> {
    let value = disposition?;
    let lower = value.to_ascii_lowercase();
    if let Some(i) = lower.find("filename*=utf-8''") {
        let rest = &value[i + "filename*=utf-8''".len()..];
        let raw = rest.split(';').next()?.trim();
        return Some(
            percent_encoding::percent_decode_str(raw)
                .decode_utf8_lossy()
                .into_owned(),
        );
    }
    let i = lower.find("filename=")?;
    let rest = value[i + "filename=".len()..].split(';').next()?.trim();
    Some(rest.trim_matches('"').to_string())
}

/// Name an invoice or a payout by its `uuid` or by your own `order_id`; one of them is required.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Lookup {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
}

impl Lookup {
    /// By the gateway's own id.
    pub fn uuid(uuid: impl Into<String>) -> Self {
        Self {
            uuid: Some(uuid.into()),
            order_id: None,
        }
    }

    /// By your reference.
    pub fn order_id(order_id: impl Into<String>) -> Self {
        Self {
            uuid: None,
            order_id: Some(order_id.into()),
        }
    }
}

impl From<&str> for Lookup {
    /// A bare string is taken as the `uuid`.
    fn from(uuid: &str) -> Self {
        Lookup::uuid(uuid)
    }
}

impl From<String> for Lookup {
    fn from(uuid: String) -> Self {
        Lookup::uuid(uuid)
    }
}

impl From<&String> for Lookup {
    fn from(uuid: &String) -> Self {
        Lookup::uuid(uuid.clone())
    }
}

/// Name an invoice by `uuid` or `order_id`.
pub type PaymentLookup = Lookup;
/// Name a payout by `uuid` or `order_id`.
pub type PayoutLookup = Lookup;

pub(crate) fn to_value<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
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
