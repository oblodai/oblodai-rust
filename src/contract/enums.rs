// GENERATED FILE - do not edit. Source: contract/contract.json (core 7b8eb828b9ec).
// Regenerate with: python3 scripts/codegen.py

/// Invoice lifecycle, as `payment.status` carries it.
///
/// `Other` keeps a value newer than this contract snapshot decodable instead of failing
/// the whole response.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum PaymentStatus {
    #[serde(rename = "select")]
    Select,
    #[serde(rename = "created")]
    Created,
    #[serde(rename = "confirm_check")]
    ConfirmCheck,
    #[serde(rename = "paid")]
    Paid,
    #[serde(rename = "paid_over")]
    PaidOver,
    #[serde(rename = "wrong_amount")]
    WrongAmount,
    #[serde(rename = "expired")]
    Expired,
    #[serde(rename = "cancelled")]
    Cancelled,
    /// A value this snapshot does not know.
    #[serde(untagged)]
    Other(String),
}

impl PaymentStatus {
    /// The wire value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Select => "select",
            Self::Created => "created",
            Self::ConfirmCheck => "confirm_check",
            Self::Paid => "paid",
            Self::PaidOver => "paid_over",
            Self::WrongAmount => "wrong_amount",
            Self::Expired => "expired",
            Self::Cancelled => "cancelled",
            Self::Other(v) => v.as_str(),
        }
    }
}

impl std::fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for PaymentStatus {
    fn from(v: &str) -> Self {
        match v {
            "select" => Self::Select,
            "created" => Self::Created,
            "confirm_check" => Self::ConfirmCheck,
            "paid" => Self::Paid,
            "paid_over" => Self::PaidOver,
            "wrong_amount" => Self::WrongAmount,
            "expired" => Self::Expired,
            "cancelled" => Self::Cancelled,
            other => Self::Other(other.to_string()),
        }
    }
}

impl From<String> for PaymentStatus {
    fn from(v: String) -> Self {
        Self::from(v.as_str())
    }
}

impl Default for PaymentStatus {
    fn default() -> Self {
        Self::Other(String::new())
    }
}

impl std::str::FromStr for PaymentStatus {
    type Err = std::convert::Infallible;
    fn from_str(v: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(v))
    }
}

/// Every `PaymentStatus` value in this contract snapshot.
pub const PAYMENT_STATUSES: &[&str] = &[
    "select",
    "created",
    "confirm_check",
    "paid",
    "paid_over",
    "wrong_amount",
    "expired",
    "cancelled",
];

/// Payout lifecycle, as `payout.status` carries it.
///
/// `Other` keeps a value newer than this contract snapshot decodable instead of failing
/// the whole response.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum PayoutStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "approved")]
    Approved,
    #[serde(rename = "awaiting_cosign")]
    AwaitingCosign,
    #[serde(rename = "broadcasting")]
    Broadcasting,
    #[serde(rename = "sent")]
    Sent,
    #[serde(rename = "confirmed")]
    Confirmed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
    /// A value this snapshot does not know.
    #[serde(untagged)]
    Other(String),
}

impl PayoutStatus {
    /// The wire value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::AwaitingCosign => "awaiting_cosign",
            Self::Broadcasting => "broadcasting",
            Self::Sent => "sent",
            Self::Confirmed => "confirmed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Other(v) => v.as_str(),
        }
    }
}

impl std::fmt::Display for PayoutStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for PayoutStatus {
    fn from(v: &str) -> Self {
        match v {
            "pending" => Self::Pending,
            "approved" => Self::Approved,
            "awaiting_cosign" => Self::AwaitingCosign,
            "broadcasting" => Self::Broadcasting,
            "sent" => Self::Sent,
            "confirmed" => Self::Confirmed,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            other => Self::Other(other.to_string()),
        }
    }
}

impl From<String> for PayoutStatus {
    fn from(v: String) -> Self {
        Self::from(v.as_str())
    }
}

impl Default for PayoutStatus {
    fn default() -> Self {
        Self::Other(String::new())
    }
}

impl std::str::FromStr for PayoutStatus {
    type Err = std::convert::Infallible;
    fn from_str(v: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(v))
    }
}

/// Every `PayoutStatus` value in this contract snapshot.
pub const PAYOUT_STATUSES: &[&str] = &[
    "pending",
    "approved",
    "awaiting_cosign",
    "broadcasting",
    "sent",
    "confirmed",
    "failed",
    "cancelled",
];

/// Payout-link (cheque) lifecycle.
///
/// `Other` keeps a value newer than this contract snapshot decodable instead of failing
/// the whole response.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum PayoutLinkStatus {
    #[serde(rename = "funded")]
    Funded,
    #[serde(rename = "claiming")]
    Claiming,
    #[serde(rename = "claimed")]
    Claimed,
    #[serde(rename = "expired")]
    Expired,
    #[serde(rename = "cancelled")]
    Cancelled,
    /// A value this snapshot does not know.
    #[serde(untagged)]
    Other(String),
}

