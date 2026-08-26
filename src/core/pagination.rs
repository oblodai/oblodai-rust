//! Offset pagination over the core's `{items, paginate}` lists.
//!
//! A [`Pager`] is a lazy plan, not a result: nothing is requested until it is awaited, streamed or
//! collected. `paginate.has_pages` is the server's own "there is more" flag, and iteration stops
//! on it (or on an empty page). A *short* page does NOT end the walk: the core may return fewer
//! rows than the limit and still have more.

use std::collections::VecDeque;
use std::future::{Future, IntoFuture};
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use super::engine::CallOptions;
use super::envelope::{decode_result, Page};
use super::transport::Transport;
use crate::contract::types::{Method, RouteSpec};
use crate::error::Result;

/// Page size used when the caller does not choose one.
pub const DEFAULT_PAGE_LIMIT: i64 = 50;

fn query_pairs(params: &Map<String, Value>) -> Vec<(String, String)> {
    params
        .iter()
        .filter_map(|(k, v)| match v {
            Value::Null => None,
            Value::String(s) => Some((k.clone(), s.clone())),
            other => Some((k.clone(), other.to_string())),
        })
        .collect()
}

/// A lazy walk over a paged list route, generic over the transport that drives it.
///
/// ```no_run
/// # async fn demo(client: &oblodai::Client) -> oblodai::Result<()> {
/// use futures_util::StreamExt;
/// // one page
/// let page = client.payments().history(Default::default()).limit(50).await?;
/// // every page, lazily
/// let mut items = client.payments().history(Default::default()).stream();
/// while let Some(payment) = items.next().await {
///     println!("{}", payment?.uuid);
/// }
/// # Ok(()) }
/// ```
#[must_use = "a pager requests nothing until it is awaited, streamed or collected"]
pub struct Pager<Tr, T> {
    transport: Tr,
    route: &'static RouteSpec,
    params: Map<String, Value>,
    limit: i64,
    offset: i64,
    prefer_payout_key: bool,
    headers: Vec<(String, String)>,
    timeout: Option<Duration>,
    deadline: Option<Duration>,
    finished: bool,
    _item: PhantomData<fn() -> T>,
}

impl<Tr, T> std::fmt::Debug for Pager<Tr, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pager")
            .field("route", &self.route.key)
            .field("limit", &self.limit)
            .field("offset", &self.offset)
            .finish_non_exhaustive()
    }
}

impl<Tr, T> Pager<Tr, T> {
    pub(crate) fn new(transport: Tr, route: &'static RouteSpec, params: Value) -> Self {
        let mut map = match params {
            Value::Object(m) => m,
            _ => Map::new(),
        };
        // Pull `limit`/`offset` out of the caller's parameters so they cannot be sent twice.
        let limit = map.remove("limit").and_then(|v| v.as_i64());
        let offset = map.remove("offset").and_then(|v| v.as_i64());
        Self {
            transport,
            route,
            params: map,
            limit: limit.unwrap_or(DEFAULT_PAGE_LIMIT),
            offset: offset.unwrap_or(0),
            prefer_payout_key: false,
            headers: Vec::new(),
            timeout: None,
            deadline: None,
            finished: false,
            _item: PhantomData,
        }
    }

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

    /// Per-attempt timeout for every page.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Overall budget for every page, retries included.
    pub fn deadline(mut self, deadline: Duration) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Sign with the payout key on a route that accepts either kind.
    pub fn prefer_payout_key(mut self, prefer: bool) -> Self {
        self.prefer_payout_key = prefer;
        self
    }

