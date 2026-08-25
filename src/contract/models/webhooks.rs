//! Webhook endpoints, delivery records and the events themselves.

use crate::contract::enums::{
    DeliveryStatus, FeeBearerResult, Network, PaymentStatus, PayoutStatus,
};
use crate::contract::models::common::{Money, Timestamp};

/// `POST /v1/webhooks`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WebhookEndpoint {
    pub endpoint_id: String,
    pub url: String,
    /// Shown once: at first registration and at rotation. Absent when only the URL was changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
}

/// `POST /v1/webhooks/rotate-secret`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WebhookSecretRotated {
    pub endpoint_id: String,
    pub url: String,
    /// The new secret, shown once.
    pub secret: String,
    /// Until then deliveries also carry `X-Webhook-Signature-Prev` signed with the old secret.
    pub previous_secret_valid_until: Timestamp,
}

/// Item of `/v1/webhooks/deliveries` and `GET /v1/sandbox/webhooks` (which adds `payload`, drops `sequence`).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WebhookDelivery {
    pub id: String,
    pub url: String,
    /// `invoice.paid`, `payout.confirmed`, `wallet.paid`, …
    pub event_type: String,
    pub status: DeliveryStatus,
    pub attempts: i64,
    pub last_error: String,
    /// Absent on the sandbox listing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<i64>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    /// The delivered event body — sandbox listing only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<std::collections::BTreeMap<String, serde_json::Value>>,
}

/// `/v1/test-webhook/*` and `/v1/payment/testing-webhook`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WebhookTestResult {
    pub ok: bool,
    pub signed: bool,
    /// Absent when the receiver could not be reached (see `error`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_code: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// `/v1/payment/testing-webhook` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// `/v1/payment/testing-webhook` only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<i64>,
}

/// `invoice.<status>` — an invoice changed state.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentEvent {
    pub uuid: String,
    /// Null on refund payouts.
    pub order_id: Option<String>,
    pub status: PaymentStatus,
    pub is_final: bool,
    pub amount: Money,
    pub currency: String,
    pub network: Network,
    pub payer_amount: Money,
    pub payer_currency: String,
    /// What actually landed on the address, in `payer_currency`.
    pub payment_amount: Money,
    pub payer_address: String,
    pub payer_address_is_refundable: bool,
    pub additional_data: String,
    pub txid: String,
    /// When the state change was committed — order events by this, or by `sequence`.
    pub event_at: Timestamp,
    /// Global, increasing (gaps are normal); a lower sequence arriving later is stale.
    pub sequence: i64,
}

/// `payout.<status>` — a payout (or refund) changed state; the body is the payout itself.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutEvent {
    pub uuid: String,
    /// Null on refund payouts.
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
    /// When the state change was committed — order events by this, or by `sequence`.
    pub event_at: Timestamp,
    /// Global, increasing (gaps are normal); a lower sequence arriving later is stale.
    pub sequence: i64,
}

/// `wallet.paid` — a deposit landed on a static wallet.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WalletEvent {
    pub uuid: String,
    /// Null on refund payouts.
    pub order_id: Option<String>,
    /// Always `"paid"`.
    pub status: String,
    pub is_final: bool,
    pub address: String,
    pub currency: String,
    pub network: Network,
    pub payer_currency: String,
    /// What actually landed on the address, in `payer_currency`.
    pub payment_amount: Money,
    pub txid: String,
    /// When the state change was committed — order events by this, or by `sequence`.
    pub event_at: Timestamp,
    /// Global, increasing (gaps are normal); a lower sequence arriving later is stale.
    pub sequence: i64,
}

/// Any delivered event, told apart by its `type` field.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WebhookEvent {
    Payment(PaymentEvent),
    Payout(PayoutEvent),
    Wallet(WalletEvent),
}

impl WebhookEvent {
    /// The object the event is about (invoice, payout or wallet deposit).
    pub fn uuid(&self) -> &str {
        match self {
            WebhookEvent::Payment(e) => &e.uuid,
            WebhookEvent::Payout(e) => &e.uuid,
            WebhookEvent::Wallet(e) => &e.uuid,
        }
    }

    /// Global, increasing (gaps are normal); a lower sequence arriving later is stale.
    pub fn sequence(&self) -> i64 {
        match self {
            WebhookEvent::Payment(e) => e.sequence,
            WebhookEvent::Payout(e) => e.sequence,
            WebhookEvent::Wallet(e) => e.sequence,
        }
    }

    /// Merchant reference, when the event carries one (null on refund payouts).
    pub fn order_id(&self) -> Option<&str> {
        match self {
            WebhookEvent::Payment(e) => e.order_id.as_deref(),
            WebhookEvent::Payout(e) => e.order_id.as_deref(),
            WebhookEvent::Wallet(e) => e.order_id.as_deref(),
        }
    }

    /// When the state change was committed.
    pub fn event_at(&self) -> &str {
        match self {
            WebhookEvent::Payment(e) => &e.event_at,
            WebhookEvent::Payout(e) => &e.event_at,
            WebhookEvent::Wallet(e) => &e.event_at,
        }
    }

    /// Whether the object reached a terminal state.
    pub fn is_final(&self) -> bool {
        match self {
            WebhookEvent::Payment(e) => e.is_final,
            WebhookEvent::Payout(e) => e.is_final,
            WebhookEvent::Wallet(e) => e.is_final,
        }
    }

    /// The object's status as the wire spells it.
    pub fn status(&self) -> &str {
        match self {
            WebhookEvent::Payment(e) => e.status.as_str(),
            WebhookEvent::Payout(e) => e.status.as_str(),
            WebhookEvent::Wallet(e) => &e.status,
        }
    }

    /// The discriminator: `"payment"`, `"payout"` or `"wallet"`.
    pub fn event_kind(&self) -> &'static str {
        match self {
            WebhookEvent::Payment(_) => "payment",
            WebhookEvent::Payout(_) => "payout",
            WebhookEvent::Wallet(_) => "wallet",
        }
    }
}
