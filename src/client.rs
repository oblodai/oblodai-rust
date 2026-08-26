//! The async client: one instance per key pair, cheap to clone, safe to share.

use std::sync::Arc;

use crate::config::ClientBuilder;
use crate::core::http::{HttpBackend, ReqwestBackend};
use crate::core::transport::Transport;
use crate::error::Result;
use crate::resources::{
    Account, Batches, Catalog, Documents, Merchants, PaymentLinks, Payments, PayoutLinks, Payouts,
    Refunds, Sandbox, Settings, Splits, Transfers, Wallets, Webhooks,
};

/// The Oblodai API client.
///
/// ```no_run
/// # use oblodai::{Client, contract::requests::PaymentRequest};
/// # async fn demo() -> oblodai::Result<()> {
/// let client = Client::new("oblodai_…", "oblodai_live_…")?;
/// let invoice = client
///     .payments()
///     .create(PaymentRequest {
///         amount: "25".into(),
///         currency: "USDT".into(),
///         network: Some("tron".into()),
///         order_id: Some("order-1001".into()),
///         ..Default::default()
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

    pub(crate) fn with_transport(transport: Transport) -> Self {
        Self { transport }
    }

    /// The transport, for advanced use (a route this SDK does not wrap yet, or a test).
    pub fn transport(&self) -> &Transport {
        &self.transport
    }

    /// Invoices, the payer-facing checkout endpoints included.
    pub fn payments(&self) -> Payments<Transport> {
        Payments::new(self.transport.clone())
    }

    /// Refunds and underpayment resolution.
    pub fn refunds(&self) -> Refunds<Transport> {
        Refunds::new(self.transport.clone())
    }

    /// Payouts to external addresses.
    pub fn payouts(&self) -> Payouts<Transport> {
        Payouts::new(self.transport.clone())
    }

    /// Payout links (cheques).
    pub fn payout_links(&self) -> PayoutLinks<Transport> {
        PayoutLinks::new(self.transport.clone())
    }

    /// Reusable payment links.
    pub fn payment_links(&self) -> PaymentLinks<Transport> {
        PaymentLinks::new(self.transport.clone())
    }

    /// Progress of asynchronous batches.
    pub fn batches(&self) -> Batches<Transport> {
        Batches::new(self.transport.clone())
    }

    /// Internal transfers between platform balances.
    pub fn transfers(&self) -> Transfers<Transport> {
        Transfers::new(self.transport.clone())
    }

    /// Static deposit wallets.
    pub fn wallets(&self) -> Wallets<Transport> {
        Wallets::new(self.transport.clone())
    }

    /// Webhook endpoints and deliveries.
    pub fn webhooks(&self) -> Webhooks<Transport> {
        Webhooks::new(self.transport.clone())
    }

    /// Generated PDF/CSV documents.
    pub fn documents(&self) -> Documents<Transport> {
        Documents::new(self.transport.clone())
    }

    /// Revenue splits.
    pub fn splits(&self) -> Splits<Transport> {
        Splits::new(self.transport.clone())
    }

    /// Merchant-level configuration.
    pub fn settings(&self) -> Settings<Transport> {
        Settings::new(self.transport.clone())
    }

    /// Balances and account-level facts.
    pub fn account(&self) -> Account<Transport> {
        Account::new(self.transport.clone())
    }

    /// Public reference data — no credentials needed.
    pub fn catalog(&self) -> Catalog<Transport> {
        Catalog::new(self.transport.clone())
    }

    /// The developer sandbox (`test_` keys only).
    pub fn sandbox(&self) -> Sandbox<Transport> {
        Sandbox::new(self.transport.clone())
    }

    /// Merchant provisioning, for platforms that onboard merchants themselves.
    pub fn merchants(&self) -> Merchants<Transport> {
        Merchants::new(self.transport.clone())
    }
}

impl ClientBuilder {
    /// Build the async client.
    pub fn build(self) -> Result<Client> {
        let core = Arc::new(self.resolve()?);
        let backend: Arc<dyn HttpBackend> = match self.backend {
            Some(b) => b,
            None => Arc::new(ReqwestBackend::new()?),
        };
        Ok(Client::with_transport(Transport::new(core, backend)))
    }
}