    /// An extra header on every page of this walk, merged over the client-wide ones.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    fn call_options(&self, limit: i64, offset: i64) -> CallOptions {
        let mut opts = CallOptions {
            prefer_payout_key: self.prefer_payout_key,
            headers: self.headers.clone(),
            timeout: self.timeout,
            deadline: self.deadline,
            // A key per page would be wrong on both sides: the core would replay page 1 forever.
            idempotency_key: None,
            ..Default::default()
        };
        if self.route.method == Method::Get {
            let mut query = query_pairs(&self.params);
            query.push(("limit".into(), limit.to_string()));
            query.push(("offset".into(), offset.to_string()));
            opts.query = query;
        } else {
            let mut body = self.params.clone();
            body.insert("limit".into(), Value::from(limit));
            body.insert("offset".into(), Value::from(offset));
            opts.body = Some(Value::Object(body));
        }
        opts
    }

    /// Note the page just fetched and say whether the walk is over.
    fn advance(&mut self, page: &Page<T>) {
        let got = page.items.len() as i64;
        self.offset += got;
        if got == 0 || !page.paginate.has_pages {
            self.finished = true;
        }
    }
}

impl<T: DeserializeOwned> Pager<Transport, T> {
    /// Fetch exactly one page at the configured offset, leaving the pager where it was.
    pub async fn page(&self) -> Result<Page<T>> {
        let value = self
            .transport
            .call_value(self.route, self.call_options(self.limit, self.offset))
            .await?;
        decode_result(value, self.route.key)
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
            pager: Some(self),
            pending: None,
            buffer: VecDeque::new(),
            done: false,
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

/// A [`Stream`](futures_core::Stream) of items across pages.
#[must_use = "a stream fetches nothing until it is polled"]
pub struct ItemStream<T> {
    pager: Option<Pager<Transport, T>>,
    pending: Option<PageFuture<T>>,
    buffer: VecDeque<T>,
    done: bool,
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
            if this.done {
                return Poll::Ready(None);
            }
            if this.pending.is_none() {
                let Some(mut pager) = this.pager.take() else {
                    this.done = true;
                    return Poll::Ready(None);
                };
                this.pending = Some(Box::pin(async move {
                    let page = pager.next_page().await;
                    (pager, page)
                }));
            }
            let fut = this.pending.as_mut().expect("just set");
            match fut.as_mut().poll(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready((pager, result)) => {
                    this.pending = None;
                    match result {
                        Err(err) => {
                            this.done = true;
                            return Poll::Ready(Some(Err(err)));
                        }
                        Ok(None) => {
                            this.done = true;
                            return Poll::Ready(None);
                        }
                        Ok(Some(page)) => {
                            this.buffer.extend(page.items);
                            this.pager = Some(pager);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(feature = "blocking")]
impl<T: DeserializeOwned> Pager<super::transport::BlockingTransport, T> {
    /// Fetch exactly one page at the configured offset, leaving the pager where it was.
    pub fn page(&self) -> Result<Page<T>> {
        let value = self
            .transport
            .call_value(self.route, self.call_options(self.limit, self.offset))?;
        decode_result(value, self.route.key)
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
    pub fn all(mut self, max_items: Option<usize>) -> Result<Vec<T>> {
        let mut out = Vec::new();
        while let Some(page) = self.next_page()? {
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

    /// Every item across every page, as a plain iterator.
    pub fn iter(self) -> BlockingItems<T> {
        BlockingItems {
            pager: Some(self),
            buffer: VecDeque::new(),
        }
    }
}

/// An iterator over every item of a blocking pager; each `next` may fetch one page.
#[cfg(feature = "blocking")]
pub struct BlockingItems<T> {
    pager: Option<Pager<super::transport::BlockingTransport, T>>,
    buffer: VecDeque<T>,
}

#[cfg(feature = "blocking")]
impl<T: DeserializeOwned> Iterator for BlockingItems<T> {
    type Item = Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(item) = self.buffer.pop_front() {
                return Some(Ok(item));
            }
            let pager = self.pager.as_mut()?;
            match pager.next_page() {
                Err(err) => {
                    self.pager = None;
                    return Some(Err(err));
                }
                Ok(None) => {
                    self.pager = None;
                    return None;
                }
                Ok(Some(page)) => self.buffer.extend(page.items),
            }
        }
    }
}
