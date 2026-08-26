//! Merchant-level configuration exposed over the API.

use serde_json::json;

use super::base::{Call, RequestBuilder};
use crate::contract::models::{
    AcceptedMethod, AccuracyConfig, ApiAllowlist, AutoRefundConfig, AutoWithdrawRule, DiscountRule,
    OkResult, PaymentFeeConfig,
};
use crate::contract::requests::{
    AutoWithdrawSetRequest, PaymentAcceptedListRequest, PaymentAcceptedSetRequest,
    PaymentAccuracySetRequest, PaymentAutorefundSetRequest, PaymentDiscountListRequest,
    PaymentDiscountSetRequest, PaymentFeeConfigSetRequest,
};
use crate::contract::routes;
use crate::core::envelope::PlainList;
use crate::core::pagination::Pager;

/// The `settings` namespace.
#[derive(Clone, Debug)]
pub struct Settings<Tr> {
    transport: Tr,
}

impl<Tr: Clone> Settings<Tr> {
    pub(crate) fn new(transport: Tr) -> Self {
        Self { transport }
    }

    /// `POST /v1/payment/discount/set` — payer-facing discount/markup per currency+network.
    pub fn set_discount(
        &self,
        params: PaymentDiscountSetRequest,
    ) -> RequestBuilder<Tr, DiscountRule> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_DISCOUNT_SET,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/payment/discount/list`.
    pub fn list_discounts(&self, params: PaymentDiscountListRequest) -> Pager<Tr, DiscountRule> {
        Pager::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_DISCOUNT_LIST,
            super::base::to_value(&params),
        )
    }

    /// `POST /v1/payment/accuracy/get` — under/overpayment tolerance.
    pub fn get_accuracy(&self) -> RequestBuilder<Tr, AccuracyConfig> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_ACCURACY_GET,
            Call::new().done(),
        )
    }

    /// `POST /v1/payment/accuracy/set`.
    pub fn set_accuracy(
        &self,
        params: PaymentAccuracySetRequest,
    ) -> RequestBuilder<Tr, AccuracyConfig> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_ACCURACY_SET,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/payment/autorefund/get`.
    pub fn get_auto_refund(&self) -> RequestBuilder<Tr, AutoRefundConfig> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_AUTOREFUND_GET,
            Call::new().done(),
        )
    }

    /// `POST /v1/payment/autorefund/set` — refund over/underpayments automatically.
    pub fn set_auto_refund(
        &self,
        params: PaymentAutorefundSetRequest,
    ) -> RequestBuilder<Tr, AutoRefundConfig> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_AUTOREFUND_SET,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/payment/accepted/list` — which currency/network pairs invoices may be paid in.
    pub fn list_accepted(&self, params: PaymentAcceptedListRequest) -> Pager<Tr, AcceptedMethod> {
        Pager::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_ACCEPTED_LIST,
            super::base::to_value(&params),
        )
    }

    /// `POST /v1/payment/accepted/set`.
    pub fn set_accepted(&self, params: PaymentAcceptedSetRequest) -> RequestBuilder<Tr, OkResult> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_ACCEPTED_SET,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/payment/fee-config/get` — share of the network fee charged to the payer.
    pub fn get_payment_fee_config(&self) -> RequestBuilder<Tr, PaymentFeeConfig> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_FEE_CONFIG_GET,
            Call::new().done(),
        )
    }

    /// `POST /v1/payment/fee-config/set`.
    pub fn set_payment_fee_config(
        &self,
        params: PaymentFeeConfigSetRequest,
    ) -> RequestBuilder<Tr, PaymentFeeConfig> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_PAYMENT_FEE_CONFIG_SET,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/auto-withdraw/list`. Payout key.
    pub fn list_auto_withdraw(&self) -> RequestBuilder<Tr, PlainList<AutoWithdrawRule>> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_AUTO_WITHDRAW_LIST,
            Call::new().done(),
        )
    }

    /// `POST /v1/auto-withdraw/set` — sweep a currency to an address once the balance passes
    /// `min_amount`. **Payout key** — a payment key is refused with `merchant.wrong_key_kind`.
    pub fn set_auto_withdraw(
        &self,
        params: AutoWithdrawSetRequest,
    ) -> RequestBuilder<Tr, PlainList<AutoWithdrawRule>> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_AUTO_WITHDRAW_SET,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/auto-withdraw/delete`. **Payout key** — a payment key is refused with
    /// `merchant.wrong_key_kind`.
    pub fn delete_auto_withdraw(
        &self,
        currency: impl Into<String>,
    ) -> RequestBuilder<Tr, PlainList<AutoWithdrawRule>> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_AUTO_WITHDRAW_DELETE,
            Call::new()
                .json(json!({ "currency": currency.into() }))
                .done(),
        )
    }

    /// `POST /v1/api-allowlist/list` — source IPs allowed to use the API keys. Payout key.
    pub fn list_api_allowlist(&self) -> RequestBuilder<Tr, ApiAllowlist> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_API_ALLOWLIST_LIST,
            Call::new().done(),
        )
    }

    /// `POST /v1/api-allowlist/add`. **Payout key** — a payment key is refused with
    /// `merchant.wrong_key_kind`.
    pub fn add_api_allowlist(&self, cidr: impl Into<String>) -> RequestBuilder<Tr, ApiAllowlist> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_API_ALLOWLIST_ADD,
            Call::new().json(json!({ "cidr": cidr.into() })).done(),
        )
    }

    /// `POST /v1/api-allowlist/remove`. **Payout key** — a payment key is refused with
    /// `merchant.wrong_key_kind`.
    pub fn remove_api_allowlist(
        &self,
        cidr: impl Into<String>,
    ) -> RequestBuilder<Tr, ApiAllowlist> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_API_ALLOWLIST_REMOVE,
            Call::new().json(json!({ "cidr": cidr.into() })).done(),
        )
    }

    /// `POST /v1/api-allowlist/enable` — switch enforcement on or off (the list is kept).
    /// **Payout key** — a payment key is refused with `merchant.wrong_key_kind`. Locking yourself
    /// out is possible: make sure the calling host is on the list first.
    pub fn enable_api_allowlist(&self, enabled: bool) -> RequestBuilder<Tr, ApiAllowlist> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_API_ALLOWLIST_ENABLE,
            Call::new().json(json!({ "enabled": enabled })).done(),
        )
    }
}
