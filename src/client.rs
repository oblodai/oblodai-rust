//! The async client: one instance per key pair, cheap to clone, safe to share.

use std::sync::Arc;

use crate::config::{ClientBuilder, ClientOptions};
use crate::core::http::{HttpBackend, ReqwestBackend};
use crate::core::transport::Transport;
use crate::error::Result;

/// The Oblodai API client. Every namespace (`client.payments()`, `client.payouts()`, …) is
/// generated from the gateway's OpenAPI contract; every method returns a builder that sends
/// nothing until it is awaited.
///
/// ```no_run
/// # use oblodai::{Client, models::PaymentRequest};
/// # async fn demo() -> oblodai::Result<()> {
/// let client = Client::new("oblodai_…", "oblodai_live_…")?;
/// let invoice = client
///     .payments()
///     .create(PaymentRequest {
///         network: Some("tron".into()),
///         order_id: Some("order-1001".into()),
///         ..PaymentRequest::new("25", "USDT")
///     })
///     .await?;
/// println!("{} {}", invoice.url, invoice.address);
/// # Ok(()) }
/// ```
#[derive(Clone, Debug)]
pub struct Client {
    transport: Transport,
}

impl Client {
    /// A client for the merchant's API key, everything else defaulted (and read from `OBLODAI_*`
    /// when set).
    pub fn new(public_id: impl Into<String>, secret: impl Into<String>) -> Result<Self> {
        ClientBuilder::new()
            .public_id(public_id)
            .secret(secret)
            .build()
    }

    /// A client configured entirely from `OBLODAI_PUBLIC_ID`, `OBLODAI_SECRET`,
    /// `OBLODAI_BASE_URL`, `OBLODAI_ADMIN_TOKEN`, `OBLODAI_LOG` and `OBLODAI_ALLOW_INSECURE`.
    pub fn from_env() -> Result<Self> {
        ClientBuilder::new().build()
    }

    /// Configure a client explicitly.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// The transport, for advanced use (a route this SDK does not wrap yet, or a test).
    pub fn transport(&self) -> Transport {
        self.transport.clone()
    }

    /// A copy of this client with some settings changed; the original is untouched, and both
    /// share the HTTP connection pool and the clock correction.
    ///
    /// ```no_run
    /// # fn demo(client: &oblodai::Client) {
    /// use std::time::Duration;
    /// use oblodai::ClientOptions;
    ///
    /// let patient = client.with_options(ClientOptions::new().timeout(Duration::from_secs(60)).max_retries(5));
    /// # let _ = patient;
    /// # }
    /// ```
    pub fn with_options(&self, options: ClientOptions) -> Self {
        Self {
            transport: self.transport.with_core(|core| options.apply(core)),
        }
    }

    crate::generated::resource_accessors!(Transport);
}

impl ClientBuilder {
    /// Build the async client.
    pub fn build(self) -> Result<Client> {
        let core = Arc::new(self.resolve()?);
        let backend: Arc<dyn HttpBackend> = match self.backend {
            Some(b) => b,
            None => Arc::new(ReqwestBackend::new()?),
        };
        Ok(Client {
            transport: Transport::new(core, backend),
        })
    }
}
