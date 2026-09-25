//! What every generated resource method is built from and returns: the call it describes
//! ([`Call`]), a lazy request builder ([`Request`]), a lazy walk over a paged list ([`Pager`]),
//! the file of a document route ([`FileResult`]) and the raw side of an answer
//! ([`RawApiResponse`]).
//!
//! A method never sends anything by itself: set the call options on the builder, then `.await`
//! it (or `.send()` it on the blocking client).
//!
//! ```no_run
//! # async fn demo(client: &oblodai::Client) -> oblodai::Result<()> {
//! use std::time::Duration;
//! use oblodai::models::PaymentRequest;
//!
//! let invoice = client
//!     .payments()
//!     .create(PaymentRequest::new("25", "USDT"))
//!     .idempotency_key("order-1001")
//!     .timeout(Duration::from_secs(10))
//!     .max_retries(3)
//!     .extra_header("X-Tenant", "eu")
//!     .request_id("req-order-1001")
//!     .await?;
//! # let _ = invoice;
//! # Ok(()) }
//! ```

use std::collections::VecDeque;
use std::future::{Future, IntoFuture};
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::core::engine::{Answer, CallOptions};
use crate::core::envelope::{decode_result, Page};
use crate::core::money::FLOAT_AMOUNT;
use crate::core::route::{Method, RouteSpec};
use crate::core::signing::HEADER_IDEMPOTENCY_KEY;
use crate::core::transport::{finish, Transport};
use crate::error::{Error, Result};

/// Page size used when the caller does not choose one.
pub const DEFAULT_PAGE_LIMIT: i64 = 50;

/// Serialize a request value; the generated models (strings, integers, `Money`, maps, vectors,
/// `serde_json::Value`) cannot fail to serialize, so the fallback is unreachable. It is a `Null`
/// rather than a panic: an SDK that forbids unsafe and returns `Result` everywhere should not abort
/// a payout call over a serializer edge case; the gateway then refuses `{}` with a validation error
/// naming the missing field.
pub(crate) fn to_value<T: serde::Serialize>(value: &T) -> Value {
    match serde_json::to_value(value) {
        Ok(value) => value,
        Err(_) => {
            debug_assert!(false, "a request value failed to serialize");
            Value::Null
        }
    }
}

/// Build a typed request (a generated model or query struct) from JSON — for parameters that
/// arrive as JSON anyway (a queue message, a config file).
///
/// A floating-point amount is refused before anything is sent, with code `sdk.float_amount`;
/// any other mismatch with the model is `sdk.bad_params`. Both are
/// [`ErrorKind::Config`](crate::ErrorKind::Config) errors.
///
/// ```
/// use oblodai::models::PaymentRequest;
/// use serde_json::json;
///
/// let ok: PaymentRequest = oblodai::from_json(json!({"amount": "25.10", "currency": "USDT"})).unwrap();
/// assert_eq!(ok.amount.as_str(), "25.10");
/// let err = oblodai::from_json::<PaymentRequest>(json!({"amount": 25.1, "currency": "USDT"}))
///     .unwrap_err();
/// assert_eq!(err.code(), "sdk.float_amount");
/// ```
pub fn from_json<T: DeserializeOwned>(value: Value) -> Result<T> {
    serde_json::from_value(value).map_err(|e| {
        let message = e.to_string();
        if message.contains(FLOAT_AMOUNT) {
            Error::config(FLOAT_AMOUNT, message, Some("amount"))
        } else {
            Error::config(
                "sdk.bad_params",
                format!("parameters do not match the model: {message}"),
                None,
            )
        }
    })
}

/// One call as a generated method describes it: the route, the body, path and query parameters.
#[derive(Clone, Debug)]
pub struct Call {
    route: &'static RouteSpec,
    body: Option<Value>,
    path_params: Vec<(&'static str, String)>,
    query: Vec<(String, String)>,
    idempotency_key_in_body: bool,
}

impl Call {
    pub(crate) fn new(route: &'static RouteSpec) -> Self {
        Self {
            route,
            body: None,
            path_params: Vec::new(),
            query: Vec::new(),
            idempotency_key_in_body: false,
        }
    }

    /// The route this call goes to.
    pub fn route(&self) -> &'static RouteSpec {
        self.route
    }

    /// A typed request body.
    pub(crate) fn body<T: serde::Serialize>(&mut self, body: T) {
        self.body = Some(to_value(&body));
    }

    pub(crate) fn path(&mut self, name: &'static str, value: impl Into<String>) {
        self.path_params.push((name, value.into()));
    }

