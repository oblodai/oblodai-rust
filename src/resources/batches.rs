//! Progress of asynchronous batches, and the internal transfers between platform balances.

use super::base::{Call, RequestBuilder};
use crate::contract::models::{BatchInfo, BatchSubmitted, TransferToPersonal, TransferToUser};
use crate::contract::requests::{
    BatchInfoRequest, TransferBatchRequest, TransferToPersonalRequest, TransferToUserRequest,
};
use crate::contract::routes;

/// The `batches` namespace.
#[derive(Clone, Debug)]
pub struct Batches<Tr> {
    transport: Tr,
}

impl<Tr: Clone> Batches<Tr> {
    pub(crate) fn new(transport: Tr) -> Self {
        Self { transport }
    }

    /// `POST /v1/batch/info` — status, counters and per-row outcomes.
    ///
    /// Signed with the merchant's API key, like every other batch route.
    pub fn info(&self, params: BatchInfoRequest) -> RequestBuilder<Tr, BatchInfo> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_BATCH_INFO,
            Call::new().body(&params).done(),
        )
    }
}

/// Internal, instant, fee-free moves between platform balances.
#[derive(Clone, Debug)]
pub struct Transfers<Tr> {
    transport: Tr,
}

impl<Tr: Clone> Transfers<Tr> {
    pub(crate) fn new(transport: Tr) -> Self {
        Self { transport }
    }

    /// `POST /v1/transfer/to-personal` — business balance → the owner's personal wallet (needs an
    /// owner link).
    /// Codes to branch on: `transfer.bad_amount`, `merchant.no_owner`,
    /// `merchant.no_personal_wallet`, `payout.insufficient_funds` (retryable),
    /// `payout.funds_maturing` (retryable).
    pub fn to_personal(
        &self,
        params: TransferToPersonalRequest,
    ) -> RequestBuilder<Tr, TransferToPersonal> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_TRANSFER_TO_PERSONAL,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/transfer/to-user` — business balance → another platform user's personal wallet.
    /// `amount` and `currency` are required.
    /// Codes to branch on: `transfer.bad_amount`, `transfer.no_recipient`,
    /// `transfer.recipient_not_found`, `transfer.bad_recipient` (the recipient is yourself),
    /// `payout.insufficient_funds` (retryable).
    pub fn to_user(&self, params: TransferToUserRequest) -> RequestBuilder<Tr, TransferToUser> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_TRANSFER_TO_USER,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/transfer/batch` — ASYNCHRONOUS batch (**≤ 5000**) of `to_user` transfers; poll
    /// `batches().info()`. `order_id` is required on every item.
    /// Codes to branch on: `batch.too_large`, `batch.empty`, `batch.order_id_required`,
    /// `batch.duplicate_order_id`, `batch.bad_recipient`.
    pub fn batch(&self, params: TransferBatchRequest) -> RequestBuilder<Tr, BatchSubmitted> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_TRANSFER_BATCH,
            Call::new().body(&params).done(),
        )
    }
}
