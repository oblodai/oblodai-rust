//! Webhook endpoint management and delivery inspection. Verification itself needs no client:
//! see [`crate::webhooks`].

use serde_json::json;

use super::base::{Call, RequestBuilder};
use crate::contract::enums::WebhookKind;
use crate::contract::models::{
    WebhookDelivery, WebhookEndpoint, WebhookSecretRotated, WebhookTestResult,
};
use crate::contract::requests::{
    PaymentTestingWebhookRequest, TestWebhookPaymentRequest, WebhooksDeliveriesRequest,
};
use crate::contract::routes;
use crate::core::pagination::Pager;

/// The `webhooks` namespace.
#[derive(Clone, Debug)]
pub struct Webhooks<Tr> {
    transport: Tr,
}

impl<Tr: Clone> Webhooks<Tr> {
    pub(crate) fn new(transport: Tr) -> Self {
        Self { transport }
    }

    /// `POST /v1/webhooks` — register (or replace) the merchant's endpoint; returns the signing
    /// secret once.
    pub fn register(&self, url: impl Into<String>) -> RequestBuilder<Tr, WebhookEndpoint> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_WEBHOOKS,
            Call::new().json(json!({ "url": url.into() })).done(),
        )
    }

    /// `POST /v1/webhooks/rotate-secret` — new secret; the old one keeps verifying until
    /// `previous_secret_valid_until`. Payout key.
    pub fn rotate_secret(&self) -> RequestBuilder<Tr, WebhookSecretRotated> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_WEBHOOKS_ROTATE_SECRET,
            Call::new().done(),
        )
    }

    /// `POST /v1/webhooks/deliveries` — delivery log, newest first.
    pub fn deliveries(&self, params: WebhooksDeliveriesRequest) -> Pager<Tr, WebhookDelivery> {
        Pager::new(
            self.transport.clone(),
            &routes::POST_V1_WEBHOOKS_DELIVERIES,
            super::base::to_value(&params),
        )
    }

    /// `POST /v1/test-webhook/{payment|payout|wallet}` — deliver a sample event of that kind to
    /// `url_callback`, signed like a real one.
    ///
    /// The three routes share one request shape; the `payout` variant needs the payout key.
    pub fn test(
        &self,
        kind: WebhookKind,
        params: TestWebhookPaymentRequest,
    ) -> RequestBuilder<Tr, WebhookTestResult> {
        let route = match kind {
            WebhookKind::Payment => &routes::POST_V1_TEST_WEBHOOK_PAYMENT,
            WebhookKind::Payout => &routes::POST_V1_TEST_WEBHOOK_PAYOUT,
            WebhookKind::Wallet => &routes::POST_V1_TEST_WEBHOOK_WALLET,
            // An unknown kind can only come from a newer core; the payment door is the one every
            // merchant has, and the core answers with a clear refusal if the kind is wrong.
            WebhookKind::Other(_) => &routes::POST_V1_TEST_WEBHOOK_PAYMENT,
        };
        RequestBuilder::new(
            self.transport.clone(),
            route,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/payment/testing-webhook` — the older rehearsal door (payment events only).
    #[deprecated(note = "use test(WebhookKind::Payment, …)")]
    pub fn test_legacy(
        &self,
        params: PaymentTestingWebhookRequest,
    ) -> RequestBuilder<Tr, WebhookTestResult> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_TESTING_WEBHOOK,
            Call::new().body(&params).done(),
        )
    }
}