    /// Flatten a typed query struct into `?a=1&b=2`, skipping absent fields.
    pub(crate) fn query<T: serde::Serialize>(&mut self, query: T) {
        if let Value::Object(map) = to_value(&query) {
            for (k, v) in map {
                match v {
                    Value::Null => {}
                    Value::String(s) => self.query.push((k, s)),
                    other => self.query.push((k, other.to_string())),
                }
            }
        }
    }

    /// Ruling 10: the call option `idempotency_key` fills the body field of that name.
    pub(crate) fn idempotency_key_in_body(&mut self) {
        self.idempotency_key_in_body = true;
    }
}

/// The per-call options every builder accepts.
#[derive(Clone, Debug, Default)]
struct Options {
    idempotency_key: Option<String>,
    timeout: Option<Duration>,
    deadline: Option<Duration>,
    max_retries: Option<u32>,
    headers: Vec<(String, String)>,
    request_id: Option<String>,
}

impl Options {
    fn apply(&self, call: &Call) -> CallOptions {
        CallOptions {
            body: call.body.clone(),
            query: call.query.clone(),
            path_params: call.path_params.clone(),
            idempotency_key: self.idempotency_key.clone(),
            idempotency_key_in_body: call.idempotency_key_in_body,
            headers: self.headers.clone(),
            timeout: self.timeout,
            deadline: self.deadline,
            max_retries: self.max_retries,
            request_id: self.request_id.clone(),
        }
    }
}

/// The call-option setters, shared by [`Request`] and [`Pager`].
macro_rules! call_options {
    () => {
        /// Your own idempotency key, so a retry after a process restart is still deduplicated.
        /// Rejected with `sdk.idempotency_unsupported` on routes the gateway does not deduplicate.
        pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
            self.opts.idempotency_key = Some(key.into());
            self
        }

        /// Per-attempt timeout. Default 30 s (or the client's).
        pub fn timeout(mut self, timeout: Duration) -> Self {
            self.opts.timeout = Some(timeout);
            self
        }

        /// Overall budget for the call, retries and pauses included. Default 90 s.
        pub fn deadline(mut self, deadline: Duration) -> Self {
            self.opts.deadline = Some(deadline);
            self
        }

        /// Retries after the first attempt for this call; `0` sends it once.
        pub fn max_retries(mut self, max_retries: u32) -> Self {
            self.opts.max_retries = Some(max_retries);
            self
        }

        /// An extra header on this call only, merged over the client-wide ones. Names the SDK
        /// owns (the signing headers, `Accept`, `Content-Type`, `User-Agent`, `X-Admin-Token`,
        /// `X-Request-ID`) are never overridden; a CR/LF or non-ASCII value is a `sdk.bad_header`
        /// config error.
        pub fn extra_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
            self.opts.headers.push((name.into(), value.into()));
            self
        }

        /// `X-Request-ID` of this call (the same on every attempt), to find it in the gateway's
        /// logs. A fresh UUID when not set.
        pub fn request_id(mut self, request_id: impl Into<String>) -> Self {
            self.opts.request_id = Some(request_id.into());
            self
        }
    };
}

/// How the answer of a route becomes the value a method returns: the envelope's `result`
/// decoded into a model, or — for a document route — the file.
pub trait Decode: Sized {
    #[doc(hidden)]
    fn decode(route: &'static RouteSpec, answer: Answer) -> Result<Self>;
}

impl<T: DeserializeOwned> Decode for T {
    fn decode(route: &'static RouteSpec, answer: Answer) -> Result<Self> {
        decode_result(finish(route, answer.response)?, route.operation_id)
    }
}

/// A document the gateway rendered (PDF/CSV): the bytes plus what they are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileResult {
    pub bytes: Vec<u8>,
    pub content_type: String,
    /// From `Content-Disposition`, when the gateway named the file.
    pub filename: Option<String>,
}

impl FileResult {
    /// Save the document.
    pub fn write_to(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::write(path, &self.bytes)
    }
}

