// GENERATED FILE - do not edit. Source: contract/contract.json (core bfca971cce71).
// Regenerate with: python3 scripts/codegen.py

#![allow(clippy::struct_excessive_bools)]
use super::enums::*;
use crate::contract::models::Money;

/// Request body of `POST /v1/api-allowlist/add`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ApiAllowlistAddRequest {
    /// IP or subnet in CIDR notation (203.0.113.7 or 203.0.113.0/24). Example: `"203.0.113.0/24"`.
    pub cidr: String,
}

/// Request body of `POST /v1/api-allowlist/enable`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ApiAllowlistEnableRequest {
    /// true — accept API calls only from listed addresses; false — the list is kept but not enforced. Example: `true`.
    pub enabled: bool,
}

/// Request body of `POST /v1/api-allowlist/remove`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ApiAllowlistRemoveRequest {
    /// IP or subnet in CIDR notation (203.0.113.7 or 203.0.113.0/24). Example: `"203.0.113.0/24"`.
    pub cidr: String,
}

/// Request body of `POST /v1/auto-withdraw/delete`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AutoWithdrawDeleteRequest {
    /// Asset whose auto-withdrawal to switch off. Example: `"USDT"`.
    pub currency: String,
}

/// Request body of `POST /v1/auto-withdraw/set`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AutoWithdrawSetRequest {
    /// Destination address (the merchant's external wallet). Example: `"TQrY8bkbpXKPt2LZbU8jqfnpFbUSF15sbx"`.
    pub address: String,
    /// Asset to withdraw automatically. Example: `"USDT"`.
    pub currency: String,
    /// Threshold: the sweep runs once the available balance of the asset reaches this amount; empty uses the network minimum. Example: `"100"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_amount: Option<Money>,
    /// Network of the destination address. Example: `"tron"`.
    pub network: Network,
}

/// Request body of `POST /v1/batch/info`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BatchInfoRequest {
    /// Batch id from the submit response. Example: `"9f4c1a2b-77de-4a55-9c1f-0e2b3d4a5f60"`.
    pub batch_id: String,
    /// How many items to return in items (pagination). Example: `100`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Offset over items. Example: `0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

/// Request body of `POST /v1/claim/{token}`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ClaimRequest {
    /// Recipient address in the payout network.
    pub address: String,
    /// Memo/tag — only for networks where it is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    /// Claim code — if the sender set one on the link. After 10 incorrect attempts the link is locked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passcode: Option<String>,
}

/// Request body of `POST /v1/documents/jobs`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DocumentsJobsRequest {
    /// File format: pdf (default) or csv. CSV is generated without layout — cheaper for large statements and loads into Excel/1C. Example: `"csv"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Start of the period, YYYY-MM-DD (defaults to the first day of the current month). Example: `"2025-01-01"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// Report type: statement (operations), fees (commissions) or ledger (balance movements). Example: `"statement"`.
    pub kind: String,
    /// Document language (default en). Example: `"ru"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// End of the period, inclusive, YYYY-MM-DD (defaults to today). The period may span up to two years. Example: `"2026-08-19"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

/// Request body of `POST /v1/documents/jobs/info`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DocumentsJobsInfoRequest {
    /// Job id from the creation response. Example: `"6f1c\u2026"`.
    pub job_id: String,
}

/// Request body of `POST /v1/exchange-rate/list`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ExchangeRateListRequest {
    /// Currency code. If set, only its rate is returned. If empty or the body is {}, rates for all currencies are returned. Example: `"ETH"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency_from: Option<String>,
    /// Quote currency: USDT by default; any pricing asset, including fiats with a direct feed (EUR, RUB, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency_to: Option<String>,
    /// Page size, 1–100; default 25.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Offset from the start of the list; default 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

/// Request body of `POST /v1/link/{id}/checkout`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LinkCheckoutRequest {
    /// Amount entered by the buyer, in the link's price currency; required for open and range, ignored for fixed. Example: `"10.00"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<Money>,
    /// Settlement currency — the coin the buyer pays with; needed only if the link did not pin pinned_currency. Example: `"USDT"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// Settlement network; needed only if the link did not pin pinned_network. Example: `"tron"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    /// Shop order number from the embedded widget (data-oblodai-order-id); carried over to the invoice and to the webhook for matching with the order; not an idempotency key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    /// Buyer email — the cheque is sent there automatically after payment. Example: `"buyer@example.com"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payer_email: Option<String>,
}

/// Request body of `POST /v1/merchants`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MerchantsRequest {
    /// Owner email; must be unique across merchants. Example: `"owner@shop.example"`.
    pub email: String,
    /// Display name of the merchant. Example: `"Acme"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Request body of `POST /v1/pay/{id}/select`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaySelectRequest {
    /// Selected payment currency. Example: `"USDT"`.
    pub currency: String,
    /// Selected network. Example: `"tron"`.
    pub network: Network,
}

