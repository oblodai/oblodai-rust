//! Payouts, refunds, transfers and the fee configuration they are priced by.

use crate::contract::enums::{FeeBearerResult, Network, PayoutStatus};
use crate::contract::models::common::{Money, Timestamp};

/// Payout as `/v1/payout`, `/info`, `/history`, `/cancel`, mass/batch elements and refunds render
/// it (core `PayoutResult`). `error`/`error_code` appear on `info` for failed payouts.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Payout {
    pub uuid: String,
    /// Merchant reference; null for refunds (they are keyed by `reference`/`refund_for`).
    pub order_id: Option<String>,
    pub status: PayoutStatus,
    pub is_final: bool,
    pub amount: Money,
    pub currency: String,
    pub network: Network,
    pub address: String,
    pub memo: String,
    /// Total debited from the balance (amount plus commission when the merchant bears the fee).
    pub payer_amount: Money,
    pub commission: Money,
    pub fee_bearer: FeeBearerResult,
    /// Balance the payout was funded from (`business`, `personal`, …).
    pub source: String,
    pub approval_required: bool,
    pub is_refund: bool,
    /// For refunds: the invoice being refunded.
    pub refund_for: Option<String>,
    pub payment_order_id: Option<String>,
    pub txid: String,
    pub document_url: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    /// `info` only, on failed payouts.
    #[serde(
        default,
        deserialize_with = "crate::contract::models::common::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub error: Option<Option<String>>,
    /// `info` only, on failed payouts.
    #[serde(
        default,
        deserialize_with = "crate::contract::models::common::double_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub error_code: Option<Option<String>>,
    /// Set on refunds of blocked static-wallet deposits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wallet_uuid: Option<String>,
}

/// `/v1/payout/calculate`. Amounts are null when the asset cannot be priced right now.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutCalculation {
    pub amount: Option<Money>,
    pub currency: String,
    pub network: Network,
    pub commission: Option<Money>,
    pub payer_amount: Option<Money>,
    pub fee_bearer: FeeBearerResult,
    pub fee_type: String,
}

/// `/v1/payout/validate` — the dry run; errors are the same the create call would raise.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutValidation {
    pub valid: bool,
    pub amount: Money,
    pub currency: String,
    pub network: Network,
    pub commission: Money,
    pub payer_amount: Money,
    pub fee_bearer: FeeBearerResult,
    /// Which balance would fund it (`business`/`personal`), when reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub funded_by: Option<String>,
    /// Non-empty when part of the balance is still maturing (reorg window).
    pub maturity_note: String,
}

/// `/v1/transfer/to-personal`: business → the owner's personal balance.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TransferToPersonal {
    pub uuid: String,
    pub currency: String,
    pub amount: Money,
    /// Always `"to_personal"`.
    pub direction: String,
    /// Personal balance after the transfer.
    pub personal_balance: Money,
    pub document_url: String,
}

/// `/v1/transfer/to-user`: business → another user's personal balance.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TransferToUser {
    pub uuid: String,
    pub currency: String,
    pub amount: Money,
    pub to_user_id: String,
    pub document_url: String,
}

/// Either transfer shape, told apart by the fields present.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Transfer {
    ToPersonal(TransferToPersonal),
    ToUser(TransferToUser),
}

/// `/v1/payout/fee-config/*`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutFeeConfig {
    pub fee_on_recipient: bool,
    /// `get` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configured: Option<bool>,
}

/// `/v1/payout/refund-fee-config/*`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RefundFeeConfig {
    pub fee_on_customer: bool,
    /// `get` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configured: Option<bool>,
}

/// `/v1/payment/fee-config/*`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentFeeConfig {
    pub payer_pays_percent: i64,
    /// `get` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}
