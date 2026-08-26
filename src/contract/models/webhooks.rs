//! Webhook endpoints, delivery records and the events themselves.

use crate::contract::enums::{
    DeliveryStatus, FeeBearerResult, Network, PaymentStatus, PayoutStatus,
};
use crate::contract::models::common::{Money, Timestamp};

/// `POST /v1/webhooks`.
///
/// `Debug` never prints `secret`: a signing secret in a log is a forged-webhook kit.
#[derive(Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WebhookEndpoint {
    pub endpoint_id: String,
    pub url: String,
    /// Shown once: at first registration and at rotation. Absent when only the URL was changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
}

impl std::fmt::Debug for WebhookEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebhookEndpoint")
            .field("endpoint_id", &self.endpoint_id)
            .field("url", &self.url)
            .field("secret", &super::common::redacted(&self.secret))
            .finish()
    }
}

/// `POST /v1/webhooks/rotate-secret`.
///
/// `Debug` never prints `secret`.
#[derive(Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WebhookSecretRotated {
    pub endpoint_id: String,
    pub url: String,
    /// The new secret, shown once.
    pub secret: String,
    /// Until then deliveries also carry `X-Webhook-Signature-Prev` signed with the old secret.
    pub previous_secret_valid_until: Timestamp,
}

impl std::fmt::Debug for WebhookSecretRotated {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebhookSecretRotated")
            .field("endpoint_id", &self.endpoint_id)
            .field("url", &self.url)
            .field("secret", &super::common::REDACTED)
            .field(
                "previous_secret_valid_until",
                &self.previous_secret_valid_until,
            )
            .finish()
    }
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
    /// Present and true ONLY on rehearsal deliveries (`webhooks.test`, sandbox). The body is signed
    /// like a live one, so a handler must check this flag (or `X-Webhook-Test`) and never act on a
    /// test event as if money moved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test: Option<bool>,
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
    /// Present and true ONLY on rehearsal deliveries (`webhooks.test`, sandbox). The body is signed
    /// like a live one, so a handler must check this flag (or `X-Webhook-Test`) and never act on a
    /// test event as if money moved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test: Option<bool>,
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
    /// Present and true ONLY on rehearsal deliveries (`webhooks.test`, sandbox). The body is signed
    /// like a live one, so a handler must check this flag (or `X-Webhook-Test`) and never act on a
    /// test event as if money moved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test: Option<bool>,
}

/// Any delivered event, told apart by its `type` field.
///
/// `Other` keeps a `type` newer than this contract snapshot readable instead of failing the whole
/// delivery: a receiver can still acknowledge it, log it and move on. The enum is
/// `#[non_exhaustive]`, so a future variant is not a breaking change — match with a `_` arm.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum WebhookEvent {
    Payment(PaymentEvent),
    Payout(PayoutEvent),
    Wallet(WalletEvent),
    /// An event type this snapshot does not know, exactly as it arrived.
    Other(serde_json::Value),
}

/// Serializes back to the wire shape: the event's own fields with `type` alongside them, and an
/// unknown event exactly as it arrived.
impl serde::Serialize for WebhookEvent {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::Error as _;
        let mut value = match self {
            WebhookEvent::Payment(e) => serde_json::to_value(e),
            WebhookEvent::Payout(e) => serde_json::to_value(e),
            WebhookEvent::Wallet(e) => serde_json::to_value(e),
            WebhookEvent::Other(v) => return v.serialize(serializer),
        }
        .map_err(S::Error::custom)?;
        if let serde_json::Value::Object(map) = &mut value {
            map.insert(
                "type".to_string(),
                serde_json::Value::String(self.event_kind().to_string()),
            );
        }
        value.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for WebhookEvent {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        let value = serde_json::Value::deserialize(d)?;
        let kind = value
            .get("type")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| D::Error::custom("event body lacks the string `type` field"))?;
        match kind {
            "payment" => serde_json::from_value(value)
                .map(WebhookEvent::Payment)
                .map_err(D::Error::custom),
            "payout" => serde_json::from_value(value)
                .map(WebhookEvent::Payout)
                .map_err(D::Error::custom),
            "wallet" => serde_json::from_value(value)
                .map(WebhookEvent::Wallet)
                .map_err(D::Error::custom),
            _ => Ok(WebhookEvent::Other(value)),
        }
    }
}