/// Request body of `POST /v1/payment`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentRequest {
    /// Under/overpayment tolerance, 0–5 %. Overrides the merchant setting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accuracy_payment_percent: Option<f64>,
    /// Private merchant data, echoed back in webhooks (not visible to the buyer).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_data: Option<String>,
    /// Amount to pay, in currency. Example: `"10"`.
    pub amount: Money,
    /// Price currency code: any of the 23 fiats (USD, EUR, RUB, …) or any coin (USDT, BTC, …). JPY and KRW have zero decimal places. Example: `"USD"`.
    pub currency: String,
    /// Allow paying up the remaining amount.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_payment_multiple: Option<bool>,
    /// Revive an expired invoice by order_id instead of creating a new one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_refresh: Option<bool>,
    /// Invoice lifetime in seconds, 300–43200; default 3600. Values outside the range are clamped to the nearest bound. Example: `3600`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifetime_seconds: Option<i64>,
    /// Settlement network (e.g. tron, ethereum). Optional — see the currency and network selection modes. Example: `"tron"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    /// Merchant reference; idempotency key. Strongly recommended. Example: `"order-1"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    /// Payer email. If set, a cheque is sent there automatically after payment; it is also the default recipient for POST /v1/payment/send-email.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payer_email: Option<String>,
    /// Deprecated: % of the network markup charged to the payer (0–100); payer-facing markups are configured via discount.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtract: Option<i64>,
    /// Payment page theme: dark | light. Example: `"dark"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    /// Settlement currency — the crypto used for payment. Defaults to currency (only if currency is a coin); with a fiat price set it explicitly or omit it together with network. Example: `"USDT"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_currency: Option<String>,
    /// Per-invoice webhook. Requires a registered endpoint (POST /v1/webhooks): delivery is signed with its secret.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_callback: Option<String>,
    /// "Back to shop" link on the payment page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_return: Option<String>,
    /// Redirect after successful payment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_success: Option<String>,
}

/// Request body of `POST /v1/payment/accepted/list`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentAcceptedListRequest {
    /// Page size, 1–100; out of range falls back to 25. Example: `25`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Offset from the start of the list (newest first). Example: `0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

/// Request body of `POST /v1/payment/accepted/set`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentAcceptedSetRequest {
    /// The full list of currency+network pairs payers may use; an empty list accepts everything in the catalog.
    pub accepted: Vec<PaymentAcceptedSetAcceptedItem>,
}

/// Item of `PaymentAcceptedSetRequest::accepted`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentAcceptedSetAcceptedItem {
    /// Asset code. Example: `"USDT"`.
    pub currency: String,
    /// Asset network. Example: `"tron"`.
    pub network: Network,
}

/// Request body of `POST /v1/payment/accuracy/set`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentAccuracySetRequest {
    /// Tolerance in percent, 1–5. Required when enabled: true; ignored when enabled: false (reset to 0). Capped at 5 %. Example: `2`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accuracy_percent: Option<i64>,
    /// Enable/disable the tolerance. Example: `true`.
    pub enabled: bool,
}

/// Request body of `POST /v1/payment/autorefund/set`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentAutorefundSetRequest {
    /// Refund the excess on overpayment (paid_over). Example: `true`.
    pub overpay: bool,
    /// Refund the funds on an expired underpayment (wrong_amount). Example: `true`.
    pub underpay: bool,
}

/// Request body of `POST /v1/payment/batch`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentBatchRequest {
    /// What to do when an item fails: continue (default) — process the rest; stop — halt processing after the first error. Example: `"continue"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_error: Option<BatchOnError>,
    /// Array of 1 to 5000 items — the same fields as POST /v1/payment; set order_id on every item: results are matched by it and it protects against duplicates.
    pub payments: Vec<PaymentBatchPaymentsItem>,
}

