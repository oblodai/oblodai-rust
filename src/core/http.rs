//! The HTTP seam. Everything above it is pure; everything below it is a socket.
//!
//! Implement [`HttpBackend`] (or [`BlockingHttpBackend`]) to route the SDK through your own client
//! — a proxy-aware `reqwest::Client`, an instrumented wrapper, or a recording stub in tests.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use super::engine::RawResponse;
#[cfg(any(feature = "reqwest-client", feature = "blocking"))]
use crate::error::Error;
use crate::error::Result;

/// Largest JSON envelope the SDK buffers. Beyond it the answer is not something the core wrote,
/// and reading on would trade a clear error for an out-of-memory kill.
pub const MAX_JSON_BODY_BYTES: usize = 8 * 1024 * 1024;

/// Largest `bare` document (PDF/CSV) the SDK buffers.
pub const MAX_FILE_BODY_BYTES: usize = 64 * 1024 * 1024;

/// One outgoing HTTP request, fully built and signed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpRequest {
    pub url: String,
    pub method: &'static str,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    /// Timeout for this attempt (already capped by the call's deadline). It covers the whole
    /// exchange, the response body included — not just the first byte.
    pub timeout: Duration,
    /// Refuse to buffer more than this many response bytes: [`MAX_JSON_BODY_BYTES`] on envelope
    /// routes, [`MAX_FILE_BODY_BYTES`] on `bare` document routes.
    pub max_body_bytes: usize,
}

/// The answer was larger than the SDK is willing to hold in memory.
#[cfg(any(feature = "reqwest-client", feature = "blocking"))]
fn too_large(status: u16, cap: usize) -> Error {
    Error::response_too_large(
        format!("response body exceeds the {cap}-byte limit the SDK buffers"),
        status,
    )
}

/// The answer did not come from the URL the SDK signed. Never followed: a redirect strips the
/// signature and can hand an idempotency key to a third party.
#[cfg(any(feature = "reqwest-client", feature = "blocking"))]
fn assert_not_redirected(requested: &str, final_url: &str) -> Result<()> {
    if requested == final_url {
        return Ok(());
    }
    Err(Error::contract(
        format!(
            "unexpected redirect: the HTTP client followed {requested} to {final_url}; the SDK \
             never follows redirects (check base_url and your client's redirect policy)"
        ),
        0,
        None,
    ))
}

/// A future returned by an async backend.
pub type BackendFuture<'a> = Pin<Box<dyn Future<Output = Result<RawResponse>> + Send + 'a>>;

/// Sends a request and returns the answer. A backend must not retry: retries are the SDK's job.
pub trait HttpBackend: Send + Sync {
    fn send<'a>(&'a self, request: HttpRequest) -> BackendFuture<'a>;
}

/// The synchronous counterpart, used by the `blocking` feature.
pub trait BlockingHttpBackend: Send + Sync {
    fn send(&self, request: HttpRequest) -> Result<RawResponse>;
}

#[cfg(feature = "reqwest-client")]
fn collect(status: u16, headers: &reqwest::header::HeaderMap, body: Vec<u8>) -> RawResponse {
    RawResponse {
        status,
        headers: headers
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_string(),
                    v.to_str().unwrap_or_default().to_string(),
                )
            })
            .collect(),
        body,
    }
}

#[cfg(feature = "reqwest-client")]
fn map_error(err: reqwest::Error) -> Error {
    if err.is_timeout() {
        Error::transport("transport.timeout", format!("request timed out: {err}"))
    } else {
        Error::transport("transport.network", format!("network error: {err}"))
    }
}

/// The default async backend: `reqwest` over rustls, redirects disabled (a redirect away from the
/// signed origin is a misconfiguration, not something to follow silently).
///
/// TLS roots come from the bundled `webpki-roots`, which is what makes the default client work with
/// no system configuration — but it also means the OS trust store is ignored. Behind a
/// TLS-inspecting corporate proxy, or against a self-hosted gateway with a private CA, either enable
/// the `native-roots` feature or hand in your own client with [`ReqwestBackend::with_client`].
#[cfg(feature = "reqwest-client")]
#[derive(Clone, Debug)]
pub struct ReqwestBackend {
    client: reqwest::Client,
}

