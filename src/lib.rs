//! Official Rust SDK for the [Oblodai](https://oblodai.com) crypto payment gateway: invoices,
//! payouts, refunds, payout links, static wallets, webhooks and documents — the whole merchant API,
//! generated from the gateway's OpenAPI contract and typed end to end.
//!
//! ```no_run
//! use oblodai::{Client, models::PaymentRequest};
//!
//! # async fn demo() -> oblodai::Result<()> {
//! let client = Client::from_env()?;
//! let invoice = client
//!     .payments()
//!     .create(PaymentRequest {
//!         network: Some("tron".into()),       // omit to let the payer choose on the pay page
//!         order_id: Some("order-1001".into()),
//!         ..PaymentRequest::new("25", "USDT") // amounts are decimal strings, never floats
//!     })
//!     .await?;
//! println!("{} {}", invoice.url, invoice.address);
//! # Ok(()) }
//! ```
//!
//! # Layout
//!
//! - `client.<resource>().<method>(…)` — one namespace per area of the API ([`generated::resources`]),
//!   one method per operation; names are fixed in `names.lock`.
//! - [`models`] and [`enums`] — request and response types. A field this SDK version does not know
//!   lands in the model's `extra` map; an enum value it does not know parses to `Other`.
//! - Every method returns a builder: [`Request`] (one answer), [`Pager`] (a paged list) or a
//!   `Request` of [`FileResult`] (a document). Set call options on it — `idempotency_key`,
//!   `timeout`, `max_retries`, `extra_header`, `request_id` — then `.await` it.
//!
//! # What the SDK does for you
//!
//! - **Signs** every request with the five-field recipe the gateway verifies
//!   (`ts \n METHOD \n path+query \n Idempotency-Key \n body`).
//! - **Retries** only what the API itself marks `retryable`, and only when re-sending cannot
//!   duplicate a side effect — a retry-safe route, or a write the gateway deduplicates by
//!   `Idempotency-Key`.
//! - **Generates an idempotency key** per logical call on create routes and reuses it across
//!   retries, so a timeout can never produce a second payout.
//! - **Names every call** with an `X-Request-ID` (yours, or a fresh UUID), the same on every
//!   attempt; errors print it: `[payout.insufficient_funds] … (request_id=…)`.
//! - **Corrects clock skew** once, from the server's `Date` header, and reverts the correction if
//!   it did not help.
//! - **Follows long-running operations**: [`Request::job`] and [`Job::wait`] (see [`lro`]).
//! - **Verifies webhooks** without a client: see [`webhooks`].
//!
//! # Errors
//!
//! Every failure is an [`Error`] carrying the API's envelope: [`code`](Error::code),
//! [`http_status`](Error::http_status), [`retryable`](Error::retryable),
//! [`retry_after`](Error::retry_after), [`request_id`](Error::request_id) and
//! [`field`](Error::field), classified by [`ErrorKind`]. The raw body is never printed by `Debug`
//! and never serialized.
//!
//! # Feature flags
//!
//! - `reqwest-client` *(default)* — the async client over `reqwest` with rustls.
//! - `blocking` — a synchronous [`blocking::Client`] over the same pure core.
//! - `native-roots` — also trust the OS certificate store (a private CA, a TLS-inspecting proxy).
//!   Without it the default client trusts the bundled webpki roots only.
//!
//! # Minimum supported Rust version
//!
//! **1.86**. The floor comes from the dependency tree, not from the SDK's own code. Bumping it is
//! a minor-version change.

#![forbid(unsafe_code)]
// With no client feature there is no `Client` to construct the resource namespaces or resolve a
// configuration, so those private constructors look dead. They are not: `reqwest-client` (the
// default) and `blocking` both use them. The pure core, the models and `webhooks` stay usable
// on their own, which is what `--no-default-features` is for.
#![cfg_attr(
    not(feature = "reqwest-client"),
    allow(dead_code, unused_macros, unused_imports)
)]
// The error carries the gateway's whole envelope (code, message, request id, field) by value,
// because that is what callers match on. Boxing it to satisfy `result_large_err` would put an
// allocation on every failure path and an extra deref in every `match err.code()`.
#![allow(clippy::result_large_err)]

#[cfg(feature = "reqwest-client")]
mod client;
pub mod config;
pub mod core;
pub mod error;
pub mod generated;
pub mod helpers;
pub mod lro;
pub mod resources;
pub mod webhooks;

#[cfg(feature = "blocking")]
pub mod blocking;

#[cfg(feature = "reqwest-client")]
pub use client::Client;
pub use config::{ClientBuilder, ClientOptions, DEFAULT_BASE_URL, SDK_VERSION};
pub use error::{Error, ErrorDetail, ErrorKind, Result};

pub use generated::{enums, models, routes};

pub use crate::core::clock::Clock;
pub use crate::core::envelope::{Page, Paginate};
pub use crate::core::hooks::{Hooks, RequestInfo, ResponseInfo};
pub use crate::core::http::{BackendFuture, HttpBackend, HttpRequest};
pub use crate::core::logger::{LogLevel, Logger};
pub use crate::core::money::Money;
pub use crate::core::retry::RetryOptions;
pub use crate::core::route::{ListKind, Method, RouteAuth, RouteSpec};
pub use crate::core::signing::{canonical_string, sign_request, sign_webhook, SignInput};
pub use crate::core::transport::Transport;
pub use lro::{Job, JobAck, JobStatus};
pub use resources::{
    from_json, FileResult, ItemStream, PageStream, Pager, RawApiResponse, Request, WithRawResponse,
};
pub use webhooks::{
    is_known_event, is_stale_event, is_test_event, parse_webhook, verify_webhook,
    verify_webhook_delivery, VerifyOptions, WebhookDeliveryInfo, WebhookEvent,
};

#[cfg(feature = "blocking")]
pub use crate::core::http::BlockingHttpBackend;
#[cfg(feature = "blocking")]
pub use crate::core::transport::BlockingTransport;