impl PayoutLinkStatus {
    /// The wire value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Funded => "funded",
            Self::Claiming => "claiming",
            Self::Claimed => "claimed",
            Self::Expired => "expired",
            Self::Cancelled => "cancelled",
            Self::Other(v) => v.as_str(),
        }
    }
}

impl std::fmt::Display for PayoutLinkStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for PayoutLinkStatus {
    fn from(v: &str) -> Self {
        match v {
            "funded" => Self::Funded,
            "claiming" => Self::Claiming,
            "claimed" => Self::Claimed,
            "expired" => Self::Expired,
            "cancelled" => Self::Cancelled,
            other => Self::Other(other.to_string()),
        }
    }
}

impl From<String> for PayoutLinkStatus {
    fn from(v: String) -> Self {
        Self::from(v.as_str())
    }
}

impl Default for PayoutLinkStatus {
    fn default() -> Self {
        Self::Other(String::new())
    }
}

impl std::str::FromStr for PayoutLinkStatus {
    type Err = std::convert::Infallible;
    fn from_str(v: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(v))
    }
}

/// Every `PayoutLinkStatus` value in this contract snapshot.
pub const PAYOUT_LINK_STATUSES: &[&str] =
    &["funded", "claiming", "claimed", "expired", "cancelled"];

/// Webhook delivery lifecycle.
///
/// `Other` keeps a value newer than this contract snapshot decodable instead of failing
/// the whole response.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum DeliveryStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "delivered")]
    Delivered,
    #[serde(rename = "dead")]
    Dead,
    /// A value this snapshot does not know.
    #[serde(untagged)]
    Other(String),
}

impl DeliveryStatus {
    /// The wire value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Pending => "pending",
            Self::Delivered => "delivered",
            Self::Dead => "dead",
            Self::Other(v) => v.as_str(),
        }
    }
}

impl std::fmt::Display for DeliveryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for DeliveryStatus {
    fn from(v: &str) -> Self {
        match v {
            "pending" => Self::Pending,
            "delivered" => Self::Delivered,
            "dead" => Self::Dead,
            other => Self::Other(other.to_string()),
        }
    }
}

impl From<String> for DeliveryStatus {
    fn from(v: String) -> Self {
        Self::from(v.as_str())
    }
}

impl Default for DeliveryStatus {
    fn default() -> Self {
        Self::Other(String::new())
    }
}

impl std::str::FromStr for DeliveryStatus {
    type Err = std::convert::Infallible;
    fn from_str(v: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(v))
    }
}

/// Every `DeliveryStatus` value in this contract snapshot.
pub const DELIVERY_STATUSES: &[&str] = &["pending", "delivered", "dead"];

/// Settlement networks the gateway supports.
///
/// `Other` keeps a value newer than this contract snapshot decodable instead of failing
/// the whole response.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Network {
    #[serde(rename = "ethereum")]
    Ethereum,
    #[serde(rename = "bsc")]
    Bsc,
    #[serde(rename = "polygon")]
    Polygon,
    #[serde(rename = "avalanche")]
    Avalanche,
    #[serde(rename = "base")]
    Base,
    #[serde(rename = "arbitrum")]
    Arbitrum,
    #[serde(rename = "tron")]
    Tron,
    #[serde(rename = "solana")]
    Solana,
    #[serde(rename = "ton")]
    Ton,
    #[serde(rename = "bitcoin")]
    Bitcoin,
    #[serde(rename = "litecoin")]
    Litecoin,
    #[serde(rename = "dogecoin")]
    Dogecoin,
    #[serde(rename = "bitcoincash")]
    Bitcoincash,
    #[serde(rename = "dash")]
    Dash,
    #[serde(rename = "xrp")]
    Xrp,
    #[serde(rename = "stellar")]
    Stellar,
    #[serde(rename = "monero")]
    Monero,
    /// A value this snapshot does not know.
    #[serde(untagged)]
    Other(String),
}

impl Network {
    /// The wire value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Ethereum => "ethereum",
            Self::Bsc => "bsc",
            Self::Polygon => "polygon",
            Self::Avalanche => "avalanche",
            Self::Base => "base",
            Self::Arbitrum => "arbitrum",
            Self::Tron => "tron",
            Self::Solana => "solana",
            Self::Ton => "ton",
            Self::Bitcoin => "bitcoin",
            Self::Litecoin => "litecoin",
            Self::Dogecoin => "dogecoin",
            Self::Bitcoincash => "bitcoincash",
            Self::Dash => "dash",
            Self::Xrp => "xrp",
            Self::Stellar => "stellar",
            Self::Monero => "monero",
            Self::Other(v) => v.as_str(),
        }
    }
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for Network {
    fn from(v: &str) -> Self {
        match v {
            "ethereum" => Self::Ethereum,
            "bsc" => Self::Bsc,
            "polygon" => Self::Polygon,
            "avalanche" => Self::Avalanche,
            "base" => Self::Base,
            "arbitrum" => Self::Arbitrum,
            "tron" => Self::Tron,
            "solana" => Self::Solana,
            "ton" => Self::Ton,
            "bitcoin" => Self::Bitcoin,
            "litecoin" => Self::Litecoin,
            "dogecoin" => Self::Dogecoin,
            "bitcoincash" => Self::Bitcoincash,
            "dash" => Self::Dash,
            "xrp" => Self::Xrp,
            "stellar" => Self::Stellar,
            "monero" => Self::Monero,
            other => Self::Other(other.to_string()),
        }
    }
}