/// Item of `PaymentBatchRequest::payments`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentBatchPaymentsItem {
    /// Under/overpayment tolerance, 0–5 %. Overrides the merchant setting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accuracy_payment_percent: Option<f64>,
    /// Private merchant data, echoed back in webhooks (not visible to the buyer).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additional_data: Option<String>,
    /// Amount to pay, in currency. Example: `"10"`.
    pub amount: Money,
    /// Price currency code: any of the 23 fiats (USD, EUR, RUB, …) or any coin (USDT, BTC, …). JPY and KRW have zero decimal places. Example: `"USD"`.
    pub currency: String,
    /// Allow paying up the remaining amount.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_payment_multiple: Option<bool>,
    /// Revive an expired invoice by order_id instead of creating a new one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_refresh: Option<bool>,
    /// Invoice lifetime in seconds, 300–43200; default 3600. Values outside the range are clamped to the nearest bound. Example: `3600`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifetime_seconds: Option<i64>,
    /// Settlement network (e.g. tron, ethereum). Optional — see the currency and network selection modes. Example: `"tron"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    /// Merchant reference; idempotency key. Strongly recommended. Example: `"order-1"`.
    pub order_id: String,
    /// Payer email. If set, a cheque is sent there automatically after payment; it is also the default recipient for POST /v1/payment/send-email.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payer_email: Option<String>,
    /// Deprecated: % of the network markup charged to the payer (0–100); payer-facing markups are configured via discount.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtract: Option<i64>,
    /// Payment page theme: dark | light. Example: `"dark"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    /// Settlement currency — the crypto used for payment. Defaults to currency (only if currency is a coin); with a fiat price set it explicitly or omit it together with network. Example: `"USDT"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_currency: Option<String>,
    /// Per-invoice webhook. Requires a registered endpoint (POST /v1/webhooks): delivery is signed with its secret.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_callback: Option<String>,
    /// "Back to shop" link on the payment page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_return: Option<String>,
    /// Redirect after successful payment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_success: Option<String>,
}

/// Request body of `POST /v1/payment/cancel`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentCancelRequest {
    /// Your order reference. Example: `"order-1"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    /// Invoice id in Oblodai. Either uuid or order_id is required; uuid takes priority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

/// Request body of `POST /v1/payment/discount/list`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentDiscountListRequest {
    /// Page size, 1–100; out of range falls back to 25. Example: `25`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Offset from the start of the list (newest first). Example: `0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

/// Request body of `POST /v1/payment/discount/set`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentDiscountSetRequest {
    /// Currency. Empty = global default for all coins. Example: `"USDT"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// Percentage, from -99 to 99. Positive is a discount, negative is a markup. Example: `3`.
    pub discount_percent: i64,
    /// Network. Empty = any network of this currency. Example: `"tron"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
}

/// Request body of `POST /v1/payment/fee-config/set`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentFeeConfigSetRequest {
    /// Share of OUR commission paid by the buyer: 0 — the merchant pays (current behaviour), 100 — the buyer pays and the invoice is issued with a markup. Applies to invoices created AFTER the change. Example: `100`.
    pub payer_pays_percent: i64,
}

/// Request body of `POST /v1/payment/history`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentHistoryRequest {
    /// Ignored on this route (payout history only). Example: `"payout"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Page size, 1–100; out of range falls back to 25. Example: `25`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Offset from the start of the list (newest first). Example: `0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
    /// Filter by status (an exact value from the status vocabulary); empty returns all. Example: `"paid"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<PaymentStatus>,
}

/// Request body of `POST /v1/payment/info`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentInfoRequest {
    /// Your order reference. Example: `"order-1"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    /// Invoice id in Oblodai. Either uuid or order_id is required; uuid takes priority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

/// Request body of `POST /v1/payment/link`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentLinkRequest {
    /// Amount — for fixed mode; required in this mode. Example: `"25.00"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_fixed: Option<Money>,
    /// Amount mode: fixed | open | range. Example: `"open"`.
    pub amount_mode: AmountMode,
    /// Price currency — fiat (USD, EUR, RUB, …) or a coin; see pricing_currencies from GET /v1/currencies. Example: `"USD"`.
    pub currency: String,
    /// Description on the payment page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Link lifetime in seconds from creation; 0 (default) — the link never expires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_in_seconds: Option<i64>,
    /// Upper bound — for range; required in this mode. Example: `"1000.00"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_amount: Option<Money>,
    /// Lower bound: an optional floor for open, a required minimum for range. Example: `"1.00"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_amount: Option<Money>,
    /// Settlement currency (coin) pinned to the link; empty — the buyer chooses the coin. Example: `"USDT"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_currency: Option<String>,
    /// Settlement network pinned to the link; empty — the buyer chooses the network. Example: `"tron"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_network: Option<Network>,
    /// Title on the payment page. Example: `"\u041f\u043e\u0434\u0434\u0435\u0440\u0436\u0430\u0442\u044c \u043f\u0440\u043e\u0435\u043a\u0442"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Request body of `POST /v1/payment/link/info`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentLinkInfoRequest {
    /// Page size for the link's payments, 1–100; out of range falls back to 25. Example: `25`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Payment link identifier. Example: `"5d3f2a71-9c84-4b0e-8d17-3e6a2c9f1b40"`.
    pub link_id: String,
    /// Offset within the link's payments. Example: `0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

/// Request body of `POST /v1/payment/link/list`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentLinkListRequest {
    /// Page size, 1–100; out of range falls back to 25. Example: `25`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Offset from the start of the list (newest first). Example: `0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

/// Request body of `POST /v1/payment/link/toggle`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentLinkToggleRequest {
    /// true — the link accepts payments; false — disabled (the page shows the link as inactive). Example: `false`.
    pub active: bool,
    /// Payment link identifier. Example: `"5d3f2a71-9c84-4b0e-8d17-3e6a2c9f1b40"`.
    pub link_id: String,
}

