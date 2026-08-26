//! Binary (`bare`) routes: the builder for a document call and the result it hands back.

use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::time::Duration;

use crate::contract::types::RouteSpec;
use crate::core::engine::CallOptions;
use crate::core::transport::Transport;
use crate::error::Result;

/// A document the gateway rendered: the bytes plus what they are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileResult {
    pub bytes: Vec<u8>,
    pub content_type: String,
    /// From `Content-Disposition`, when the gateway named the file.
    pub filename: Option<String>,
}

/// A prepared call to a `bare` route: it answers with bytes, not an envelope.
///
/// The same cancellation caveat as [`RequestBuilder`](super::base::RequestBuilder) applies: prefer
/// [`deadline`](Self::deadline) over dropping the future.
#[must_use = "nothing is sent until this builder is awaited (or `.send()` is called)"]
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

    /// Sign with the payout key on a route that accepts either key kind.
    pub fn prefer_payout_key(mut self, prefer: bool) -> Self {
        self.opts.prefer_payout_key = prefer;
        self
    }

    /// An extra header on this call only, merged over the client-wide ones. Names the SDK owns
    /// (the signing headers, `Accept`, `Content-Type`, `User-Agent`, `X-Admin-Token`) are never
    /// overridden; a CR/LF or non-ASCII value is a `sdk.bad_header` config error.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.opts.headers.push((name.into(), value.into()));
        self
    }

    /// Present for symmetry with [`RequestBuilder`](super::base::RequestBuilder), but every `bare` document route in the
    /// contract is `idempotent: false`, so this always fails the call with
    /// `sdk.idempotency_unsupported` before anything is sent.
    #[deprecated(
        note = "no document route deduplicates by Idempotency-Key; setting one always fails with \
                sdk.idempotency_unsupported"
    )]
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