impl Decode for FileResult {
    fn decode(_route: &'static RouteSpec, answer: Answer) -> Result<Self> {
        let raw = answer.response;
        Ok(FileResult {
            content_type: raw.content_type().to_string(),
            filename: filename_from(raw.header("content-disposition")),
            bytes: raw.body,
        })
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

/// The raw side of a successful call: status, headers and request id, and
/// [`parse`](Self::parse) for the value the method returns otherwise. An error status is still an
/// `Err`, exactly as without [`Request::with_raw_response`].
#[derive(Clone)]
pub struct RawApiResponse<T> {
    route: &'static RouteSpec,
    answer: Answer,
    _out: PhantomData<fn() -> T>,
}

impl<T> std::fmt::Debug for RawApiResponse<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawApiResponse")
            .field("status", &self.status())
            .field("request_id", &self.request_id())
            .field("route", &self.route.operation_id)
            .finish_non_exhaustive()
    }
}

impl<T> RawApiResponse<T> {
    pub(crate) fn new(route: &'static RouteSpec, answer: Answer) -> Self {
        Self {
            route,
            answer,
            _out: PhantomData,
        }
    }

    pub fn status(&self) -> u16 {
        self.answer.response.status
    }

    pub fn headers(&self) -> &[(String, String)] {
        &self.answer.response.headers
    }

    /// One header, looked up case-insensitively.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.answer.response.header(name)
    }

    /// The response's `X-Request-ID`, else the one this SDK sent with the call.
    pub fn request_id(&self) -> &str {
        &self.answer.request_id
    }

    /// The body as it came off the wire.
    pub fn body(&self) -> &[u8] {
        &self.answer.response.body
    }
}

impl<T: Decode> RawApiResponse<T> {
    /// The value the method returns without `with_raw_response`.
    pub fn parse(&self) -> Result<T> {
        T::decode(self.route, self.answer.clone())
    }
}

/// A prepared call. Nothing is sent until it is awaited (or, on the blocking client, sent).
///
/// # Cancellation
///
/// Dropping the future cancels the call — and the auto-generated idempotency key lives in that
/// future, so it dies with it. Bound a call with [`deadline`](Self::deadline) rather than with
/// `tokio::time::timeout` or `select!`; if a retry has to survive a cancel or a process restart,
/// supply your own [`idempotency_key`](Self::idempotency_key).
#[must_use = "nothing is sent until this builder is awaited (or `.send()` is called)"]
pub struct Request<Tr, T> {
    transport: Tr,
    call: Call,
    opts: Options,
    _out: PhantomData<fn() -> T>,
}

impl<Tr, T> std::fmt::Debug for Request<Tr, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Request")
            .field("route", &self.call.route.operation_id)
            .finish_non_exhaustive()
    }
}

impl<Tr, T> Request<Tr, T> {
    pub(crate) fn new(transport: Tr, call: Call) -> Self {
        Self {
            transport,
            call,
            opts: Options::default(),
            _out: PhantomData,
        }
    }

    call_options!();

    /// The route this call goes to.
    pub fn route(&self) -> &'static RouteSpec {
        self.call.route
    }

    /// The same call, answering with a [`RawApiResponse`] (status, headers, request id,
    /// `parse()`) instead of the parsed value.
    pub fn with_raw_response(self) -> WithRawResponse<Tr, T> {
        WithRawResponse(self)
    }

    pub(crate) fn parts(self) -> (Tr, &'static RouteSpec, CallOptions) {
        let opts = self.opts.apply(&self.call);
        (self.transport, self.call.route, opts)
    }
}

impl<T: Decode> Request<Transport, T> {
    /// Send and decode.
    pub async fn send(self) -> Result<T> {
        let (transport, route, opts) = self.parts();
        T::decode(route, transport.execute(route, opts).await?)
    }

    /// Send and return the `result` payload as raw JSON — an escape hatch for a field this SDK
    /// version's models do not carry yet.
    pub async fn send_json(self) -> Result<Value> {
        let (transport, route, opts) = self.parts();
        transport.call_value(route, opts).await
    }
}

impl<T: Decode + Send + 'static> IntoFuture for Request<Transport, T> {
    type Output = Result<T>;
    type IntoFuture = Pin<Box<dyn Future<Output = Result<T>> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

#[cfg(feature = "blocking")]
impl<T: Decode> Request<crate::core::transport::BlockingTransport, T> {
    /// Send and decode.
    pub fn send(self) -> Result<T> {
        let (transport, route, opts) = self.parts();
        T::decode(route, transport.execute(route, opts)?)
    }

    /// Send and return the `result` payload as raw JSON.
    pub fn send_json(self) -> Result<Value> {
        let (transport, route, opts) = self.parts();
        transport.call_value(route, opts)
    }
}

/// A [`Request`] that answers with a [`RawApiResponse`].
#[must_use = "nothing is sent until this builder is awaited (or `.send()` is called)"]
#[derive(Debug)]
pub struct WithRawResponse<Tr, T>(Request<Tr, T>);