/// Request body of `POST /v1/payment/qr`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentQrRequest {
    /// Your order reference. Example: `"order-1"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    /// Invoice id in Oblodai. Either uuid or order_id is required; uuid takes priority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

/// Request body of `POST /v1/payment/refund`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentRefundRequest {
    /// Refund destination address. Defaults to the payment's payer_address; required only for Bitcoin/UTXO.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// Partial amount. Defaults to the full amount received. Example: `"10"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<Money>,
    /// Network. Example: `"tron"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    /// Your order reference for the payment. Either uuid or order_id is required. Example: `"order-1"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    /// Optional refund idempotency key: distinguishes two different refunds with the same (payment, address, amount); a repeat with the same value is deduplicated. This is not order_id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// Payment id. Either uuid or order_id is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

/// Request body of `POST /v1/payment/resend`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentResendRequest {
    /// Your order reference. Example: `"order-1"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    /// Invoice id in Oblodai. Either uuid or order_id is required; uuid takes priority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

/// Request body of `POST /v1/payment/resolve`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentResolveRequest {
    /// accept — accept the partial payment, refund — return it to the payer. Example: `"accept"`.
    pub action: ResolveAction,
    /// refund only: refund address. Defaults to the payment's recorded payer_address; if it is empty (Bitcoin/UTXO) the address is required, otherwise refund.no_address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// refund only: refund network, defaults to the payment network.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    /// Your payment identifier. Example: `"ord-1001"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    /// refund only: your refund deduplication key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// Payment UUID. Either uuid or order_id is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

/// Request body of `POST /v1/payment/send-email`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentSendEmailRequest {
    /// Where to send it. Defaults to the payer_email set on the payment. Example: `"buyer@example.com"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Your order reference. Example: `"order-1"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    /// Payment id in Oblodai. Either uuid or order_id is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

/// Request body of `POST /v1/payment/services`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentServicesRequest {
    /// Page size, 1–100; out of range falls back to 25. Example: `25`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Offset from the start of the list (newest first). Example: `0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

/// Request body of `POST /v1/payment/testing-webhook`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PaymentTestingWebhookRequest {
    /// Status in the body. Default paid. Example: `"paid"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<PaymentStatus>,
    /// Where to send the test body. If not provided, delivery goes to the project's registered endpoint; without an endpoint it fails with webhook.no_endpoint. Signed with the project endpoint's secret, including when url is passed explicitly. Example: `"https://shop.example/hook"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Request body of `POST /v1/payout`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutRequest {
    /// Recipient address.
    pub address: String,
    /// Payout amount, in currency. Example: `"25"`.
    pub amount: Money,
    /// Currency code (for example USDT). Example: `"USDT"`.
    pub currency: String,
    /// Fund the payout by converting the balance. USDT → currency only. Example: `"USDT"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_currency: Option<String>,
    /// Who pays the network fee: true — amount+fee is debited from the balance and the recipient receives amount; false — the recipient receives amount-fee; not provided — the project fee-config.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_subtract: Option<bool>,
    /// Destination tag/memo (TON Jetton). Maximum 120 characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    /// Network (tron, ethereum, …). Required for coins with several networks. Example: `"tron"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    /// Your payout number; idempotency key. Example: `"payout-1"`.
    pub order_id: String,
    /// Origin label: api (default) or manual.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Custom webhook URL for this payout (passes the SSRF check). Requires a registered endpoint (POST /v1/webhooks): delivery is signed with its secret.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_callback: Option<String>,
}

/// Request body of `POST /v1/payout/approve`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutApproveRequest {
    /// Payout id.
    pub uuid: String,
}

/// Request body of `POST /v1/payout/batch`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutBatchRequest {
    /// What to do when an item fails: continue (default) — process the rest; stop — halt processing after the first error. Example: `"continue"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_error: Option<BatchOnError>,
    /// Array of 1 to 5000 items — the same fields as POST /v1/payout; order_id is required on every item and serves as the idempotency key: a repeat returns the already created payout.
    pub payouts: Vec<PayoutBatchPayoutsItem>,
}

/// Item of `PayoutBatchRequest::payouts`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutBatchPayoutsItem {
    /// Recipient address.
    pub address: String,
    /// Payout amount, in currency. Example: `"25"`.
    pub amount: Money,
    /// Currency code (for example USDT). Example: `"USDT"`.
    pub currency: String,
    /// Fund the payout by converting the balance. USDT → currency only. Example: `"USDT"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_currency: Option<String>,
    /// Who pays the network fee: true — amount+fee is debited from the balance and the recipient receives amount; false — the recipient receives amount-fee; not provided — the project fee-config.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_subtract: Option<bool>,
    /// Destination tag/memo (TON Jetton). Maximum 120 characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    /// Network (tron, ethereum, …). Required for coins with several networks. Example: `"tron"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    /// Your payout number; idempotency key. Example: `"payout-1"`.
    pub order_id: String,
    /// Origin label: api (default) or manual.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Custom webhook URL for this payout (passes the SSRF check). Requires a registered endpoint (POST /v1/webhooks): delivery is signed with its secret.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_callback: Option<String>,
}

