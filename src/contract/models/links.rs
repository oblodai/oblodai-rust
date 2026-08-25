//! Payout links (cheques) and payment links, merchant- and payer-facing.

use crate::contract::enums::{AmountMode, FeeBearer, Network, PaymentStatus, PayoutLinkStatus};
use crate::contract::models::common::{Money, Timestamp};

/// Payout link (cheque) as `/v1/payout/link`, `/info`, `/list`, `/cancel` and batch elements render it.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutLink {
    pub link_id: String,
    pub status: PayoutLinkStatus,
    pub amount: Money,
    pub currency: String,
    pub network: Network,
    /// Null while the asset cannot be priced.
    pub commission: Option<Money>,
    /// Null while the asset cannot be priced.
    pub payer_amount: Option<Money>,
    pub fee_bearer: FeeBearer,
    pub fee_type: String,
    pub reference: String,
    pub title: String,
    pub note: String,
    pub passcode_protected: bool,
    pub expires_at: Timestamp,
    pub created_at: Timestamp,
    /// Present on create and batch-create only — the secret the recipient claims with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_token: Option<String>,
    /// Present on create and batch-create only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_url: Option<String>,
    /// Present on batch-create only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<String>,
    /// Set once claimed: the payout that paid the recipient.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payout_id: Option<String>,
    /// Set once claimed: the address the recipient claimed to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_address: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// The generated passcode, shown once on create when `passcode: "auto"` was requested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passcode: Option<String>,
}

/// `GET /v1/claim/{token}` — what the recipient sees before claiming.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ClaimPreview {
    pub status: PayoutLinkStatus,
    pub claimable: bool,
    pub amount: Money,
    pub currency: String,
    pub network: Network,
    /// Null while the asset cannot be priced.
    pub commission: Option<Money>,
    /// Null while the asset cannot be priced.
    pub payer_amount: Option<Money>,
    pub fee_bearer: FeeBearer,
    pub fee_type: String,
    pub title: String,
    pub note: String,
    pub expires_at: Timestamp,
}

/// `POST /v1/claim/{token}` — the payout minted by a claim.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ClaimResult {
    /// The payout that pays the recipient (`payouts.info({ uuid: payout_id })`).
    pub payout_id: String,
    pub status: PayoutLinkStatus,
    pub address: String,
    pub amount: Money,
    pub currency: String,
    pub network: Network,
    /// Null while the asset cannot be priced.
    pub commission: Option<Money>,
    /// Null while the asset cannot be priced.
    pub payer_amount: Option<Money>,
    pub fee_bearer: FeeBearer,
    pub fee_type: String,
}

/// An invoice spawned by a payment link, as `/v1/payment/link/info` lists it.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentLinkPayment {
    pub uuid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    pub amount: Money,
    pub currency: String,
    pub status: PaymentStatus,
    pub created_at: Timestamp,
}

/// Payment link as `/v1/payment/link/info` and `/list` render it. Amount fields depend on `amount_mode`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentLink {
    pub link_id: String,
    pub url: String,
    pub active: bool,
    pub title: String,
    pub description: String,
    pub amount_mode: AmountMode,
    pub currency: String,
    /// `fixed` links.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_fixed: Option<Money>,
    /// `range` links.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_amount: Option<Money>,
    /// `range` links.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_amount: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_currency: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_network: Option<Network>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<Timestamp>,
    pub document_url: String,
    pub created_at: Timestamp,
    /// `info` only: invoices spawned by this link.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payments: Option<Vec<PaymentLinkPayment>>,
}

/// `POST /v1/payment/link` acknowledgement.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentLinkCreated {
    pub link_id: String,
    pub url: String,
    pub document_url: String,
}

/// `POST /v1/payment/link/toggle` — the link's new state.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentLinkToggled {
    pub link_id: String,
    pub active: bool,
}

/// `GET /v1/link/{id}` — the payer-facing view.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PublicPaymentLink {
    pub link_id: String,
    pub title: String,
    pub description: String,
    pub amount_mode: AmountMode,
    pub currency: String,
    /// `fixed` links.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_fixed: Option<Money>,
    /// `range` links.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_amount: Option<Money>,
    /// `range` links.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_amount: Option<Money>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_currency: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_network: Option<Network>,
}