impl<T: Decode> WithRawResponse<Transport, T> {
    /// Send; the answer is not parsed until [`RawApiResponse::parse`].
    pub async fn send(self) -> Result<RawApiResponse<T>> {
        let (transport, route, opts) = self.0.parts();
        let answer = transport.execute(route, opts).await?;
        Ok(RawApiResponse::new(route, answer))
    }
}

impl<T: Decode + Send + 'static> IntoFuture for WithRawResponse<Transport, T> {
    type Output = Result<RawApiResponse<T>>;
    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

#[cfg(feature = "blocking")]
impl<T: Decode> WithRawResponse<crate::core::transport::BlockingTransport, T> {
    /// Send; the answer is not parsed until [`RawApiResponse::parse`].
    pub fn send(self) -> Result<RawApiResponse<T>> {
        let (transport, route, opts) = self.0.parts();
        let answer = transport.execute(route, opts)?;
        Ok(RawApiResponse::new(route, answer))
    }
}

/// A lazy walk over a paged list route (`{items, paginate}`).
///
/// Nothing is requested until it is awaited (the first page), streamed (every item), walked page
/// by page ([`by_page`](Self::by_page)) or collected ([`all`](Self::all)). `paginate.has_pages` is
/// the server's own "there is more" flag, and a walk stops on it (or on an empty page); a *short*
/// page does not end it.
///
/// ```no_run
/// # async fn demo(client: &oblodai::Client) -> oblodai::Result<()> {
/// use futures_util::StreamExt;
/// use oblodai::models::HistoryRequest;
///
/// let first = client.payments().list_history(HistoryRequest::default()).limit(20).await?;
/// let mut items = client.payments().list_history(HistoryRequest::default()).stream();
/// while let Some(payment) = items.next().await {
///     println!("{}", payment?.uuid);
/// }
/// # let _ = first;
/// # Ok(()) }
/// ```
#[must_use = "a pager requests nothing until it is awaited, streamed or collected"]
pub struct Pager<Tr, T> {
    transport: Tr,
    call: Call,
    opts: Options,
    limit: i64,
    offset: i64,
    finished: bool,
    _item: PhantomData<fn() -> T>,
}

impl<Tr, T> std::fmt::Debug for Pager<Tr, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pager")
            .field("route", &self.call.route.operation_id)
            .field("limit", &self.limit)
            .field("offset", &self.offset)
            .finish_non_exhaustive()
    }
}

fn as_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

impl<Tr, T> Pager<Tr, T> {
    pub(crate) fn new(transport: Tr, mut call: Call) -> Self {
        // Pull `limit`/`offset` out of the request so they cannot be sent twice.
        let (mut limit, mut offset) = (None, None);
        if let Some(Value::Object(map)) = &mut call.body {
            limit = map.remove("limit").as_ref().and_then(as_i64);
            offset = map.remove("offset").as_ref().and_then(as_i64);
        }
        call.query.retain(|(k, v)| match k.as_str() {
            "limit" => {
                limit = v.parse().ok().or(limit);
                false
            }
            "offset" => {
                offset = v.parse().ok().or(offset);
                false
            }
            _ => true,
        });
        Self {
            transport,
            call,
            opts: Options::default(),
            limit: limit.unwrap_or(DEFAULT_PAGE_LIMIT),
            offset: offset.unwrap_or(0),
            finished: false,
            _item: PhantomData,
        }
    }

    call_options!();

