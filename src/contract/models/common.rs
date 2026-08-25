//! Building blocks shared by every response model: the money newtype, timestamps, batch elements
//! and the trivial `{ "ok": true }` acknowledgement.

use crate::contract::enums::FeeBearerResult;

/// Decimal amount rendered by the core at the asset's own scale (`"10.000000"` for USDT).
/// Never a float: `f64` cannot hold 18 decimals, and rounding a payout is a real loss.
#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(transparent)]
pub struct Money(pub String);

impl Money {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn into_string(self) -> String {
        self.0
    }
}
impl std::fmt::Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl From<&str> for Money {
    fn from(v: &str) -> Self {
        Money(v.to_string())
    }
}
impl From<String> for Money {
    fn from(v: String) -> Self {
        Money(v)
    }
}
impl std::ops::Deref for Money {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

/// RFC 3339 timestamp in UTC (`2026-08-25T20:58:55Z`).
pub type Timestamp = String;

/// Distinguishes "absent" from "present and null" for a field that can be both.
pub fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    serde::Deserialize::deserialize(de).map(Some)
}

/// Element of every batch listing (`/v1/payout/mass`, `/v1/payout/link/batch`, `/v1/batch/info`).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BatchElement<T> {
    /// Zero-based position of the item in the submitted batch.
    pub idx: i64,
    /// Whether this element succeeded.
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    /// The created object, when the element succeeded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    /// Human-readable failure text, when it failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_status: Option<i64>,
}

/// How a fee was settled on a priced result. `fee_type` is the pricing mode (`percent`/`fixed`/…).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FeeInfo {
    pub commission: Money,
    pub fee_bearer: FeeBearerResult,
    pub fee_type: String,
}

/// Kinds of asynchronous batches: `payment`, `payout`, `refund`, `transfer`, `payout_link` —
/// an open vocabulary, so it stays a string.
pub type BatchKind = String;

/// Lifecycle of an asynchronous batch: `queued`, `processing`, `done`, `stopped` —
/// an open vocabulary, so it stays a string.
pub type BatchStatus = String;

/// The bare acknowledgement a few routes answer with.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OkResult {
    pub ok: bool,
}
