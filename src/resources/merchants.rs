//! Merchant provisioning — for platforms that onboard merchants themselves.
//!
//! These routes are not HMAC-signed; a self-hosted gateway gates them with its admin token
//! (`ClientBuilder::admin_token`).

use super::base::{Call, RequestBuilder};
use crate::contract::models::{MerchantOnboarded, SandboxStore};
use crate::contract::requests::MerchantsRequest;
use crate::contract::routes;

/// The `merchants` namespace.
#[derive(Clone, Debug)]
pub struct Merchants<Tr> {
    transport: Tr,
}

impl<Tr: Clone> Merchants<Tr> {
    pub(crate) fn new(transport: Tr) -> Self {
        Self { transport }
    }

    /// `POST /v1/merchants` — create a merchant and mint its payment and payout keys (shown once).
    pub fn create(&self, params: MerchantsRequest) -> RequestBuilder<Tr, MerchantOnboarded> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_MERCHANTS,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/merchants/{id}/sandbox` — the merchant's dev store and its `test_` key
    /// (idempotent: a second call returns the same store with `created: false`).
    pub fn create_sandbox(
        &self,
        merchant_id: impl Into<String>,
    ) -> RequestBuilder<Tr, SandboxStore> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_MERCHANTS_ID_SANDBOX,
            Call::new().path("id", merchant_id).done(),
        )
    }
}
