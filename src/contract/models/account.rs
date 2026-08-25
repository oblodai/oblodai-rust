//! Account-wide state: balances, referrals, static wallets, allowlists and merchant settings.

use crate::contract::enums::Network;
use crate::contract::models::common::{Money, Timestamp};

/// One asset's balance, as `/v1/balance` reports it.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BalanceEntry {
    pub currency: String,
    /// Available (spendable) balance.
    pub balance: Money,
}

/// Balances grouped by the owner of the funds.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BalanceByOwner {
    /// The merchant's own (business) balances.
    pub merchant: Vec<BalanceEntry>,
}

/// `/v1/balance`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Balance {
    pub balance: BalanceByOwner,
}

/// The trailing-week slice of `/v1/referral/info`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReferralWeek {
    pub referred_count: i64,
    pub earnings_by_asset: std::collections::BTreeMap<String, Money>,
}

/// `/v1/referral/info`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReferralInfo {
    pub code: String,
    pub link: String,
    /// Referral tiers, basis points.
    pub tier_bps: Vec<i64>,
    pub referred_count: i64,
    pub earnings_by_asset: std::collections::BTreeMap<String, Money>,
    pub week: ReferralWeek,
}

/// `/v1/vrcs` — volatility risk control (auto-convert volatile deposits to USDT).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VrcsStatus {
    pub enabled: bool,
}

/// Static (permanent) deposit wallet — `/v1/wallet`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Wallet {
    pub uuid: String,
    pub address: String,
    pub network: Network,
    pub currency: String,
    pub order_id: String,
    /// Hosted page showing the address and QR.
    pub url: String,
    pub document_url: String,
    /// XRP destination tag, when the network needs one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination_tag: Option<String>,
    /// TON and Stellar memo, when the network needs one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address_xaddress: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address_muxed: Option<String>,
}

/// `/v1/wallet/block`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WalletBlocked {
    pub uuid: String,
    pub address: String,
    pub blocked: bool,
}

/// `/v1/wallet/qr` — the address QR as a data URI.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WalletQr {
    /// `data:image/png;base64,…`
    pub image: String,
}

/// `/v1/auto-withdraw/*` entry.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AutoWithdrawRule {
    pub currency: String,
    pub network: Network,
    pub address: String,
    pub min_amount: Money,
}

/// `/v1/api-allowlist/*` — entries are CIDRs.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ApiAllowlist {
    pub enabled: bool,
    pub items: Vec<String>,
}

/// A per-method price adjustment offered on the pay page.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DiscountRule {
    pub currency: String,
    pub network: Network,
    /// Positive = discount for the payer, negative = markup.
    pub discount_percent: i64,
}

/// `/v1/payment/accuracy/*` — how much underpayment still counts as paid.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AccuracyConfig {
    pub enabled: bool,
    pub accuracy_percent: i64,
}

/// `/v1/payment/autorefund/*` — automatic refunds of over- and underpayments.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AutoRefundConfig {
    pub overpay: bool,
    pub underpay: bool,
    /// `get` only: whether the merchant ever set it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configured: Option<bool>,
}

/// `/v1/payment/accepted/*` — a currency/network pair the merchant accepts.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AcceptedMethod {
    pub currency: String,
    pub network: Network,
    pub available: bool,
    /// Why it is unavailable, when it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// `/v1/split/rule` and `/v1/split/rule/list` items.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SplitRule {
    pub rule_id: String,
    /// Share of every payment, percent, as a decimal string.
    pub percent: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    /// Set for on-platform partner rules (reversible on refund).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merchant_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reversible: Option<bool>,
}

/// `/v1/split/config/*` — how long a split payout is held so a refund can reverse it.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SplitConfig {
    pub refund_hold_seconds: i64,
}

/// `/v1/split/recipient/optin*` — whether this account accepts on-platform split shares.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SplitOptIn {
    pub enabled: bool,
}

/// The reporting window a `DocumentJob` covers.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DocumentPeriod {
    pub from: String,
    pub to: String,
}

/// The rendered artefact of a finished `DocumentJob`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DocumentFile {
    /// A signed link; or fetch the bytes with `documents.jobFile`.
    pub download_url: String,
    pub expires_at: Timestamp,
    pub rows: i64,
    pub size_bytes: i64,
}

/// `/v1/documents/jobs` and `/jobs/info`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DocumentJob {
    pub job_id: String,
    pub kind: String,
    pub format: String,
    pub lang: String,
    /// `queued`, `processing`, `done` or `failed`.
    pub status: String,
    pub period: DocumentPeriod,
    /// Human hint while queued (e.g. "15s").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ready_within: Option<String>,
    /// Set once done.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<DocumentFile>,
    /// Set once failed.
    #[serde(
        default,
        deserialize_with = "crate::contract::models::common::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub error: Option<Option<String>>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