/// Request body of `POST /v1/payout/calculate`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutCalculateRequest {
    /// Payout amount as a decimal string. Example: `"10"`.
    pub amount: Money,
    /// Payout asset (USDT, BTC, …). Example: `"USDT"`.
    pub currency: String,
    /// true — the fee is debited from the balance on top of the amount (the recipient gets exactly amount); false — the fee is taken out of the payout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_subtract: Option<bool>,
    /// Payout network; required when the asset lives on several networks. Example: `"tron"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
}

/// Request body of `POST /v1/payout/cancel`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutCancelRequest {
    /// Id of the payout (or refund) to cancel.
    pub uuid: String,
}

/// Request body of `POST /v1/payout/fee-config/set`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutFeeConfigSetRequest {
    /// true — the recipient pays the network fee (receives less); false — the merchant bears the fee. Example: `true`.
    pub fee_on_recipient: bool,
}

/// Request body of `POST /v1/payout/history`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutHistoryRequest {
    /// payout — ordinary payouts, refund — refunds; empty returns both. Example: `"payout"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<PayoutKind>,
    /// Page size, 1–100; out of range falls back to 25. Example: `25`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Offset from the start of the list (newest first). Example: `0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
    /// Filter by status (an exact value from the status vocabulary); empty returns all. Example: `"paid"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<PayoutStatus>,
}

/// Request body of `POST /v1/payout/info`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutInfoRequest {
    /// Your order reference. Example: `"order-1"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    /// Invoice id in Oblodai. Either uuid or order_id is required; uuid takes priority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

/// Request body of `POST /v1/payout/link`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutLinkRequest {
    /// Amount in currency, as a string; greater than zero. Example: `"25"`.
    pub amount: Money,
    /// Payout crypto asset (USDT, BTC, …); fiat is not possible. Example: `"USDT"`.
    pub currency: String,
    /// If set, the recipient receives an email with a "Claim funds" button; a delivery failure does not cancel link creation. Example: `"user@example.com"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Link lifetime in seconds, clamped to 3600–2592000 (one hour to 30 days); without the field or with 0 the link lives 1 hour, not the maximum — set it explicitly. Example: `604800`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_in_seconds: Option<i64>,
    /// Who pays the network fee: "recipient" (default — deducted from the amount, the recipient receives less) or "merchant" (the amount plus the fee is reserved, the recipient receives exactly amount). Example: `"merchant"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fee_bearer: Option<FeeBearer>,
    /// Payout network for the recipient (tron, bitcoin, …). Example: `"tron"`.
    pub network: Network,
    /// Message to the recipient (shown on the claim page and in the email). Example: `"\u0421\u043f\u0430\u0441\u0438\u0431\u043e \u0437\u0430 \u0443\u0447\u0430\u0441\u0442\u0438\u0435"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Claim code — a second factor for the link: "auto" — we generate it and return it ONCE in the response, or your own (6–64 visible characters), empty — no code. Pass the code to the recipient over a channel SEPARATE from the link (it is not put into the email); after 10 incorrect attempts the link is locked. Example: `"auto"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passcode: Option<String>,
    /// Your deduplication key, unique per merchant; the Idempotency-Key header has no effect on this endpoint. Example: `"bonus-42"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// Title — shown to the recipient on the claim page. Example: `"\u0411\u043e\u043d\u0443\u0441"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Request body of `POST /v1/payout/link/batch`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutLinkBatchRequest {
    /// Up to 500 links per call; each one succeeds or fails independently, the response is aligned with the request indexes.
    pub items: Vec<PayoutLinkBatchItemsItem>,
}