impl From<String> for Network {
    fn from(v: String) -> Self {
        Self::from(v.as_str())
    }
}

impl Default for Network {
    fn default() -> Self {
        Self::Other(String::new())
    }
}

impl std::str::FromStr for Network {
    type Err = std::convert::Infallible;
    fn from_str(v: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(v))
    }
}

/// Every `Network` value in this contract snapshot.
pub const NETWORKS: &[&str] = &[
    "ethereum",
    "bsc",
    "polygon",
    "avalanche",
    "base",
    "arbitrum",
    "tron",
    "solana",
    "ton",
    "bitcoin",
    "litecoin",
    "dogecoin",
    "bitcoincash",
    "dash",
    "xrp",
    "stellar",
    "monero",
];

/// Who pays the network fee, as requested.
///
/// `Other` keeps a value newer than this contract snapshot decodable instead of failing
/// the whole response.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum FeeBearer {
    #[serde(rename = "recipient")]
    Recipient,
    #[serde(rename = "merchant")]
    Merchant,
    /// A value this snapshot does not know.
    #[serde(untagged)]
    Other(String),
}

impl FeeBearer {
    /// The wire value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Recipient => "recipient",
            Self::Merchant => "merchant",
            Self::Other(v) => v.as_str(),
        }
    }
}

impl std::fmt::Display for FeeBearer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for FeeBearer {
    fn from(v: &str) -> Self {
        match v {
            "recipient" => Self::Recipient,
            "merchant" => Self::Merchant,
            other => Self::Other(other.to_string()),
        }
    }
}

impl From<String> for FeeBearer {
    fn from(v: String) -> Self {
        Self::from(v.as_str())
    }
}

impl Default for FeeBearer {
    fn default() -> Self {
        Self::Other(String::new())
    }
}

impl std::str::FromStr for FeeBearer {
    type Err = std::convert::Infallible;
    fn from_str(v: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(v))
    }
}

/// Every `FeeBearer` value in this contract snapshot.
pub const FEE_BEARERS: &[&str] = &["recipient", "merchant"];

/// Who paid the network fee, as settled.
///
/// `Other` keeps a value newer than this contract snapshot decodable instead of failing
/// the whole response.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum FeeBearerResult {
    #[serde(rename = "recipient")]
    Recipient,
    #[serde(rename = "merchant")]
    Merchant,
    #[serde(rename = "gateway")]
    Gateway,
    /// A value this snapshot does not know.
    #[serde(untagged)]
    Other(String),
}

impl FeeBearerResult {
    /// The wire value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Recipient => "recipient",
            Self::Merchant => "merchant",
            Self::Gateway => "gateway",
            Self::Other(v) => v.as_str(),
        }
    }
}

impl std::fmt::Display for FeeBearerResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for FeeBearerResult {
    fn from(v: &str) -> Self {
        match v {
            "recipient" => Self::Recipient,
            "merchant" => Self::Merchant,
            "gateway" => Self::Gateway,
            other => Self::Other(other.to_string()),
        }
    }
}

impl From<String> for FeeBearerResult {
    fn from(v: String) -> Self {
        Self::from(v.as_str())
    }
}

impl Default for FeeBearerResult {
    fn default() -> Self {
        Self::Other(String::new())
    }
}

impl std::str::FromStr for FeeBearerResult {
    type Err = std::convert::Infallible;
    fn from_str(v: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(v))
    }
}

/// Every `FeeBearerResult` value in this contract snapshot.
pub const FEE_BEARER_RESULTS: &[&str] = &["recipient", "merchant", "gateway"];

/// What an asynchronous batch does after a failed row.
///
/// `Other` keeps a value newer than this contract snapshot decodable instead of failing
/// the whole response.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum BatchOnError {
    #[serde(rename = "continue")]
    Continue,
    #[serde(rename = "stop")]
    Stop,
    /// A value this snapshot does not know.
    #[serde(untagged)]
    Other(String),
}

impl BatchOnError {
    /// The wire value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Continue => "continue",
            Self::Stop => "stop",
            Self::Other(v) => v.as_str(),
        }
    }
}

