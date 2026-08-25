//! Refunds are payouts in the invoice's own asset; underpayments are resolved (accept or refund).
//! Every route here needs the payout key.

use super::base::{Call, RequestBuilder};
use crate::contract::models::{BatchSubmitted, Payout, Resolution};
use crate::contract::requests::{PaymentRefundRequest, PaymentResolveRequest, RefundBatchRequest};
use crate::contract::routes;

/// The `refunds` namespace.
#[derive(Clone, Debug)]
pub struct Refunds<Tr> {
    transport: Tr,
}

impl<Tr: Clone> Refunds<Tr> {
    pub(crate) fn new(transport: Tr) -> Self {
        Self { transport }
    }

    /// `POST /v1/payment/refund` — refund a paid invoice, fully or partially.
    pub fn create(&self, params: PaymentRefundRequest) -> RequestBuilder<Tr, Payout> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_REFUND,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/payment/resolve` — settle an underpaid (`wrong_amount`) invoice: keep what
    /// arrived, or send it back.
    pub fn resolve(&self, params: PaymentResolveRequest) -> RequestBuilder<Tr, Resolution> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_RESOLVE,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/refund/batch` — up to 5000 refunds; track with `batches().info()`.
    pub fn batch(&self, params: RefundBatchRequest) -> RequestBuilder<Tr, BatchSubmitted> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_REFUND_BATCH,
            Call::new().body(&params).done(),
        )
    }
}
