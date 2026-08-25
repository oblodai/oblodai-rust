//! The HTTP seam. Everything above it is pure; everything below it is a socket.
//!
//! Implement [`HttpBackend`] (or [`BlockingHttpBackend`]) to route the SDK through your own client
//! — a proxy-aware `reqwest::Client`, an instrumented wrapper, or a recording stub in tests.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use super::engine::RawResponse;
use crate::error::{Error, Result};

/// One outgoing HTTP request, fully built and signed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpRequest {
    pub url: String,
    pub method: &'static str,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    /// Timeout for this attempt (already capped by the call's deadline).
    pub timeout: Duration,
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
            let res = req.send().await.map_err(map_error)?;
            let status = res.status().as_u16();
            let headers = res.headers().clone();
            let body = res.bytes().await.map_err(map_error)?.to_vec();
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
        let status = res.status().as_u16();
        let headers = res.headers().clone();
        let body = res.bytes().map_err(map_error)?.to_vec();
        Ok(collect(status, &headers, body))
    }
}