impl std::fmt::Display for BatchOnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for BatchOnError {
    fn from(v: &str) -> Self {
        match v {
            "continue" => Self::Continue,
            "stop" => Self::Stop,
            other => Self::Other(other.to_string()),
        }
    }
}

impl From<String> for BatchOnError {
    fn from(v: String) -> Self {
        Self::from(v.as_str())
    }
}

impl Default for BatchOnError {
    fn default() -> Self {
        Self::Other(String::new())
    }
}

impl std::str::FromStr for BatchOnError {
    type Err = std::convert::Infallible;
    fn from_str(v: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(v))
    }
}

/// Every `BatchOnError` value in this contract snapshot.
pub const BATCH_ON_ERRORS: &[&str] = &["continue", "stop"];

/// Kind of sample event `webhooks.test` delivers.
///
/// `Other` keeps a value newer than this contract snapshot decodable instead of failing
/// the whole response.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum WebhookKind {
    #[serde(rename = "payment")]
    Payment,
    #[serde(rename = "payout")]
    Payout,
    #[serde(rename = "wallet")]
    Wallet,
    /// A value this snapshot does not know.
    #[serde(untagged)]
    Other(String),
}

impl WebhookKind {
    /// The wire value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Payment => "payment",
            Self::Payout => "payout",
            Self::Wallet => "wallet",
            Self::Other(v) => v.as_str(),
        }
    }
}

impl std::fmt::Display for WebhookKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for WebhookKind {
    fn from(v: &str) -> Self {
        match v {
            "payment" => Self::Payment,
            "payout" => Self::Payout,
            "wallet" => Self::Wallet,
            other => Self::Other(other.to_string()),
        }
    }
}

impl From<String> for WebhookKind {
    fn from(v: String) -> Self {
        Self::from(v.as_str())
    }
}

impl Default for WebhookKind {
    fn default() -> Self {
        Self::Other(String::new())
    }
}

impl std::str::FromStr for WebhookKind {
    type Err = std::convert::Infallible;
    fn from_str(v: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(v))
    }
}

/// Every `WebhookKind` value in this contract snapshot.
pub const WEBHOOK_KINDS: &[&str] = &["payment", "payout", "wallet"];

/// How the core itself classifies a failure (the envelope's own taxonomy).
///
/// `Other` keeps a value newer than this contract snapshot decodable instead of failing
/// the whole response.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum CoreErrorKind {
    #[serde(rename = "invalid")]
    Invalid,
    #[serde(rename = "unauthorized")]
    Unauthorized,
    #[serde(rename = "forbidden")]
    Forbidden,
    #[serde(rename = "not_found")]
    NotFound,
    #[serde(rename = "conflict")]
    Conflict,
    #[serde(rename = "rate_limited")]
    RateLimited,
    #[serde(rename = "unavailable")]
    Unavailable,
    #[serde(rename = "internal")]
    Internal,
    /// A value this snapshot does not know.
    #[serde(untagged)]
    Other(String),
}

impl CoreErrorKind {
    /// The wire value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Invalid => "invalid",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "forbidden",
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::RateLimited => "rate_limited",
            Self::Unavailable => "unavailable",
            Self::Internal => "internal",
            Self::Other(v) => v.as_str(),
        }
    }
}

impl std::fmt::Display for CoreErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for CoreErrorKind {
    fn from(v: &str) -> Self {
        match v {
            "invalid" => Self::Invalid,
            "unauthorized" => Self::Unauthorized,
            "forbidden" => Self::Forbidden,
            "not_found" => Self::NotFound,
            "conflict" => Self::Conflict,
            "rate_limited" => Self::RateLimited,
            "unavailable" => Self::Unavailable,
            "internal" => Self::Internal,
            other => Self::Other(other.to_string()),
        }
    }
}

impl From<String> for CoreErrorKind {
    fn from(v: String) -> Self {
        Self::from(v.as_str())
    }
}

impl Default for CoreErrorKind {
    fn default() -> Self {
        Self::Other(String::new())
    }
}

impl std::str::FromStr for CoreErrorKind {
    type Err = std::convert::Infallible;
    fn from_str(v: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(v))
    }
}

/// Every `CoreErrorKind` value in this contract snapshot.
pub const CORE_ERROR_KINDS: &[&str] = &[
    "invalid",
    "unauthorized",
    "forbidden",
    "not_found",
    "conflict",
    "rate_limited",
    "unavailable",
    "internal",
];

/// How a payment link prices its invoices.
///
/// `Other` keeps a value newer than this contract snapshot decodable instead of failing
/// the whole response.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum AmountMode {
    #[serde(rename = "fixed")]
    Fixed,
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "range")]
    Range,
    /// A value this snapshot does not know.
    #[serde(untagged)]
    Other(String),
}

