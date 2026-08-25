//! Response models — the shapes the gateway answers with.
//!
//! Hand-written from the reference SDK's contract and verified field by field against the golden
//! bodies in `contract/fixtures/`, which were recorded from a live gateway. Wire names are kept
//! exactly as the core spells them (snake_case, never renamed), unknown fields are tolerated so a
//! newer core does not break decoding, and a field the core sends as `null` round-trips back as
//! `null` rather than disappearing.
//!
//! Amounts are [`Money`] — a decimal string, never a float.

pub mod account;
pub mod catalog;
pub mod common;
pub mod links;
pub mod merchants;
pub mod payments;
pub mod payouts;
pub mod sandbox;
pub mod webhooks;

pub use account::{
    AcceptedMethod, AccuracyConfig, ApiAllowlist, AutoRefundConfig, AutoWithdrawRule, Balance,
    BalanceByOwner, BalanceEntry, DiscountRule, DocumentFile, DocumentJob, DocumentPeriod,
    ReferralInfo, ReferralWeek, SplitConfig, SplitOptIn, SplitRule, VrcsStatus, Wallet,
    WalletBlocked, WalletQr,
};
pub use catalog::{Currencies, CurrencyInfo, CurrencyNetwork, ExchangeRate, PricingCurrency};
pub use common::{
    double_option, BatchElement, BatchKind, BatchStatus, FeeInfo, Money, OkResult, Timestamp,
};
pub use links::{
    ClaimPreview, ClaimResult, PaymentLink, PaymentLinkCreated, PaymentLinkPayment,
    PaymentLinkToggled, PayoutLink, PublicPaymentLink,
};
pub use merchants::{ApiKeyPair, MerchantOnboarded, SandboxStore};
pub use payments::{
    BatchInfo, BatchInfoItem, BatchSubmitted, EmailSent, Payment, PaymentRefund, PaymentTx,
    PublicPayment, QrCode, Resolution, ResolutionAccepted, ResolutionRefunded, ServiceCommission,
    ServiceLimit, ServiceMethod,
};
pub use payouts::{
    PaymentFeeConfig, Payout, PayoutCalculation, PayoutFeeConfig, PayoutValidation,
    RefundFeeConfig, Transfer, TransferToPersonal, TransferToUser,
};
pub use sandbox::{FaucetResult, SandboxDeposit, SandboxReplay, SandboxReset};
pub use webhooks::{
    PaymentEvent, PayoutEvent, WalletEvent, WebhookDelivery, WebhookEndpoint, WebhookEvent,
    WebhookSecretRotated, WebhookTestResult,
};
