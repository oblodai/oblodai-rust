//! Developer sandbox (`test_` keys only): fake money, simulated deposits, webhook inspector.

use serde_json::json;

use super::base::{Call, RequestBuilder};
use crate::contract::models::{
    FaucetResult, SandboxDeposit, SandboxReplay, SandboxReset, WebhookDelivery,
};
use crate::contract::requests::{SandboxDepositRequest, SandboxFaucetRequest};
use crate::contract::routes;
use crate::core::pagination::Pager;

/// The `sandbox` namespace.
#[derive(Clone, Debug)]
pub struct Sandbox<Tr> {
    transport: Tr,
}

impl<Tr: Clone> Sandbox<Tr> {
    pub(crate) fn new(transport: Tr) -> Self {
        Self { transport }
    }

    /// `POST /v1/sandbox/faucet` — credit test funds. Payout key.
    pub fn faucet(&self, params: SandboxFaucetRequest) -> RequestBuilder<Tr, FaucetResult> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_SANDBOX_FAUCET,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/sandbox/deposit` — simulate an on-chain deposit to an invoice (repeat the txid to
    /// add confirmations).
    pub fn deposit(&self, params: SandboxDepositRequest) -> RequestBuilder<Tr, SandboxDeposit> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_SANDBOX_DEPOSIT,
            Call::new().body(&params).done(),
        )
    }

    /// `GET /v1/sandbox/webhooks` — deliveries with their payloads.
    pub fn webhooks(&self) -> Pager<Tr, WebhookDelivery> {
        Pager::new(
            self.transport.clone(),
            &routes::GET_V1_SANDBOX_WEBHOOKS,
            serde_json::Value::Null,
        )
    }

    /// `POST /v1/sandbox/webhooks/replay` — re-send a terminal (delivered/dead) delivery.
    pub fn replay(&self, delivery_id: impl Into<String>) -> RequestBuilder<Tr, SandboxReplay> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_SANDBOX_WEBHOOKS_REPLAY,
            Call::new()
                .json(json!({ "delivery_id": delivery_id.into() }))
                .done(),
        )
    }

    /// `POST /v1/sandbox/reset` — cancel open invoices and zero balances. Payout key.
    pub fn reset(&self) -> RequestBuilder<Tr, SandboxReset> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_SANDBOX_RESET,
            Call::new().done(),
        )
    }
}