impl AmountMode {
    /// The wire value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Fixed => "fixed",
            Self::Open => "open",
            Self::Range => "range",
            Self::Other(v) => v.as_str(),
        }
    }
}

impl std::fmt::Display for AmountMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for AmountMode {
    fn from(v: &str) -> Self {
        match v {
            "fixed" => Self::Fixed,
            "open" => Self::Open,
            "range" => Self::Range,
            other => Self::Other(other.to_string()),
        }
    }
}

impl From<String> for AmountMode {
    fn from(v: String) -> Self {
        Self::from(v.as_str())
    }
}

impl Default for AmountMode {
    fn default() -> Self {
        Self::Other(String::new())
    }
}

impl std::str::FromStr for AmountMode {
    type Err = std::convert::Infallible;
    fn from_str(v: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(v))
    }
}

/// Every `AmountMode` value in this contract snapshot.
pub const AMOUNT_MODES: &[&str] = &["fixed", "open", "range"];

/// Which side of the payout ledger `payout.history` lists.
///
/// `Other` keeps a value newer than this contract snapshot decodable instead of failing
/// the whole response.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum PayoutKind {
    #[serde(rename = "payout")]
    Payout,
    #[serde(rename = "refund")]
    Refund,
    /// A value this snapshot does not know.
    #[serde(untagged)]
    Other(String),
}

impl PayoutKind {
    /// The wire value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Payout => "payout",
            Self::Refund => "refund",
            Self::Other(v) => v.as_str(),
        }
    }
}

impl std::fmt::Display for PayoutKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for PayoutKind {
    fn from(v: &str) -> Self {
        match v {
            "payout" => Self::Payout,
            "refund" => Self::Refund,
            other => Self::Other(other.to_string()),
        }
    }
}

impl From<String> for PayoutKind {
    fn from(v: String) -> Self {
        Self::from(v.as_str())
    }
}

impl Default for PayoutKind {
    fn default() -> Self {
        Self::Other(String::new())
    }
}

impl std::str::FromStr for PayoutKind {
    type Err = std::convert::Infallible;
    fn from_str(v: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(v))
    }
}

/// Every `PayoutKind` value in this contract snapshot.
pub const PAYOUT_KINDS: &[&str] = &["payout", "refund"];

/// How an underpaid (`wrong_amount`) invoice is settled.
///
/// `Other` keeps a value newer than this contract snapshot decodable instead of failing
/// the whole response.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ResolveAction {
    #[serde(rename = "accept")]
    Accept,
    #[serde(rename = "refund")]
    Refund,
    /// A value this snapshot does not know.
    #[serde(untagged)]
    Other(String),
}

impl ResolveAction {
    /// The wire value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Accept => "accept",
            Self::Refund => "refund",
            Self::Other(v) => v.as_str(),
        }
    }
}

impl std::fmt::Display for ResolveAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for ResolveAction {
    fn from(v: &str) -> Self {
        match v {
            "accept" => Self::Accept,
            "refund" => Self::Refund,
            other => Self::Other(other.to_string()),
        }
    }
}

impl From<String> for ResolveAction {
    fn from(v: String) -> Self {
        Self::from(v.as_str())
    }
}

impl Default for ResolveAction {
    fn default() -> Self {
        Self::Other(String::new())
    }
}

impl std::str::FromStr for ResolveAction {
    type Err = std::convert::Infallible;
    fn from_str(v: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(v))
    }
}

/// Every `ResolveAction` value in this contract snapshot.
pub const RESOLVE_ACTIONS: &[&str] = &["accept", "refund"];

/// Webhook event types: `invoice.<status>`, `payout.<status>`, `wallet.paid`.
pub const EVENT_TYPES: &[&str] = &[
    "invoice.select",
    "invoice.created",
    "invoice.confirm_check",
    "invoice.paid",
    "invoice.paid_over",
    "invoice.wrong_amount",
    "invoice.expired",
    "invoice.cancelled",
    "payout.pending",
    "payout.approved",
    "payout.awaiting_cosign",
    "payout.broadcasting",
    "payout.sent",
    "payout.confirmed",
    "payout.failed",
    "payout.cancelled",
    "wallet.paid",
];