/// Item of `PayoutLinkBatchRequest::items`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutLinkBatchItemsItem {
    /// Amount in currency, as a string; greater than zero. Example: `"25"`.
    pub amount: Money,
    /// Payout crypto asset (USDT, BTC, …); fiat is not possible. Example: `"USDT"`.
    pub currency: String,
    /// If set, the recipient receives an email with a "Claim funds" button; a delivery failure does not cancel link creation. Example: `"user@example.com"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Link lifetime in seconds, clamped to 3600–2592000 (one hour to 30 days); without the field or with 0 the link lives 1 hour, not the maximum — set it explicitly. Example: `604800`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_in_seconds: Option<i64>,
    /// Who pays the network fee: "recipient" (default — deducted from the amount, the recipient receives less) or "merchant" (the amount plus the fee is reserved, the recipient receives exactly amount). Example: `"merchant"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fee_bearer: Option<FeeBearer>,
    /// Payout network for the recipient (tron, bitcoin, …). Example: `"tron"`.
    pub network: Network,
    /// Message to the recipient (shown on the claim page and in the email). Example: `"\u0421\u043f\u0430\u0441\u0438\u0431\u043e \u0437\u0430 \u0443\u0447\u0430\u0441\u0442\u0438\u0435"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Claim code — a second factor for the link: "auto" — we generate it and return it ONCE in the response, or your own (6–64 visible characters), empty — no code. Pass the code to the recipient over a channel SEPARATE from the link (it is not put into the email); after 10 incorrect attempts the link is locked. Example: `"auto"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passcode: Option<String>,
    /// Your deduplication key, unique per merchant; the Idempotency-Key header has no effect on this endpoint. Example: `"bonus-42"`.
    pub reference: String,
    /// Title — shown to the recipient on the claim page. Example: `"\u0411\u043e\u043d\u0443\u0441"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Request body of `POST /v1/payout/link/cancel`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutLinkCancelRequest {
    /// Payout link id (link_id from the creation response).
    pub link_id: String,
}

/// Request body of `POST /v1/payout/link/cheque`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutLinkChequeRequest {
    /// Claim secret from the payout link creation response. Stored only as a hash and never reissued — the cheque can be printed only while you still hold the token. Example: `"nUqx1yG3\u2026"`.
    pub claim_token: String,
    /// Document language — one of the 41 supported codes (en by default); the full list is in the document.unknown_lang error. Example: `"ru"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
}

/// Request body of `POST /v1/payout/link/info`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutLinkInfoRequest {
    /// Payout link id (link_id from the creation response).
    pub link_id: String,
}

/// Request body of `POST /v1/payout/link/list`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutLinkListRequest {
    /// How many links to return per page. Example: `50`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Offset from the start of the list (paging). Example: `0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

/// Request body of `POST /v1/payout/mass`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutMassRequest {
    /// Array of up to 100 items; the fields of each are as in POST /v1/payout.
    pub payouts: Vec<PayoutMassPayoutsItem>,
    /// Origin label, applied to every item without its own source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Item of `PayoutMassRequest::payouts`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutMassPayoutsItem {
    /// Recipient address.
    pub address: String,
    /// Payout amount, in currency. Example: `"25"`.
    pub amount: Money,
    /// Currency code (for example USDT). Example: `"USDT"`.
    pub currency: String,
    /// Fund the payout by converting the balance. USDT → currency only. Example: `"USDT"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_currency: Option<String>,
    /// Who pays the network fee: true — amount+fee is debited from the balance and the recipient receives amount; false — the recipient receives amount-fee; not provided — the project fee-config.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_subtract: Option<bool>,
    /// Destination tag/memo (TON Jetton). Maximum 120 characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    /// Network (tron, ethereum, …). Required for coins with several networks. Example: `"tron"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    /// Your payout number; idempotency key. Example: `"payout-1"`.
    pub order_id: String,
    /// Origin label: api (default) or manual.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Custom webhook URL for this payout (passes the SSRF check). Requires a registered endpoint (POST /v1/webhooks): delivery is signed with its secret.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_callback: Option<String>,
}

/// Request body of `POST /v1/payout/refund-fee-config/set`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutRefundFeeConfigSetRequest {
    /// true — the customer receives net (the customer pays the fee); false — the merchant pays the fee and the customer receives gross. Example: `true`.
    pub fee_on_customer: bool,
}

/// Request body of `POST /v1/payout/services`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutServicesRequest {
    /// Page size, 1–100; out of range falls back to 25. Example: `25`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Offset from the start of the list (newest first). Example: `0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

/// Request body of `POST /v1/payout/validate`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayoutValidateRequest {
    /// Recipient address.
    pub address: String,
    /// Payout amount, in currency. Example: `"25"`.
    pub amount: Money,
    /// Currency code (for example USDT). Example: `"USDT"`.
    pub currency: String,
    /// Fund the payout by converting the balance. USDT → currency only. Example: `"USDT"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_currency: Option<String>,
    /// Who pays the network fee: true — amount+fee is debited from the balance and the recipient receives amount; false — the recipient receives amount-fee; not provided — the project fee-config.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_subtract: Option<bool>,
    /// Destination tag/memo (TON Jetton). Maximum 120 characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    /// Network (tron, ethereum, …). Required for coins with several networks. Example: `"tron"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    /// Your payout number; idempotency key. Example: `"payout-1"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    /// Origin label: api (default) or manual.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Custom webhook URL for this payout (passes the SSRF check). Requires a registered endpoint (POST /v1/webhooks): delivery is signed with its secret.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_callback: Option<String>,
}