    /// Page size. Default 50.
    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = limit;
        self
    }

    /// Where to start. Default 0.
    pub fn offset(mut self, offset: i64) -> Self {
        self.offset = offset;
        self
    }

    /// The route this walk goes to.
    pub fn route(&self) -> &'static RouteSpec {
        self.call.route
    }

    /// The request of one page. A key per page would be wrong on both sides — the gateway
    /// would replay page 1 forever — so a caller's key is refused on a list.
    fn page_options(&self, limit: i64, offset: i64) -> Result<CallOptions> {
        let route = self.call.route;
        if self.opts.idempotency_key.is_some() && !route.idempotent {
            return Err(Error::config(
                "sdk.idempotency_unsupported",
                format!(
                    "{} {} does not deduplicate by {HEADER_IDEMPOTENCY_KEY}; drop the \
                     idempotency key from this call",
                    route.method, route.path
                ),
                Some("idempotency_key"),
            ));
        }
        let mut opts = self.opts.apply(&self.call);
        opts.idempotency_key = None;
        if route.method == Method::Get {
            opts.query.push(("limit".into(), limit.to_string()));
            opts.query.push(("offset".into(), offset.to_string()));
        } else {
            let mut body = match opts.body.take() {
                Some(Value::Object(map)) => map,
                _ => Map::new(),
            };
            body.insert("limit".into(), Value::from(limit));
            body.insert("offset".into(), Value::from(offset));
            opts.body = Some(Value::Object(body));
        }
        Ok(opts)
    }

    /// Note the page just fetched and say whether the walk is over.
    fn advance(&mut self, page: &Page<T>) {
        let got = page.items.len() as i64;
        self.offset += got;
        if got == 0 || !page.paginate.has_pages {
            self.finished = true;
        }
    }

    /// The first page, answering with a [`RawApiResponse`] of it.
    pub fn with_raw_response(self) -> WithRawResponse<Tr, Page<T>> {
        let mut call = self.call;
        if call.route.method == Method::Get {
            call.query.push(("limit".into(), self.limit.to_string()));
            call.query.push(("offset".into(), self.offset.to_string()));
        } else {
            let mut body = match call.body.take() {
                Some(Value::Object(map)) => map,
                _ => Map::new(),
            };
            body.insert("limit".into(), Value::from(self.limit));
            body.insert("offset".into(), Value::from(self.offset));
            call.body = Some(Value::Object(body));
        }
        // A caller's idempotency key stays in the options: the engine refuses it on a list route
        // when the call is sent, like every other option mistake.
        WithRawResponse(Request {
            transport: self.transport,
            call,
            opts: self.opts,
            _out: PhantomData,
        })
    }
}

impl<T: DeserializeOwned> Pager<Transport, T> {
    /// Fetch exactly one page at the configured offset, leaving the pager where it was.
    pub async fn page(&self) -> Result<Page<T>> {
        let route = self.call.route;
        let value = self
            .transport
            .call_value(route, self.page_options(self.limit, self.offset)?)
            .await?;
        decode_result(value, route.operation_id)
    }

    /// Fetch the next page and advance; `None` once the list is exhausted.
    pub async fn next_page(&mut self) -> Result<Option<Page<T>>> {
        if self.finished {
            return Ok(None);
        }
        let page: Page<T> = self.page().await?;
        self.advance(&page);
        Ok(Some(page))
    }

    /// Collect every item across pages, optionally capped.
    pub async fn all(mut self, max_items: Option<usize>) -> Result<Vec<T>> {
        let mut out = Vec::new();
        while let Some(page) = self.next_page().await? {
            for item in page.items {
                if max_items.is_some_and(|m| out.len() >= m) {
                    return Ok(out);
                }
                out.push(item);
            }
            if max_items.is_some_and(|m| out.len() >= m) {
                return Ok(out);
            }
        }
        Ok(out)
    }
}

impl<T: DeserializeOwned + Send + 'static> Pager<Transport, T> {
    /// Every item across every page, as a [`futures_core::Stream`]. Each poll fetches at most one
    /// page, and the first page is not requested until the stream is polled.
    pub fn stream(self) -> ItemStream<T> {
        ItemStream {
            pages: self.by_page(),
            buffer: VecDeque::new(),
        }
    }

    /// Every page, one request per page, as a [`futures_core::Stream`] of [`Page`]s.
    pub fn by_page(self) -> PageStream<T> {
        PageStream {
            pager: Some(self),
            pending: None,
        }
    }
}

/// `await` a pager for its first page — the common case when you only need one.
impl<T: DeserializeOwned + Send + 'static> IntoFuture for Pager<Transport, T> {
    type Output = Result<Page<T>>;
    type IntoFuture = Pin<Box<dyn Future<Output = Result<Page<T>>> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.page().await })
    }
}

type PageFuture<T> =
    Pin<Box<dyn Future<Output = (Pager<Transport, T>, Result<Option<Page<T>>>)> + Send>>;

/// A [`Stream`](futures_core::Stream) of the pages of a list.
#[must_use = "a stream fetches nothing until it is polled"]
pub struct PageStream<T> {
    pager: Option<Pager<Transport, T>>,
    pending: Option<PageFuture<T>>,
}

impl<T> std::fmt::Debug for PageStream<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PageStream").finish_non_exhaustive()
    }
}

