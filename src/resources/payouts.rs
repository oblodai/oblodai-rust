//! Outgoing transfers to external addresses. Every route here needs the payout key.

use serde_json::json;

use super::base::{Call, PayoutLookup, RequestBuilder};
use crate::contract::models::{
    BatchElement, BatchSubmitted, Payout, PayoutCalculation, PayoutFeeConfig, PayoutValidation,
    RefundFeeConfig, ServiceMethod,
};
use crate::contract::requests::{
    PayoutBatchRequest, PayoutCalculateRequest, PayoutFeeConfigSetRequest, PayoutHistoryRequest,
    PayoutMassRequest, PayoutRefundFeeConfigSetRequest, PayoutRequest, PayoutServicesRequest,
    PayoutValidateRequest,
};
use crate::contract::routes;
use crate::core::envelope::PlainList;
use crate::core::pagination::Pager;

/// The `payouts` namespace.
#[derive(Clone, Debug)]
pub struct Payouts<Tr> {
    transport: Tr,
}

impl<Tr: Clone> Payouts<Tr> {
    pub(crate) fn new(transport: Tr) -> Self {
        Self { transport }
    }

    /// `POST /v1/payout` — create and (for API keys) auto-approve a payout. Idempotent by
    /// `order_id` and by `Idempotency-Key`. Errors worth handling: `payout.insufficient_funds`
    /// (retryable), `payout.funds_maturing`, `payout.bad_address`, `payout.memo_required`.
    pub fn create(&self, params: PayoutRequest) -> RequestBuilder<Tr, Payout> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/payout/validate` — dry run: every check `create` makes, nothing reserved or sent.
    pub fn validate(&self, params: PayoutValidateRequest) -> RequestBuilder<Tr, PayoutValidation> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_VALIDATE,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/payout/calculate` — commission and net amount without creating anything.
    pub fn calculate(
        &self,
        params: PayoutCalculateRequest,
    ) -> RequestBuilder<Tr, PayoutCalculation> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_CALCULATE,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/payout/info` — by `uuid` or `order_id`. Refunds are payouts too (`is_refund`).
    pub fn info(&self, lookup: impl Into<PayoutLookup>) -> RequestBuilder<Tr, Payout> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_INFO,
            Call::new().body(&lookup.into()).done(),
        )
    }

    /// Alias of [`info`](Self::info).
    pub fn get(&self, lookup: impl Into<PayoutLookup>) -> RequestBuilder<Tr, Payout> {
        self.info(lookup)
    }

    /// `POST /v1/payout/cancel` — cancel while not yet broadcast (pending/approved/awaiting_cosign);
    /// 409 `payout.not_pending` after.
    pub fn cancel(&self, uuid: impl Into<String>) -> RequestBuilder<Tr, Payout> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_CANCEL,
            Call::new().json(json!({ "uuid": uuid.into() })).done(),
        )
    }

    /// `POST /v1/payout/approve` — approve a payout awaiting manual approval.
    pub fn approve(&self, uuid: impl Into<String>) -> RequestBuilder<Tr, Payout> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_APPROVE,
            Call::new().json(json!({ "uuid": uuid.into() })).done(),
        )
    }

    /// `POST /v1/payout/history` — newest first. `kind: "refund"` lists refunds only.
    pub fn history(&self, params: PayoutHistoryRequest) -> Pager<Tr, Payout> {
        Pager::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_HISTORY,
            super::base::to_value(&params),
        )
    }

    /// Alias of [`history`](Self::history).
    pub fn list(&self, params: PayoutHistoryRequest) -> Pager<Tr, Payout> {
        self.history(params)
    }

    /// `POST /v1/payout/mass` — SYNCHRONOUS batch (≤100): each element reports its own outcome in
    /// the response.
    pub fn mass(
        &self,
        params: PayoutMassRequest,
    ) -> RequestBuilder<Tr, PlainList<BatchElement<Payout>>> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_MASS,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/payout/batch` — ASYNCHRONOUS batch (≤5000): returns a ticket; poll
    /// `batches().info()`. `order_id` is required on every item.
    pub fn batch(&self, params: PayoutBatchRequest) -> RequestBuilder<Tr, BatchSubmitted> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_BATCH,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/payout/services` — currencies/networks available for payouts.
    pub fn services(&self, params: PayoutServicesRequest) -> Pager<Tr, ServiceMethod> {
        Pager::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_SERVICES,
            super::base::to_value(&params),
        )
    }

    /// `POST /v1/payout/fee-config/get`.
    pub fn get_fee_config(&self) -> RequestBuilder<Tr, PayoutFeeConfig> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_FEE_CONFIG_GET,
            Call::new().done(),
        )
    }

    /// `POST /v1/payout/fee-config/set` — who bears the network fee by default.
    pub fn set_fee_config(
        &self,
        params: PayoutFeeConfigSetRequest,
    ) -> RequestBuilder<Tr, PayoutFeeConfig> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_FEE_CONFIG_SET,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/payout/refund-fee-config/get`.
    pub fn get_refund_fee_config(&self) -> RequestBuilder<Tr, RefundFeeConfig> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_REFUND_FEE_CONFIG_GET,
            Call::new().done(),
        )
    }

    /// `POST /v1/payout/refund-fee-config/set` — who bears the fee on refunds.
    pub fn set_refund_fee_config(
        &self,
        params: PayoutRefundFeeConfigSetRequest,
    ) -> RequestBuilder<Tr, RefundFeeConfig> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_REFUND_FEE_CONFIG_SET,
            Call::new().body(&params).done(),
        )
    }
}
