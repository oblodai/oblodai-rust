//! Payout links (cheques) and reusable payment links.

use serde_json::json;

use super::base::{Call, RequestBuilder};
use super::files::FileBuilder;
use super::refs::{IdRef, PageParams};
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

/// Payout links: funds reserved now, claimed later by whoever holds the token.
#[derive(Clone, Debug)]
pub struct PayoutLinks<Tr> {
    transport: Tr,
}

impl<Tr: Clone> PayoutLinks<Tr> {
    pub(crate) fn new(transport: Tr) -> Self {
        Self { transport }
    }

    /// `POST /v1/payout/link` — reserve funds and mint a claim token (`claim_token`/`claim_url` are
    /// returned once). Idempotent by `reference`.
    ///
    /// Codes to branch on: `payoutlink.insufficient_funds` (retryable), `payoutlink.funds_maturing`
    /// (retryable), `payoutlink.disabled`, `payoutlink.bad_amount`,
    /// `payoutlink.duplicate_reference` (that `reference` already minted a different link),
    /// `payoutlink.reference_required`, `idempotency.key_reused`.
    pub fn create(&self, params: PayoutLinkRequest) -> RequestBuilder<Tr, PayoutLink> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_LINK,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/payout/link/info` — by link id, or by a [`PayoutLink`] you already hold.
    pub fn info(&self, link: impl Into<IdRef>) -> RequestBuilder<Tr, PayoutLink> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_LINK_INFO,
            Call::new()
                .json(json!({ "link_id": link.into().into_string() }))
                .done(),
        )
    }

    /// Alias of [`info`](Self::info), same signature.
    pub fn get(&self, link: impl Into<IdRef>) -> RequestBuilder<Tr, PayoutLink> {
        self.info(link)
    }

    /// `POST /v1/payout/link/list`.
    pub fn list(&self, params: PayoutLinkListRequest) -> Pager<Tr, PayoutLink> {
        Pager::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_LINK_LIST,
            super::base::to_value(&params),
        )
    }

    /// `POST /v1/payout/link/cancel` — release the reserved funds of an unclaimed link. Codes to
    /// branch on: `payoutlink.not_found`, `payoutlink.bad_state`, `payoutlink.already_claimed`,
    /// `payoutlink.claim_in_progress`, `payoutlink.cancelled`.
    pub fn cancel(&self, link: impl Into<IdRef>) -> RequestBuilder<Tr, PayoutLink> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYOUT_LINK_CANCEL,
            Call::new()
                .json(json!({ "link_id": link.into().into_string() }))
                .done(),
        )
    }

    /// `POST /v1/payout/link/batch` — SYNCHRONOUS: many links in one signed call (**≤ 500 items**),
    /// per-element outcomes. `reference` is required on every item.
    ///
    /// Call-level codes to branch on: `payoutlink.batch_too_large` (> 500),
    /// `payoutlink.empty_batch`, `payoutlink.disabled`, `payoutlink.insufficient_funds`
    /// (retryable), `payoutlink.idempotency_required`. A per-element failure arrives inside the 200
    /// as `items[].ok == false` — check every element.
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

    /// `POST /v1/payout/link/cheque` — printable PDF cheque for a claim token. Codes to branch on:
    /// `cheque.token_required`, `payoutlink.not_found`, `payoutlink.token`.
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
    /// No credentials needed — this route is `auth: public`, like `claim_preview`.
    ///
    /// Codes to branch on: `payoutlink.not_found`, `payoutlink.expired`,
    /// `payoutlink.already_claimed`, `payoutlink.claim_in_progress`,
    /// `payoutlink.passcode_required`, `payoutlink.passcode_wrong`, `payoutlink.passcode_locked`,
    /// `payoutlink.no_address`, `payoutlink.unsupported_network`, `request.rate_limited`.
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

    /// `POST /v1/payment/link` — mint a reusable link. **Payment key.**
    ///
    /// Codes to branch on: `paylink.bad_mode`, `paylink.amount_required`, `paylink.bad_amount`,
    /// `paylink.bad_bounds`, `paylink.bad_range`, `paylink.order_id_invalid`,
    /// `paylink.unavailable`.
    pub fn create(&self, params: PaymentLinkRequest) -> RequestBuilder<Tr, PaymentLinkCreated> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_LINK,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/payment/link/info` — the link plus a page of the invoices it spawned
    /// (`payments`); `page` pages that inner list.
    ///
    /// ```no_run
    /// # async fn demo(client: &oblodai::Client) -> oblodai::Result<()> {
    /// use oblodai::PageParams;
    /// let link = client.payment_links().info("lnk_1", PageParams::default().limit(100)).await?;
    /// # let _ = link; Ok(()) }
    /// ```
    pub fn info(
        &self,
        link: impl Into<IdRef>,
        page: PageParams,
    ) -> RequestBuilder<Tr, PaymentLink> {
        let mut body = json!({ "link_id": link.into().into_string() });
        if let Some(limit) = page.limit {
            body["limit"] = json!(limit);
        }
        if let Some(offset) = page.offset {
            body["offset"] = json!(offset);
        }
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_LINK_INFO,
            Call::new().json(body).done(),
        )
    }

    /// Alias of [`info`](Self::info), same signature.
    pub fn get(&self, link: impl Into<IdRef>, page: PageParams) -> RequestBuilder<Tr, PaymentLink> {
        self.info(link, page)
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
        link: impl Into<IdRef>,
        active: bool,
    ) -> RequestBuilder<Tr, PaymentLinkToggled> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_LINK_TOGGLE,
            Call::new()
                .json(json!({ "link_id": link.into().into_string(), "active": active }))
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
    ///
    /// Codes to branch on: `paylink.not_found`, `paylink.disabled`, `paylink.amount_required`,
    /// `paylink.below_min`, `paylink.above_max`, `paylink.rate_limited`, `accepted.no_network`.
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
