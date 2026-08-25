//! Invoices and everything the payment routes answer with.

use crate::contract::enums::{BatchOnError, FeeBearerResult, Network, PaymentStatus, PayoutStatus};
use crate::contract::models::common::{BatchKind, BatchStatus, Money, Timestamp};

/// One on-chain deposit attributed to an invoice.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentTx {
    pub txid: String,
    pub amount: Money,
    pub network: Network,
    pub height: i64,
    pub created_at: Timestamp,
}

/// A refund issued against an invoice (a payout in disguise; full detail via `payouts.info`).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentRefund {
    pub uuid: String,
    pub address: String,
    pub amount: Money,
    pub status: PayoutStatus,
    pub is_final: bool,
    pub txid: String,
    pub created_at: Timestamp,
}

/// Invoice as `/v1/payment`, `/v1/payment/info`, `/v1/payment/history` and `/v1/payment/cancel`
/// render it (core `paymentResult`). `refunds`/`refund_status` are present on `info` only.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Payment {
    pub uuid: String,
    pub order_id: String,
    pub status: PaymentStatus,
    pub is_final: bool,
    /// Priced amount in `currency`.
    pub amount: Money,
    pub currency: String,
    /// Settlement network; empty until the payer selects one on a multi-network invoice.
    pub network: Network,
    /// Amount due in the payer asset (`payer_currency`).
    pub payer_amount: Money,
    pub payer_currency: String,
    pub amount_paid: Money,
    pub amount_remaining: Money,
    pub address: String,
    /// XRP destination tag / Stellar memo / TON memo, when the network needs one.
    pub destination_tag: String,
    pub memo: String,
    pub address_xaddress: String,
    pub address_muxed: String,
    /// `data:image/png;base64,…` QR of the payment URI.
    pub address_qr_code: String,
    pub is_multi: bool,
    pub url: String,
    pub url_return: String,
    pub url_success: String,
    pub expired_at: Timestamp,
    pub rate_expires_at: Timestamp,
    pub exchange_rate: Money,
    pub confirmations: i64,
    pub required_confirmations: i64,
    pub txid: String,
    pub tx_list: Vec<PaymentTx>,
    pub paid_at: Option<Timestamp>,
    pub payer_address: String,
    pub payer_address_is_refundable: bool,
    pub payer_email: String,
    pub additional_data: String,
    pub commission: Money,
    pub merchant_amount: Money,
    pub document_url: String,
    pub is_test: bool,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    /// `info` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refunds: Option<Vec<PaymentRefund>>,
    /// `info` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refund_status: Option<String>,
}

/// The payer-facing view (`GET /v1/pay/{id}`, `/select`, link checkout): no merchant-only fields.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PublicPayment {
    pub uuid: String,
    pub order_id: String,
    pub status: PaymentStatus,
    pub is_final: bool,
    pub amount: Money,
    pub currency: String,
    pub network: Network,
    pub payer_amount: Money,
    pub payer_currency: String,
    pub amount_paid: Money,
    pub amount_remaining: Money,
    pub address: String,
    pub destination_tag: String,
    pub memo: String,
    pub address_xaddress: String,
    pub address_muxed: String,
    /// `data:image/png;base64,…` QR of the payment URI.
    pub address_qr_code: String,
    pub is_multi: bool,
    pub url: String,
    pub url_return: String,
    pub url_success: String,
    pub expired_at: Timestamp,
    pub rate_expires_at: Timestamp,
    pub confirmations: i64,
    pub required_confirmations: i64,
    pub txid: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// `/v1/payment/qr` and `GET /v1/pay/{id}/qr`. All fields are empty while the invoice has no real
/// address: sandbox invoices (synthetic `sandbox:` address) and `select` invoices awaiting a network.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QrCode {
    /// `data:image/png;base64,…`
    pub image: String,
    /// What the QR encodes: a payment URI when `is_uri`, else the bare address.
    pub payload: String,
    pub is_uri: bool,
    pub address: String,
}

/// `/v1/payment/resolve` with `action: "accept"` — the underpayment was kept as full settlement.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResolutionAccepted {
    /// Always `"accepted"`.
    pub resolution: String,
    pub payment_uuid: String,
    pub order_id: String,
    pub currency: String,
    pub amount_kept: Money,
}

/// `/v1/payment/resolve` with `action: "refund"` — the underpayment was sent back; the body is the
/// refund payout itself, plus `resolution: "refunded"`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResolutionRefunded {
    /// Always `"refunded"`.
    pub resolution: String,
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
    /// Balance the payout was funded from.
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
}

/// `/v1/payment/resolve` — either arm, told apart by the fields present.
// `Refunded` is a whole payout; the accepted arm is five fields. Boxing it would only move
// the size somewhere else, and callers match on the arm, not on its size.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Resolution {
    Accepted(ResolutionAccepted),
    Refunded(ResolutionRefunded),
}

impl Resolution {
    /// `"accepted"` or `"refunded"`, without matching.
    pub fn resolution(&self) -> &str {
        match self {
            Resolution::Accepted(r) => &r.resolution,
            Resolution::Refunded(r) => &r.resolution,
        }
    }
}

/// `/v1/payment/send-email`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EmailSent {
    pub ok: bool,
    pub email: String,
    pub uuid: String,
}

/// Limits of a `ServiceMethod`; both bounds are null when the asset cannot be priced right now.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ServiceLimit {
    /// The asset the bounds are expressed in, when the core reports it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    pub min_amount: Option<Money>,
    pub max_amount: Option<Money>,
}

/// Fee of a `ServiceMethod`. `fee_amount`/`percent` are null when that half does not apply.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ServiceCommission {
    pub currency: String,
    pub fee_amount: Option<Money>,
    pub percent: Option<String>,
    pub fee_type: String,
}

/// Item of `/v1/payment/services` and `/v1/payout/services`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ServiceMethod {
    pub currency: String,
    pub network: Network,
    pub is_available: bool,
    /// Limits are null when the asset cannot be priced right now.
    pub limit: ServiceLimit,
    pub commission: ServiceCommission,
}

/// `/v1/payment/batch`, `/v1/refund/batch`, `/v1/payout/batch`, `/v1/transfer/batch` acknowledgement.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BatchSubmitted {
    pub batch_id: String,
    pub kind: BatchKind,
    pub status: BatchStatus,
    pub count: i64,
}

/// One element of `/v1/batch/info`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BatchInfoItem {
    /// Zero-based position of the item in the submitted batch.
    pub idx: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    pub status: String,
    /// The created object, shape depending on the batch `kind`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<std::collections::BTreeMap<String, serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_status: Option<i64>,
}

/// `/v1/batch/info`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BatchInfo {
    pub batch_id: String,
    pub kind: BatchKind,
    pub status: BatchStatus,
    pub on_error: BatchOnError,
    pub total: i64,
    pub succeeded: i64,
    pub failed: i64,
    pub items: Vec<BatchInfoItem>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
