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

    /// `POST /v1/payment/refund` — refund a paid invoice, fully or partially. **Payout key.**
    ///
    /// Codes to branch on: `refund.nothing_to_refund`, `refund.exceeds_refundable`,
    /// `refund.no_address` (the payer address is not refundable — ask for one), `refund.dust`
    /// (below the network minimum), `refund.reference_collision`, `payout.insufficient_funds`
    /// (retryable), `merchant.wrong_key_kind`.
    pub fn create(&self, params: PaymentRefundRequest) -> RequestBuilder<Tr, Payout> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_REFUND,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/payment/resolve` — settle an underpaid (`wrong_amount`) invoice: keep what
    /// arrived, or send it back. **Payout key.** The answer's own `resolution` field says which
    /// branch the core took.
    ///
    /// Codes to branch on: `payment.not_found`, `payment.bad_status` (not `wrong_amount`),
    /// `refund.nothing_to_refund`, `refund.no_address`, `refund.exceeds_excess`,
    /// `merchant.wrong_key_kind`.
    pub fn resolve(&self, params: PaymentResolveRequest) -> RequestBuilder<Tr, Resolution> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_RESOLVE,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/refund/batch` — ASYNCHRONOUS batch (**≤ 5000**); track with `batches().info()`.
    /// **Payout key.** Codes to branch on: `batch.too_large`, `batch.empty`,
    /// `batch.reference_required`, `batch.duplicate_reference`, `batch.invoice_required`,
    /// `merchant.wrong_key_kind`.
    pub fn batch(&self, params: RefundBatchRequest) -> RequestBuilder<Tr, BatchSubmitted> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_REFUND_BATCH,
            Call::new().body(&params).done(),
        )
    }
}