impl<T: DeserializeOwned + Send + 'static> futures_core::Stream for PageStream<T> {
    type Item = Result<Page<T>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.pending.is_none() {
            let Some(mut pager) = this.pager.take() else {
                return Poll::Ready(None);
            };
            this.pending = Some(Box::pin(async move {
                let page = pager.next_page().await;
                (pager, page)
            }));
        }
        let fut = this.pending.as_mut().expect("just set");
        match fut.as_mut().poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready((pager, result)) => {
                this.pending = None;
                match result {
                    Err(err) => Poll::Ready(Some(Err(err))),
                    Ok(None) => Poll::Ready(None),
                    Ok(Some(page)) => {
                        this.pager = Some(pager);
                        Poll::Ready(Some(Ok(page)))
                    }
                }
            }
        }
    }
}

/// A [`Stream`](futures_core::Stream) of items across pages.
#[must_use = "a stream fetches nothing until it is polled"]
pub struct ItemStream<T> {
    pages: PageStream<T>,
    buffer: VecDeque<T>,
}

impl<T> std::fmt::Debug for ItemStream<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ItemStream")
            .field("buffered", &self.buffer.len())
            .finish_non_exhaustive()
    }
}

impl<T: DeserializeOwned + Send + Unpin + 'static> futures_core::Stream for ItemStream<T> {
    type Item = Result<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            if let Some(item) = this.buffer.pop_front() {
                return Poll::Ready(Some(Ok(item)));
            }
            match Pin::new(&mut this.pages).poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Ready(Some(Err(err))) => {
                    this.pages.pager = None;
                    return Poll::Ready(Some(Err(err)));
                }
                Poll::Ready(Some(Ok(page))) => this.buffer.extend(page.items),
            }
        }
    }
}

#[cfg(feature = "blocking")]
mod blocking_pager {
    use super::*;
    use crate::core::transport::BlockingTransport;

    impl<T: DeserializeOwned> Pager<BlockingTransport, T> {
        /// Fetch exactly one page at the configured offset, leaving the pager where it was.
        pub fn page(&self) -> Result<Page<T>> {
            let route = self.call.route;
            let value = self
                .transport
                .call_value(route, self.page_options(self.limit, self.offset)?)?;
            decode_result(value, route.operation_id)
        }

        /// Fetch the next page and advance; `None` once the list is exhausted.
        pub fn next_page(&mut self) -> Result<Option<Page<T>>> {
            if self.finished {
                return Ok(None);
            }
            let page: Page<T> = self.page()?;
            self.advance(&page);
            Ok(Some(page))
        }

        /// Collect every item across pages, optionally capped.
        pub fn all(self, max_items: Option<usize>) -> Result<Vec<T>> {
            let mut out = Vec::new();
            for item in self.iter() {
                if max_items.is_some_and(|m| out.len() >= m) {
                    break;
                }
                out.push(item?);
            }
            Ok(out)
        }

        /// Every item across every page, as a plain iterator.
        pub fn iter(self) -> BlockingItems<T> {
            BlockingItems {
                pages: self.by_page(),
                buffer: VecDeque::new(),
            }
        }

        /// Every page, one request per page.
        pub fn by_page(self) -> BlockingPages<T> {
            BlockingPages { pager: Some(self) }
        }
    }

    /// An iterator over the pages of a blocking pager.
    pub struct BlockingPages<T> {
        pager: Option<Pager<BlockingTransport, T>>,
    }

    impl<T: DeserializeOwned> Iterator for BlockingPages<T> {
        type Item = Result<Page<T>>;

        fn next(&mut self) -> Option<Self::Item> {
            let pager = self.pager.as_mut()?;
            match pager.next_page() {
                Err(err) => {
                    self.pager = None;
                    Some(Err(err))
                }
                Ok(None) => {
                    self.pager = None;
                    None
                }
                Ok(Some(page)) => Some(Ok(page)),
            }
        }
    }

    /// An iterator over every item of a blocking pager; each `next` may fetch one page.
    pub struct BlockingItems<T> {
        pages: BlockingPages<T>,
        buffer: VecDeque<T>,
    }

    impl<T: DeserializeOwned> Iterator for BlockingItems<T> {
        type Item = Result<T>;

        fn next(&mut self) -> Option<Self::Item> {
            loop {
                if let Some(item) = self.buffer.pop_front() {
                    return Some(Ok(item));
                }
                match self.pages.next()? {
                    Err(err) => return Some(Err(err)),
                    Ok(page) => self.buffer.extend(page.items),
                }
            }
        }
    }
}

#[cfg(feature = "blocking")]
pub use blocking_pager::{BlockingItems, BlockingPages};
