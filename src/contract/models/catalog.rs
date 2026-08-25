//! The asset catalogue and exchange rates.

use crate::contract::enums::Network;

/// One network a currency lives on.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CurrencyNetwork {
    pub network: Network,
    /// `native` or `token`.
    pub kind: String,
    /// Token contract address, for tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract: Option<String>,
    pub min_confirmations: i64,
    /// Deposits and payouts both possible right now.
    pub available: bool,
    pub deposit_available: bool,
    pub payout_available: bool,
    /// The network offered first on the pay page.
    pub default_offer: bool,
}

/// One payable asset and the networks it settles on.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CurrencyInfo {
    pub currency: String,
    pub decimals: i64,
    pub networks: Vec<CurrencyNetwork>,
}

/// A currency an invoice may be priced in (crypto or fiat).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PricingCurrency {
    pub currency: String,
    pub decimals: i64,
    pub fiat: bool,
}

/// `GET /v1/currencies`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Currencies {
    pub currencies: Vec<CurrencyInfo>,
    pub pricing_currencies: Vec<PricingCurrency>,
}

/// `/v1/exchange-rate/list` item: 1 `from` = `course` `to`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ExchangeRate {
    pub from: String,
    pub to: String,
    pub course: String,
}
