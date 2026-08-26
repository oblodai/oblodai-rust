//! The synchronous client, behind the `blocking` feature.
//!
//! It is the same method tree as [`crate::Client`] — the resource types are generic over their
//! transport — driving the same pure core (signing, envelopes, retry decisions) over a blocking
//! HTTP backend. Methods return the value instead of a future: `client.payments().create(p).send()?`.

use std::sync::Arc;

use crate::config::ClientBuilder;
use crate::core::http::{BlockingHttpBackend, ReqwestBlockingBackend};
use crate::core::transport::BlockingTransport;
use crate::error::Result;
use crate::resources::{
    Account, Batches, Catalog, Documents, Merchants, PaymentLinks, Payments, PayoutLinks, Payouts,
    Refunds, Sandbox, Settings, Splits, Transfers, Wallets, Webhooks,
};

/// The synchronous Oblodai API client.
///
/// ```no_run
/// # use oblodai::{blocking::Client, contract::requests::PaymentRequest};
/// # fn demo() -> oblodai::Result<()> {
/// let client = Client::new("oblodai_…", "oblodai_live_…")?;
/// let invoice = client
///     .payments()
///     .create(PaymentRequest { amount: "25".into(), currency: "USDT".into(), ..Default::default() })
///     .send()?;
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
    pub fn transport(&self) -> &BlockingTransport {
        &self.transport
    }

    /// Invoices, the payer-facing checkout endpoints included.
    pub fn payments(&self) -> Payments<BlockingTransport> {
        Payments::new(self.transport.clone())
    }

    /// Refunds and underpayment resolution. Payout key.
    pub fn refunds(&self) -> Refunds<BlockingTransport> {
        Refunds::new(self.transport.clone())
    }

    /// Payouts to external addresses. Payout key.
    pub fn payouts(&self) -> Payouts<BlockingTransport> {
        Payouts::new(self.transport.clone())
    }

    /// Payout links (cheques). Payout key.
    pub fn payout_links(&self) -> PayoutLinks<BlockingTransport> {
        PayoutLinks::new(self.transport.clone())
    }

    /// Reusable payment links.
    pub fn payment_links(&self) -> PaymentLinks<BlockingTransport> {
        PaymentLinks::new(self.transport.clone())
    }

    /// Progress of asynchronous batches.
    pub fn batches(&self) -> Batches<BlockingTransport> {
        Batches::new(self.transport.clone())
    }

    /// Internal transfers between platform balances. Payout key.
    pub fn transfers(&self) -> Transfers<BlockingTransport> {
        Transfers::new(self.transport.clone())
    }

    /// Static deposit wallets.
    pub fn wallets(&self) -> Wallets<BlockingTransport> {
        Wallets::new(self.transport.clone())
    }

    /// Webhook endpoints and deliveries.
    pub fn webhooks(&self) -> Webhooks<BlockingTransport> {
        Webhooks::new(self.transport.clone())
    }

    /// Generated PDF/CSV documents.
    pub fn documents(&self) -> Documents<BlockingTransport> {
        Documents::new(self.transport.clone())
    }

    /// Revenue splits. Payout key.
    pub fn splits(&self) -> Splits<BlockingTransport> {
        Splits::new(self.transport.clone())
    }

    /// Merchant-level configuration.
    pub fn settings(&self) -> Settings<BlockingTransport> {
        Settings::new(self.transport.clone())
    }

    /// Balances and account-level facts.
    pub fn account(&self) -> Account<BlockingTransport> {
        Account::new(self.transport.clone())
    }

    /// Public reference data — no credentials needed.
    pub fn catalog(&self) -> Catalog<BlockingTransport> {
        Catalog::new(self.transport.clone())
    }

    /// The developer sandbox (`test_` keys only).
    pub fn sandbox(&self) -> Sandbox<BlockingTransport> {
        Sandbox::new(self.transport.clone())
    }

    /// Merchant provisioning, for platforms that onboard merchants themselves.
    pub fn merchants(&self) -> Merchants<BlockingTransport> {
        Merchants::new(self.transport.clone())
    }
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
