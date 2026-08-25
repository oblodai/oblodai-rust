//! Balances, account-level facts, and the public reference catalogue.

use serde_json::json;

use super::base::{Call, RequestBuilder};
use crate::contract::models::{Balance, Currencies, ExchangeRate, ReferralInfo, VrcsStatus};
use crate::contract::requests::ExchangeRateListRequest;
use crate::contract::routes;
use crate::core::pagination::Pager;

/// The `account` namespace.
#[derive(Clone, Debug)]
pub struct Account<Tr> {
    transport: Tr,
}

impl<Tr: Clone> Account<Tr> {
    pub(crate) fn new(transport: Tr) -> Self {
        Self { transport }
    }

    /// `POST /v1/balance` — available balance per currency.
    pub fn balance(&self) -> RequestBuilder<Tr, Balance> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_BALANCE,
            Call::new().done(),
        )
    }

    /// `POST /v1/referral/info` — referral code, link and earnings.
    pub fn referral(&self) -> RequestBuilder<Tr, ReferralInfo> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_REFERRAL_INFO,
            Call::new().done(),
        )
    }

    /// `POST /v1/vrcs` — read the volatility-risk conversion setting (auto-convert volatile
    /// deposits to USDT).
    pub fn vrcs(&self) -> RequestBuilder<Tr, VrcsStatus> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_VRCS,
            Call::new().done(),
        )
    }

    /// `POST /v1/vrcs` — switch volatility-risk conversion on or off.
    pub fn set_vrcs(&self, enabled: bool) -> RequestBuilder<Tr, VrcsStatus> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_VRCS,
            Call::new().json(json!({ "enabled": enabled })).done(),
        )
    }
}

/// Public reference data — no credentials needed.
#[derive(Clone, Debug)]
pub struct Catalog<Tr> {
    transport: Tr,
}

impl<Tr: Clone> Catalog<Tr> {
    pub(crate) fn new(transport: Tr) -> Self {
        Self { transport }
    }

    /// `GET /v1/currencies` — every asset, its networks and live availability.
    pub fn currencies(&self) -> RequestBuilder<Tr, Currencies> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::GET_V1_CURRENCIES,
            Call::new().done(),
        )
    }

    /// `POST /v1/exchange-rate/list` — current rates, optionally filtered by `currency_from` /
    /// `currency_to`.
    pub fn exchange_rates(&self, params: ExchangeRateListRequest) -> Pager<Tr, ExchangeRate> {
        Pager::new(
            self.transport.clone(),
            &routes::POST_V1_EXCHANGE_RATE_LIST,
            super::base::to_value(&params),
        )
    }
}
