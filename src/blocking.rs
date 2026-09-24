//! The synchronous client, behind the `blocking` feature.
//!
//! It is the same method tree as [`crate::Client`] — the generated namespaces are generic over
//! their transport — driving the same pure core (signing, envelopes, retry decisions) over a
//! blocking HTTP backend. Builders are sent with `.send()`: `client.payments().create(p).send()?`.

use std::sync::Arc;

use crate::config::{ClientBuilder, ClientOptions};
use crate::core::http::{BlockingHttpBackend, ReqwestBlockingBackend};
use crate::core::transport::BlockingTransport;
use crate::error::Result;

/// The synchronous Oblodai API client.
///
/// ```no_run
/// # use oblodai::{blocking::Client, models::PaymentRequest};
/// # fn demo() -> oblodai::Result<()> {
/// let client = Client::new("oblodai_…", "oblodai_live_…")?;
/// let invoice = client.payments().create(PaymentRequest::new("25", "USDT")).send()?;
/// # let _ = invoice;
/// # Ok(()) }
/// ```
#[derive(Clone, Debug)]
pub struct Client {
    transport: BlockingTransport,
}

impl Client {
    /// A client for one key pair.
    pub fn new(public_id: impl Into<String>, secret: impl Into<String>) -> Result<Self> {
        ClientBuilder::new()
            .public_id(public_id)
            .secret(secret)
            .build_blocking()
    }

    /// A client configured entirely from the `OBLODAI_*` environment variables.
    pub fn from_env() -> Result<Self> {
        ClientBuilder::new().build_blocking()
    }

    /// Configure a client explicitly, then call
    /// [`build_blocking`](ClientBuilder::build_blocking).
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// The transport, for advanced use.
    pub fn transport(&self) -> BlockingTransport {
        self.transport.clone()
    }

    /// A copy of this client with some settings changed; see
    /// [`crate::Client::with_options`].
    pub fn with_options(&self, options: ClientOptions) -> Self {
        Self {
            transport: self.transport.with_core(|core| options.apply(core)),
        }
    }

    crate::generated::resource_accessors!(BlockingTransport);
}

impl ClientBuilder {
    /// Build the synchronous client.
    pub fn build_blocking(self) -> Result<Client> {
        let core = Arc::new(self.resolve()?);
        let backend: Arc<dyn BlockingHttpBackend> = match self.blocking_backend {
            Some(b) => b,
            None => Arc::new(ReqwestBlockingBackend::new()?),
        };
        Ok(Client {
            transport: BlockingTransport::new(core, backend),
        })
    }
}