#[cfg(feature = "reqwest-client")]
impl ReqwestBackend {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(std::time::Duration::from_secs(10))
            .pool_idle_timeout(std::time::Duration::from_secs(60))
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| {
                Error::config(
                    "sdk.bad_config",
                    format!("cannot build an HTTP client: {e}"),
                    None,
                )
            })?;
        Ok(Self { client })
    }

    /// Wrap a client you configured yourself (proxy, custom TLS roots, connection limits).
    pub fn with_client(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[cfg(feature = "reqwest-client")]
impl HttpBackend for ReqwestBackend {
    fn send<'a>(&'a self, request: HttpRequest) -> BackendFuture<'a> {
        Box::pin(async move {
            let mut req = self
                .client
                .request(
                    reqwest::Method::from_bytes(request.method.as_bytes())
                        .expect("GET and POST are valid methods"),
                    &request.url,
                )
                .timeout(request.timeout);
            for (k, v) in &request.headers {
                req = req.header(k, v);
            }
            if let Some(body) = request.body {
                req = req.body(body);
            }
            let mut res = req.send().await.map_err(map_error)?;
            assert_not_redirected(&request.url, res.url().as_str())?;
            let status = res.status().as_u16();
            let headers = res.headers().clone();
            // Read in chunks so an unbounded body is refused instead of buffered.
            let mut body: Vec<u8> = Vec::new();
            while let Some(chunk) = res.chunk().await.map_err(map_error)? {
                if body.len() + chunk.len() > request.max_body_bytes {
                    return Err(too_large(status, request.max_body_bytes));
                }
                body.extend_from_slice(&chunk);
            }
            Ok(collect(status, &headers, body))
        })
    }
}

/// The default synchronous backend.
#[cfg(feature = "blocking")]
#[derive(Clone, Debug)]
pub struct ReqwestBlockingBackend {
    client: reqwest::blocking::Client,
}

#[cfg(feature = "blocking")]
impl ReqwestBlockingBackend {
    pub fn new() -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(std::time::Duration::from_secs(10))
            .pool_idle_timeout(std::time::Duration::from_secs(60))
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| {
                Error::config(
                    "sdk.bad_config",
                    format!("cannot build an HTTP client: {e}"),
                    None,
                )
            })?;
        Ok(Self { client })
    }

    pub fn with_client(client: reqwest::blocking::Client) -> Self {
        Self { client }
    }
}

#[cfg(feature = "blocking")]
impl BlockingHttpBackend for ReqwestBlockingBackend {
    fn send(&self, request: HttpRequest) -> Result<RawResponse> {
        let mut req = self
            .client
            .request(
                reqwest::Method::from_bytes(request.method.as_bytes())
                    .expect("GET and POST are valid methods"),
                &request.url,
            )
            .timeout(request.timeout);
        for (k, v) in &request.headers {
            req = req.header(k, v);
        }
        if let Some(body) = request.body {
            req = req.body(body);
        }
        let res = req.send().map_err(map_error)?;
        assert_not_redirected(&request.url, res.url().as_str())?;
        let status = res.status().as_u16();
        let headers = res.headers().clone();
        // One byte over the cap is enough to know the answer is too large.
        let mut body: Vec<u8> = Vec::new();
        std::io::Read::read_to_end(
            &mut std::io::Read::take(res, request.max_body_bytes as u64 + 1),
            &mut body,
        )
        .map_err(|e| Error::transport("transport.network", format!("network error: {e}")))?;
        if body.len() > request.max_body_bytes {
            return Err(too_large(status, request.max_body_bytes));
        }
        Ok(collect(status, &headers, body))
    }
}
