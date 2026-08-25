//! Payout links (cheques) and reusable payment links.

use serde_json::json;

use super::base::{Call, FileBuilder, RequestBuilder};
use crate::contract::models::{
    BatchElement, ClaimPreview, ClaimResult, PaymentLink, PaymentLinkCreated, PaymentLinkToggled,
    PayoutLink, PublicPayment, PublicPaymentLink,
};
use crate::contract::requests::{
    ClaimRequest, LinkCheckoutRequest, PaymentLinkListRequest, PaymentLinkRequest,
    PayoutLinkBatchRequest, PayoutLinkChequeRequest, PayoutLinkListRequest, PayoutLinkRequest,
};
use crate::contract::routes;
use crate::core::envelope::PlainList;
use crate::core::pagination::Pager;

/// Payout links: funds reserved now, claimed later by whoever holds the token. Payout key.
#[derive(Clone, Debug)]
pub struct PayoutLinks<Tr> {
    transport: Tr,
}

impl<Tr: Clone> PayoutLinks<Tr> {
    pub(crate) fn new(transport: Tr) -> Self {
        Self { transport }
    }

    /// `POST /v1/payout/link` — reserve funds and mint a claim token (`claim_token`/`claim_url`
    /// are returned once). Idempotent by `reference`.
    pub fn create(&self, params: PayoutLinkRequest) -> RequestBuilder<Tr, PayoutLink> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_LINK,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/payout/link/info`.
    pub fn info(&self, link_id: impl Into<String>) -> RequestBuilder<Tr, PayoutLink> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_LINK_INFO,
            Call::new()
                .json(json!({ "link_id": link_id.into() }))
                .done(),
        )
    }

    /// Alias of [`info`](Self::info).
    pub fn get(&self, link_id: impl Into<String>) -> RequestBuilder<Tr, PayoutLink> {
        self.info(link_id)
    }

    /// `POST /v1/payout/link/list`.
    pub fn list(&self, params: PayoutLinkListRequest) -> Pager<Tr, PayoutLink> {
        Pager::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_LINK_LIST,
            super::base::to_value(&params),
        )
    }

    /// `POST /v1/payout/link/cancel` — release the reserved funds of an unclaimed link.
    pub fn cancel(&self, link_id: impl Into<String>) -> RequestBuilder<Tr, PayoutLink> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_LINK_CANCEL,
            Call::new()
                .json(json!({ "link_id": link_id.into() }))
                .done(),
        )
    }

    /// `POST /v1/payout/link/batch` — SYNCHRONOUS: many links in one signed call, per-element
    /// outcomes. `reference` is required on every item.
    pub fn batch(
        &self,
        params: PayoutLinkBatchRequest,
    ) -> RequestBuilder<Tr, PlainList<BatchElement<PayoutLink>>> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_LINK_BATCH,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/payout/link/cheque` — printable PDF cheque for a claim token.
    pub fn cheque(&self, params: PayoutLinkChequeRequest) -> FileBuilder<Tr> {
        FileBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_LINK_CHEQUE,
            Call::new().body(&params).done(),
        )
    }

    // --- recipient side (public, unsigned) ---

    /// `GET /v1/claim/{token}` — what the recipient sees before claiming. No credentials needed.
    pub fn claim_preview(&self, token: impl Into<String>) -> RequestBuilder<Tr, ClaimPreview> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::GET_V1_CLAIM_TOKEN,
            Call::new().path("token", token).done(),
        )
    }

    /// `POST /v1/claim/{token}` — claim to an address (and passcode when the link has one).
    /// No credentials needed.
    pub fn claim(
        &self,
        token: impl Into<String>,
        params: ClaimRequest,
    ) -> RequestBuilder<Tr, ClaimResult> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_CLAIM_TOKEN,
            Call::new().path("token", token).body(&params).done(),
        )
    }
}

/// Reusable payment links (tip jars, price tags): each checkout spawns an invoice. Payment key.
#[derive(Clone, Debug)]
pub struct PaymentLinks<Tr> {
    transport: Tr,
}

impl<Tr: Clone> PaymentLinks<Tr> {
    pub(crate) fn new(transport: Tr) -> Self {
        Self { transport }
    }

    /// `POST /v1/payment/link`.
    pub fn create(&self, params: PaymentLinkRequest) -> RequestBuilder<Tr, PaymentLinkCreated> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_LINK,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/payment/link/info` — the link plus a page of the invoices it spawned
    /// (`payments`); `limit`/`offset` page that inner list.
    pub fn info(&self, link_id: impl Into<String>) -> RequestBuilder<Tr, PaymentLink> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_LINK_INFO,
            Call::new()
                .json(json!({ "link_id": link_id.into() }))
                .done(),
        )
    }

    /// Alias of [`info`](Self::info), with paging of the invoices the link spawned.
    pub fn get(
        &self,
        link_id: impl Into<String>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> RequestBuilder<Tr, PaymentLink> {
        let mut body = json!({ "link_id": link_id.into() });
        if let Some(limit) = limit {
            body["limit"] = json!(limit);
        }
        if let Some(offset) = offset {
            body["offset"] = json!(offset);
        }
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_LINK_INFO,
            Call::new().json(body).done(),
        )
    }

    /// `POST /v1/payment/link/list`.
    pub fn list(&self, params: PaymentLinkListRequest) -> Pager<Tr, PaymentLink> {
        Pager::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_LINK_LIST,
            super::base::to_value(&params),
        )
    }

    /// `POST /v1/payment/link/toggle` — enable or disable a link.
    pub fn toggle(
        &self,
        link_id: impl Into<String>,
        active: bool,
    ) -> RequestBuilder<Tr, PaymentLinkToggled> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_LINK_TOGGLE,
            Call::new()
                .json(json!({ "link_id": link_id.into(), "active": active }))
                .done(),
        )
    }

    // --- payer side (public, unsigned) ---

    /// `GET /v1/link/{id}` — the link as the payer sees it. No credentials needed.
    pub fn public_view(&self, link_id: impl Into<String>) -> RequestBuilder<Tr, PublicPaymentLink> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::GET_V1_LINK_ID,
            Call::new().path("id", link_id).done(),
        )
    }

    /// `POST /v1/link/{id}/checkout` — spawn an invoice from the link (rate-capped per IP).
    /// No credentials needed.
    pub fn checkout(
        &self,
        link_id: impl Into<String>,
        params: LinkCheckoutRequest,
    ) -> RequestBuilder<Tr, PublicPayment> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_LINK_ID_CHECKOUT,
            Call::new().path("id", link_id).body(&params).done(),
        )
    }
}
