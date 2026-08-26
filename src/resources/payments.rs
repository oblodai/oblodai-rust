//! Invoices: create, look up, cancel, list, and the payer-facing checkout endpoints. Payment key.

use super::base::{Call, RequestBuilder};
use super::refs::PaymentLookup;
use crate::contract::models::{
    BatchSubmitted, EmailSent, OkResult, Payment, PublicPayment, QrCode, ServiceMethod,
};
use crate::contract::requests::{
    PaySelectRequest, PaymentBatchRequest, PaymentHistoryRequest, PaymentRequest,
    PaymentSendEmailRequest, PaymentServicesRequest,
};
use crate::contract::routes;
use crate::core::pagination::Pager;

/// The `payments` namespace.
#[derive(Clone, Debug)]
pub struct Payments<Tr> {
    transport: Tr,
}

impl<Tr: Clone> Payments<Tr> {
    pub(crate) fn new(transport: Tr) -> Self {
        Self { transport }
    }

    /// `POST /v1/payment` — create an invoice. Idempotent by `order_id` and by `Idempotency-Key`.
    /// **Payment key.**
    ///
    /// Codes to branch on: `invoice.bad_price`, `payment.bad_amount`, `payment.below_minimum`,
    /// `payment.unsupported_network`, `payment.network_required`, `accepted.no_network`,
    /// `request.unknown_currency`, `idempotency.key_reused`, `request.rate_limited`.
    pub fn create(&self, params: PaymentRequest) -> RequestBuilder<Tr, Payment> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/payment/info` — by `uuid` or `order_id`; includes `refunds` and `refund_status`.
    pub fn info(&self, lookup: impl Into<PaymentLookup>) -> RequestBuilder<Tr, Payment> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_INFO,
            Call::new().body(&lookup.into()).done(),
        )
    }

    /// Alias of [`info`](Self::info).
    pub fn get(&self, lookup: impl Into<PaymentLookup>) -> RequestBuilder<Tr, Payment> {
        self.info(lookup)
    }

    /// `POST /v1/payment/cancel` — cancel an unpaid invoice (409 `invoice.not_payable` once a
    /// deposit was seen).
    pub fn cancel(&self, lookup: impl Into<PaymentLookup>) -> RequestBuilder<Tr, Payment> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_CANCEL,
            Call::new().body(&lookup.into()).done(),
        )
    }

    /// `POST /v1/payment/history` — newest first. `await` for one page, `.stream()` for every item.
    pub fn history(&self, params: PaymentHistoryRequest) -> Pager<Tr, Payment> {
        Pager::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_HISTORY,
            super::base::to_value(&params),
        )
    }

    /// Alias of [`history`](Self::history).
    pub fn list(&self, params: PaymentHistoryRequest) -> Pager<Tr, Payment> {
        self.history(params)
    }

    /// `POST /v1/payment/batch` — create up to 5000 invoices asynchronously; track with
    /// `batches().info()`.
    pub fn batch(&self, params: PaymentBatchRequest) -> RequestBuilder<Tr, BatchSubmitted> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_BATCH,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/payment/qr` — QR image of the invoice's payment URI.
    pub fn qr(&self, lookup: impl Into<PaymentLookup>) -> RequestBuilder<Tr, QrCode> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_QR,
            Call::new().body(&lookup.into()).done(),
        )
    }

    /// `POST /v1/payment/services` — currencies/networks accepted for deposits, with limits and
    /// fees.
    pub fn services(&self, params: PaymentServicesRequest) -> Pager<Tr, ServiceMethod> {
        Pager::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_SERVICES,
            super::base::to_value(&params),
        )
    }

    /// `POST /v1/payment/send-email` — email the receipt (defaults to the invoice's `payer_email`).
    pub fn send_email(&self, params: PaymentSendEmailRequest) -> RequestBuilder<Tr, EmailSent> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_SEND_EMAIL,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/payment/resend` — re-deliver the invoice's last webhook.
    pub fn resend(&self, lookup: impl Into<PaymentLookup>) -> RequestBuilder<Tr, OkResult> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_RESEND,
            Call::new().body(&lookup.into()).done(),
        )
    }

    // --- payer-facing (public, unsigned) — for custom checkout pages ---

    /// `GET /v1/pay/{id}` — the invoice as the payer sees it. No credentials needed.
    pub fn public_view(&self, uuid: impl Into<String>) -> RequestBuilder<Tr, PublicPayment> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::GET_V1_PAY_ID,
            Call::new().path("id", uuid).done(),
        )
    }

    /// `POST /v1/pay/{id}/select` — pick the asset/network on a multi-currency invoice.
    /// No credentials needed.
    pub fn select(
        &self,
        uuid: impl Into<String>,
        params: PaySelectRequest,
    ) -> RequestBuilder<Tr, PublicPayment> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAY_ID_SELECT,
            Call::new().path("id", uuid).body(&params).done(),
        )
    }

    /// `GET /v1/pay/{id}/qr` — QR for the payer page. No credentials needed.
    pub fn public_qr(&self, uuid: impl Into<String>) -> RequestBuilder<Tr, QrCode> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::GET_V1_PAY_ID_QR,
            Call::new().path("id", uuid).done(),
        )
    }
}