impl WebhookEvent {
    /// The object the event is about (invoice, payout or wallet deposit).
    pub fn uuid(&self) -> &str {
        match self {
            WebhookEvent::Payment(e) => &e.uuid,
            WebhookEvent::Payout(e) => &e.uuid,
            WebhookEvent::Wallet(e) => &e.uuid,
            WebhookEvent::Other(v) => str_at(v, "uuid").unwrap_or_default(),
        }
    }

    /// Global, increasing (gaps are normal); a lower sequence arriving later is stale. `None` when
    /// the event carries no integer `sequence` — an unknown type, or one the gateway omitted it on.
    pub fn sequence(&self) -> Option<i64> {
        match self {
            WebhookEvent::Payment(e) => Some(e.sequence),
            WebhookEvent::Payout(e) => Some(e.sequence),
            WebhookEvent::Wallet(e) => Some(e.sequence),
            WebhookEvent::Other(v) => v.get("sequence").and_then(serde_json::Value::as_i64),
        }
    }

    /// Merchant reference, when the event carries one (null on refund payouts).
    pub fn order_id(&self) -> Option<&str> {
        match self {
            WebhookEvent::Payment(e) => e.order_id.as_deref(),
            WebhookEvent::Payout(e) => e.order_id.as_deref(),
            WebhookEvent::Wallet(e) => e.order_id.as_deref(),
            WebhookEvent::Other(v) => str_at(v, "order_id"),
        }
    }

    /// When the state change was committed.
    pub fn event_at(&self) -> &str {
        match self {
            WebhookEvent::Payment(e) => &e.event_at,
            WebhookEvent::Payout(e) => &e.event_at,
            WebhookEvent::Wallet(e) => &e.event_at,
            WebhookEvent::Other(v) => str_at(v, "event_at").unwrap_or_default(),
        }
    }

    /// Whether the object reached a terminal state.
    pub fn is_final(&self) -> bool {
        match self {
            WebhookEvent::Payment(e) => e.is_final,
            WebhookEvent::Payout(e) => e.is_final,
            WebhookEvent::Wallet(e) => e.is_final,
            WebhookEvent::Other(v) => {
                v.get("is_final").and_then(serde_json::Value::as_bool) == Some(true)
            }
        }
    }

    /// The object's status as the wire spells it.
    pub fn status(&self) -> &str {
        match self {
            WebhookEvent::Payment(e) => e.status.as_str(),
            WebhookEvent::Payout(e) => e.status.as_str(),
            WebhookEvent::Wallet(e) => &e.status,
            WebhookEvent::Other(v) => str_at(v, "status").unwrap_or_default(),
        }
    }

    /// True on a rehearsal delivery (`webhooks.test`, sandbox): signed exactly like a live one,
    /// but nothing moved — never credit an order on it. Works on an unknown event type too.
    pub fn is_test(&self) -> bool {
        match self {
            WebhookEvent::Payment(e) => e.test == Some(true),
            WebhookEvent::Payout(e) => e.test == Some(true),
            WebhookEvent::Wallet(e) => e.test == Some(true),
            WebhookEvent::Other(v) => {
                v.get("test").and_then(serde_json::Value::as_bool) == Some(true)
            }
        }
    }

    /// The discriminator as the wire spells it: `"payment"`, `"payout"`, `"wallet"`, or whatever
    /// an unknown event carried.
    pub fn event_kind(&self) -> &str {
        match self {
            WebhookEvent::Payment(_) => "payment",
            WebhookEvent::Payout(_) => "payout",
            WebhookEvent::Wallet(_) => "wallet",
            WebhookEvent::Other(v) => str_at(v, "type").unwrap_or_default(),
        }
    }

    /// Whether this snapshot models the event's `type`. `false` means [`WebhookEvent::Other`]:
    /// acknowledge the delivery, log it, and do not try to read money out of it.
    pub fn is_known(&self) -> bool {
        !matches!(self, WebhookEvent::Other(_))
    }

    /// The delivery body exactly as it arrived, for an event type this snapshot does not model.
    pub fn raw(&self) -> Option<&serde_json::Value> {
        match self {
            WebhookEvent::Other(v) => Some(v),
            _ => None,
        }
    }
}

/// A string field of a raw event body.
fn str_at<'a>(value: &'a serde_json::Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(serde_json::Value::as_str)
}