/// Request body of `POST /v1/refund/batch`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RefundBatchRequest {
    /// What to do when an item fails: continue (default) — process the rest; stop — halt processing after the first error. Example: `"continue"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_error: Option<BatchOnError>,
    /// Array of 1 to 5000 items — the same fields as POST /v1/payment/refund; every item requires reference (idempotency key) and either uuid or order_id of the payment.
    pub refunds: Vec<RefundBatchRefundsItem>,
}

/// Item of `RefundBatchRequest::refunds`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RefundBatchRefundsItem {
    /// Refund destination address. Defaults to the payment's payer_address; required only for Bitcoin/UTXO.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// Partial amount. Defaults to the full amount received. Example: `"10"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<Money>,
    /// Network. Example: `"tron"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    /// Your order reference for the payment. Either uuid or order_id is required. Example: `"order-1"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    /// Optional refund idempotency key: distinguishes two different refunds with the same (payment, address, amount); a repeat with the same value is deduplicated. This is not order_id.
    pub reference: String,
    /// Payment id. Either uuid or order_id is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

/// Request body of `POST /v1/sandbox/deposit`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SandboxDepositRequest {
    /// Amount in the invoice currency; empty — pay exactly what is due, anything else is a way to produce an under/overpayment. Example: `"10"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<Money>,
    /// How many confirmations the deposit arrived with; 0 — fully confirmed; fewer than required — a way to test the pending→confirmed transition (repeat the same txid with a higher number). Example: `0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirmations: Option<i64>,
    /// UUID of the test invoice being "paid".
    pub invoice_id: String,
    /// Repeating the same txid tests your idempotency; empty — a new txid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub txid: Option<String>,
}

/// Request body of `POST /v1/sandbox/faucet`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SandboxFaucetRequest {
    /// Amount of test money, as a string; capped at 1000000 per call. Example: `"1000"`.
    pub amount: Money,
    /// Top-up asset (USDT, BTC, …). Example: `"USDT"`.
    pub asset: String,
    /// Safe-retry key; empty — every call creates a new top-up.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

/// Request body of `POST /v1/sandbox/webhooks/replay`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SandboxWebhooksReplayRequest {
    /// Delivery id from GET /v1/sandbox/webhooks.
    pub delivery_id: String,
}

/// Request body of `POST /v1/split/config/set`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SplitConfigSetRequest {
    /// How many seconds to defer split settlement; range 0–7776000 (up to 90 days). 0 — send the shares immediately: you take on the risk that a refund becomes impossible. Example: `172800`.
    pub refund_hold_seconds: i64,
}

/// Request body of `POST /v1/split/recipient/optin`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SplitRecipientOptinRequest {
    /// Allow other merchants to route split shares to your balance. true — enable receiving, false — disable (new rules targeting you stop being created; existing ones keep executing). Example: `true`.
    pub enabled: bool,
}

/// Request body of `POST /v1/split/rule`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SplitRuleRequest {
    /// External crypto address of the partner; the share leaves as a real on-chain transaction — irreversible. Exactly one recipient option: either address+network or merchant_id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    /// Id of the partner merchant inside Oblodai; the share moves through internal accounting and is clawed back on a refund. Example: `"b4c1f0e2-5a77-4d31-9f08-2c6e7a1b3d94"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merchant_id: Option<String>,
    /// Address network. Required together with address. Example: `"tron"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    /// Comment for yourself (shown in the rule list).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// Share of every payment, as a string: "10" = 10 %, "2.5" = 2.5 %. Greater than 0 and at most 100, step 0.01 %; the sum of all rules cannot exceed 100 %. Example: `"10"`.
    pub percent: String,
}

/// Request body of `POST /v1/split/rule/delete`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SplitRuleDeleteRequest {
    /// Rule identifier from POST /v1/split/rule or the list. Example: `"9f4c1a2b-77de-4a55-9c1f-0e2b3d4a5f60"`.
    pub rule_id: String,
}

/// Request body of `POST /v1/split/rule/list`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SplitRuleListRequest {
    /// Page size, 1–100; out of range falls back to 25. Example: `25`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Offset from the start of the list (newest first). Example: `0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

/// Request body of `POST /v1/test-webhook/payment`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TestWebhookPaymentRequest {
    /// Currency in the body. Example: `"USDT"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// Network in the body. Example: `"tron"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    /// Your order_id, which is put into the test event body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    /// Status in the body — from the status dictionary of this event type. Default paid (confirmed for a payout). Example: `"paid"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<PaymentStatus>,
    /// Where to send the test body. Example: `"https://shop.example/oblodai/callback"`.
    pub url_callback: String,
    /// UUID of the object (payment, wallet or payout) put into the test event body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