/// Every error code the core source can emit (`family.reason`).
pub const ERROR_CODES: &[&str] = &[
    "accepted.no_network",
    "accepted.unknown_method",
    "accounting.unavailable",
    "accuracy.out_of_range",
    "admin.bad_request",
    "admin.disabled",
    "admin.unauthorized",
    "analytics.bad_time",
    "apiallow.bad_cidr",
    "apiallow.empty",
    "apiallow.last_entry",
    "apiallow.platform_unidentifiable",
    "apiallow.too_many",
    "auth.bad_timestamp",
    "auth.body_too_large",
    "auth.ip_not_allowed",
    "autopilot.bad_energy",
    "autopilot.bad_usd",
    "autopilot.energy_no_free_trx",
    "autopilot.energy_too_large",
    "autopilot.gas_deep_deficit",
    "autopilot.gas_target_below_min",
    "autopilot.usd_too_large",
    "autowithdraw.bad_min",
    "autowithdraw.missing",
    "batch.bad_id",
    "batch.bad_recipient",
    "batch.disabled",
    "batch.duplicate_order_id",
    "batch.duplicate_reference",
    "batch.empty",
    "batch.invoice_required",
    "batch.not_found",
    "batch.order_id_required",
    "batch.reference_required",
    "batch.too_large",
    "batch.unsupported_kind",
    "blocklist.invalid_address",
    "blocklist.missing_fields",
    "blocklist.unknown_network",
    "cheque.token_required",
    "commission.bad_merchant",
    "commission.fixed_out_of_range",
    "commission.out_of_range",
    "compliance.blocked",
    "compliance.blocked_address",
    "compliance.blocklist_unavailable",
    "compliance.no_destination",
    "compliance.no_network",
    "confirmations.bad_tier",
    "confirmations.no_network",
    "confirmations.tier_too_large",
    "db.unavailable",
    "deposit.generation_stale",
    "discount.out_of_range",
    "document.bad_format",
    "document.bad_id",
    "document.bad_job_id",
    "document.bad_kind",
    "document.bad_signature",
    "document.balance_unavailable",
    "document.batch_unavailable",
    "document.daily_quota",
    "document.disabled",
    "document.encode_failed",
    "document.fees_unavailable",
    "document.job_expired",
    "document.job_failed",
    "document.job_not_found",
    "document.job_not_ready",
    "document.jobs_disabled",
    "document.ledger_unavailable",
    "document.link_expired",
    "document.no_split",
    "document.not_found",
    "document.render_failed",
    "document.render_rejected",
    "document.render_unavailable",
    "document.too_many_jobs",
    "document.unknown_kind",
    "document.unknown_lang",
    "document.wallet_abandoned",
    "document.wallet_blocked",
    "email.bad_recipient",
    "email.disabled",
    "email.no_recipient",
    "email.rate_limited",
    "history.bad_merchant_id",
    "idempotency.bad_key",
    "idempotency.in_progress",
    "idempotency.key_reused",
    "idempotency.unavailable",
    "internal",
    "invoice.address_failed",
    "invoice.already_paid",
    "invoice.bad_deposit",
    "invoice.bad_id",
    "invoice.bad_price",
    "invoice.bad_reversal_amount",
    "invoice.deposit_asset_mismatch",
    "invoice.deposit_exists",
    "invoice.deposit_pending",
    "invoice.expired",
    "invoice.fiat_pay_asset",
    "invoice.generation_stale",
    "invoice.no_pay_asset",
    "invoice.no_user",
    "invoice.not_payable",
    "invoice.not_selectable",
    "invoice.nothing_due",
    "invoice.partially_paid",
    "invoice.pay_asset_not_selected",
    "invoice.quote_failed",
    "invoice.refresh_lease",
    "invoice.refresh_not_expired",
    "invoice.refresh_paid",
    "invoice.refresh_select",
    "killswitch.reason_required",
    "killswitch.unavailable",
    "ledger.asset_mismatch",
    "ledger.idempotency_conflict",
    "ledger.sandbox_live_mix",
    "ledger.unavailable",
    "merchant.already_sandbox",
    "merchant.bad_fee_bearer",
    "merchant.bad_id",
    "merchant.bad_key_kind",
    "merchant.bad_project_id",
    "merchant.bad_settles_to",
    "merchant.bad_signature",
    "merchant.bad_subtract",
    "merchant.close_unavailable",
    "merchant.email_required",
    "merchant.key_mode_mismatch",
    "merchant.no_owner",
    "merchant.no_personal_wallet",
    "merchant.not_found",
    "merchant.project_mismatch",
    "merchant.secret_decrypt",
    "merchant.unknown_key",
    "merchant.wrong_key_kind",
    "minimum.negative",
    "minimum.no_network",
    "onramp.admit",
    "onramp.advance",
    "onramp.already_delivered",
    "onramp.bad_invoice",
    "onramp.bad_status",
    "onramp.captured",
    "onramp.expire",
    "onramp.fail_captured",
    "onramp.for_invoice",
    "onramp.force_completed",
    "onramp.invoice_not_found",
    "onramp.invoice_not_payable",
    "onramp.money_already_landed",
    "onramp.no_evidence",
    "onramp.no_refund_evidence",
    "onramp.not_captured",
    "onramp.not_found",
    "onramp.open",
    "onramp.open_conflict",
    "onramp.overdue",
    "onramp.owner_required",
    "onramp.refund_evidence",
    "onramp.release",
    "onramp.suppressed_in",
    "onramp.suppresses",
    "onramp.target_required",
    "onramp.terminal",
    "onramp.token",
    "pay.bad_uuid",
    "pay.below_minimum",
    "pay.method_not_accepted",
    "pay.minimum_unavailable",
    "pay.not_selectable",
    "paylink.above_max",
    "paylink.amount_required",
    "paylink.bad_amount",
    "paylink.bad_bounds",
    "paylink.bad_id",
    "paylink.bad_max",
    "paylink.bad_min",
    "paylink.bad_mode",
    "paylink.bad_range",
    "paylink.below_min",
    "paylink.disabled",
    "paylink.expires_in_negative",
    "paylink.expires_in_too_large",
    "paylink.not_found",
    "paylink.not_positive",
    "paylink.order_id_invalid",
    "paylink.order_id_too_long",
    "paylink.rate_limited",
    "paylink.unavailable",
    "payment.bad_accuracy",
    "payment.bad_amount",
    "payment.bad_payer_email",
    "payment.bad_redirect_url",
    "payment.bad_status",
    "payment.bad_subtract",
    "payment.bad_url_callback",
    "payment.bad_uuid",
    "payment.below_minimum",
    "payment.discount_unavailable",
    "payment.minimum_unavailable",
    "payment.network_required",
    "payment.no_lookup",
    "payment.not_found",
    "payment.subtract_impossible",
    "payment.to_currency_required",
    "payment.unknown_to_currency",
    "payment.unsupported_network",
    "payout.address_network_mismatch",
    "payout.amount_below_fee",
    "payout.approver_is_creator",
    "payout.asset_mismatch",
    "payout.bad_address",
    "payout.bad_amount",
    "payout.bad_kind",
    "payout.bad_memo",
    "payout.bad_owner_kind",
    "payout.bad_state",
    "payout.bad_status",
    "payout.bad_url_callback",
    "payout.bad_user",
    "payout.bad_uuid",
    "payout.batch_too_large",
    "payout.broadcast_in_flight",
    "payout.convert_bad_amount",
    "payout.convert_frozen",
    "payout.convert_insufficient",
    "payout.convert_no_rate",
    "payout.convert_same_asset",
    "payout.convert_unsupported",
    "payout.destination_internal",
    "payout.empty_batch",
    "payout.fee_asset_mismatch",
    "payout.freeze_unknown",
    "payout.from_currency_unsupported",
    "payout.frozen",
    "payout.funds_maturing",
    "payout.insufficient_funds",
    "payout.memo_conflict",
    "payout.memo_required",
    "payout.memo_too_long",
    "payout.no_destination",
    "payout.no_lookup",
    "payout.no_owner",
    "payout.no_sender",
    "payout.no_txid",
    "payout.no_user",
    "payout.not_broadcasting",
    "payout.not_found",
    "payout.not_pending",
    "payout.not_sent",
    "payout.order_id_required",
    "payout.reference_collision",
    "payout.release_reason_required",
    "payout.reserved_reference",
    "payout.test_merchant",
    "payout_link.disabled",
    "payoutctl.bad_body",
    "payoutctl.bad_cap",
    "payoutctl.bad_source",
    "payoutctl.unavailable",
    "payoutlink.already_claimed",
    "payoutlink.bad_amount",
    "payoutlink.bad_fee_bearer",
    "payoutlink.bad_id",
    "payoutlink.bad_passcode",
    "payoutlink.bad_state",
    "payoutlink.batch_too_large",
    "payoutlink.cancelled",
    "payoutlink.claim_in_progress",
    "payoutlink.destination_internal",
    "payoutlink.disabled",
    "payoutlink.duplicate_reference",
    "payoutlink.empty_batch",
    "payoutlink.expired",
    "payoutlink.funds_maturing",
    "payoutlink.idempotency_required",
    "payoutlink.insufficient_funds",
    "payoutlink.no_address",
    "payoutlink.not_found",
    "payoutlink.not_funded",
    "payoutlink.passcode",
    "payoutlink.passcode_locked",
    "payoutlink.passcode_required",
    "payoutlink.passcode_wrong",
    "payoutlink.reference_required",
    "payoutlink.token",
    "payoutlink.unavailable",
    "payoutlink.unsupported_network",
    "personal.amount_invalid",
    "personal.amount_too_small",
    "personal.asset_mismatch",
    "personal.bad_amount",
    "personal.bad_deposit_id",
    "personal.bad_fund_id",
    "personal.bad_hold_amount",
    "personal.bad_hold_id",
    "personal.bad_source",
    "personal.bad_source_id",
    "personal.bad_stake_amount",
    "personal.conversions_frozen",
    "personal.convert_unsupported",
    "personal.funds_maturing",
    "personal.hold_claimed",
    "personal.hold_idem_reuse",
    "personal.hold_kind",
    "personal.hold_no_recipient",
    "personal.hold_not_found",
    "personal.hold_settled",
    "personal.idempotency_conflict",
    "personal.insufficient",
    "personal.interest_excessive",
    "personal.no_idem",
    "personal.no_recipient",
    "personal.no_user",
    "personal.recipient_not_found",
    "personal.self_transfer",
    "personal.stake_not_found",
    "postgres.lock_pool_busy",
    "qr.no_address",
    "quarantine.bad_amount",
    "quarantine.credit_reversed",
    "quarantine.not_open",
    "quarantine.test_merchant",
    "quarantine.unknown_asset",
    "quarantine.unknown_merchant",
    "rate.unavailable",
    "rates.deviation",
    "rates.fiat_pay_asset",
    "rates.no_pay_asset",
    "rates.no_source",
    "rates.non_positive",
    "rates.stale_rate",
    "rates.unavailable",
    "referral.disabled",
    "referral.no_code",
    "referral.no_tiers",
    "referral.out_of_range",
    "refund.bad_amount",
    "refund.chain_ambiguous",
    "refund.destination_internal",
    "refund.dust",
    "refund.exceeds_excess",
    "refund.exceeds_refundable",
    "refund.fence_check",
    "refund.no_address",
    "refund.nothing_to_refund",
    "refund.paid_internally",
    "refund.reference_collision",
    "refund.too_many_attempts",
    "report.too_large",
    "request.bad_id",
    "request.bad_json",
    "request.body_read",
    "request.conflicting_field",
    "request.control_char",
    "request.duplicate_field",
    "request.method_not_allowed",
    "request.missing_field",
    "request.not_found",
    "request.nul_byte",
    "request.rate_limited",
    "request.reference_invalid",
    "request.reference_too_long",
    "request.too_deep",
    "request.unknown_currency",
    "request.unreadable",
    "resolution.already_refunded",
    "resolution.already_resolved",
    "resolution.bad_action",
    "resolution.chain_ambiguous",
    "resolution.disabled",
    "resolution.not_underpaid",
    "routing.awaiting_settle",
    "routing.onramp_check_failed",
    "routing.refund_in_flight",
    "routing.refund_payer_unresolved",
    "routing.share_payout_check_failed",
    "routing.share_reattempts_exhausted",
    "routing.underpay_unresolved",
    "sandbox.amount_too_large",
    "sandbox.bad_amount",
    "sandbox.bad_asset",
    "sandbox.bad_delivery",
    "sandbox.bad_invoice",
    "sandbox.convert_not_available",
    "sandbox.delivery_not_found",
    "sandbox.invoice_not_found",
    "sandbox.live_key",
    "sandbox.transfer_not_available",
    "split.bad_destination",
    "split.bad_hold",
    "split.bad_id",
    "split.bad_merchant",
    "split.bad_percent",
    "split.bad_reversal_claim",
    "split.consent_check_failed",
    "split.dest_check_failed",
    "split.dest_not_found",
    "split.disabled",
    "split.duplicate_destination",
    "split.network_required",
    "split.not_found",
    "split.recipient_not_opted_in",
    "split.self_destination",
    "statement.bad_from",
    "statement.bad_range",
    "statement.bad_to",
    "statement.range_too_long",
    "statement.unavailable",
    "transfer.bad_amount",
    "transfer.bad_recipient",
    "transfer.no_recipient",
    "transfer.recipient_not_found",
    "treasury.already_usdt",
    "treasury.bad_amount",
    "treasury.bad_received",
    "treasury.conversion_parked",
    "treasury.convert_in_flight",
    "treasury.empty_fill",
    "treasury.exceeds_cold",
    "treasury.exceeds_free",
    "treasury.exchange_unconfigured",
    "treasury.freeze_unknown",
    "treasury.frozen",
    "treasury.inflight_unparsable",
    "treasury.no_address",
    "treasury.no_ccy_map",
    "treasury.no_free_balance",
    "treasury.no_price",
    "treasury.no_ref",
    "treasury.no_target_hot",
    "treasury.not_configured",
    "treasury.payouts_frozen",
    "treasury.ref_destination_mismatch",
    "treasury.ref_required",
    "treasury.ref_reused",
    "treasury.same_asset",
    "treasury.send_in_flight",
    "treasury.unknown_asset",
    "treasury.vrcs_book_missing",
    "tron.bad_resource",
    "tron.not_configured",
    "tron.ref_required",
    "usd.parse",
    "vrcs.read",
    "wallet.abandoned",
    "wallet.bad_before",
    "wallet.bad_uuid",
    "wallet.deposits_unavailable",
    "wallet.no_address",
    "wallet.no_network",
    "wallet.sandbox_unsupported",
    "wallet.static_disabled",
    "wallet.unsupported_network",
    "webhook.bad_currency",
    "webhook.bad_status",
    "webhook.bad_url",
    "webhook.bad_uuid",
    "webhook.no_endpoint",
    "webhook.no_url",
    "webhook.rotation_in_overlap",
    "webhook.test_failed",
];