/// Request body of `POST /v1/test-webhook/payout`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TestWebhookPayoutRequest {
    /// Currency in the body. Example: `"USDT"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// Network in the body. Example: `"tron"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    /// Your order_id, which is put into the test event body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    /// Status in the body — from the status dictionary of this event type. Default paid (confirmed for a payout). Example: `"paid"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<PayoutStatus>,
    /// Where to send the test body. Example: `"https://shop.example/oblodai/callback"`.
    pub url_callback: String,
    /// UUID of the object (payment, wallet or payout) put into the test event body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

/// Request body of `POST /v1/test-webhook/wallet`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TestWebhookWalletRequest {
    /// Currency in the body. Example: `"USDT"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// Network in the body. Example: `"tron"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    /// Your order_id, which is put into the test event body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    /// Status in the body — from the status dictionary of this event type. Default paid (confirmed for a payout). Example: `"paid"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Where to send the test body. Example: `"https://shop.example/oblodai/callback"`.
    pub url_callback: String,
    /// UUID of the object (payment, wallet or payout) put into the test event body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

/// Request body of `POST /v1/transfer/batch`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TransferBatchRequest {
    /// What to do when an item fails: continue (default) — process the rest; stop — halt processing after the first error. Example: `"continue"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_error: Option<BatchOnError>,
    /// Array of 1 to 5000 items — the same fields as POST /v1/transfer/to-user; every item requires order_id (idempotency key) and to_user_id (user UUID).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transfers: Option<Vec<TransferBatchTransfersItem>>,
}

/// Item of `TransferBatchRequest::transfers`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TransferBatchTransfersItem {
    /// Transfer amount in currency. Example: `"50"`.
    pub amount: Money,
    /// Currency code (cryptocurrency). Example: `"USDT"`.
    pub currency: String,
    /// Idempotency key: a repeat with the same order_id is a no-op; required in a transfer batch.
    pub order_id: String,
    /// Platform user id of the recipient (UUID, not username); a username is resolved to an id via the cabinet public profile /public/users/{username}.
    pub to_user_id: String,
}

/// Request body of `POST /v1/transfer/to-personal`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TransferToPersonalRequest {
    /// Transfer amount in currency. Example: `"50"`.
    pub amount: Money,
    /// Currency code (cryptocurrency). Example: `"USDT"`.
    pub currency: String,
    /// Idempotency key: a repeat with the same order_id is a no-op. Always send it, otherwise retrying the request after a network timeout creates a second transfer. Example: `"transfer-1"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
}

/// Request body of `POST /v1/transfer/to-user`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TransferToUserRequest {
    /// Transfer amount in currency. Example: `"50"`.
    pub amount: Money,
    /// Currency code (cryptocurrency). Example: `"USDT"`.
    pub currency: String,
    /// Idempotency key: a repeat with the same order_id is a no-op; required in a transfer batch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    /// Platform user id of the recipient (UUID, not username); a username is resolved to an id via the cabinet public profile /public/users/{username}.
    pub to_user_id: String,
}

/// Request body of `POST /v1/vrcs`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VrcsRequest {
    /// true — enable auto-conversion of volatile deposits to USDT, false — disable; omit to read the current state. Example: `true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

/// Request body of `POST /v1/wallet`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WalletRequest {
    /// Symbol of the receiving currency (USDT, BTC, ETH, …). Example: `"USDT"`.
    pub currency: String,
    /// Receiving network (tron, ethereum, bitcoin, …). Example: `"tron"`.
    pub network: Network,
    /// Your customer/order identifier. Pins a dedicated permanent address to the customer. Example: `"client-42"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
}

/// Request body of `POST /v1/wallet/block`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WalletBlockRequest {
    /// Static wallet address. Example: `"TXk9...c3Fd"`.
    pub address: String,
    /// true — block (the default when the field is omitted); false — unblock.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_force_block: Option<bool>,
}

/// Request body of `POST /v1/wallet/blocked-address-refund`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WalletBlockedAddressRefundRequest {
    /// Refund destination address.
    pub address: String,
    /// Destination tag/memo (XRP destination tag, XLM memo id, TON comment). Required for a classic address on a tag/memo network if the tag is not embedded in the X-/M-address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    /// Static wallet id (from the /v1/wallet response).
    pub uuid: String,
}

/// Request body of `POST /v1/wallet/qr`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WalletQrRequest {
    /// Arbitrary address to render into a QR code (PNG as a data: URI).
    pub address: String,
}

/// Request body of `POST /v1/webhooks`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WebhooksRequest {
    /// HTTPS callback URL. SSRF check: private and local addresses are rejected. Example: `"https://shop.example/oblodai/callback"`.
    pub url: String,
}

/// Request body of `POST /v1/webhooks/deliveries`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WebhooksDeliveriesRequest {
    /// Page size, 1–100; out of range falls back to 25. Example: `25`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    /// Offset from the start of the list (newest first). Example: `0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}
