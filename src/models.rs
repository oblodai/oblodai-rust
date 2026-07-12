//! Модели объектов API.
//!
//! Суммы — строки в единицах валюты (`"25.00"`), не числа, — чтобы не терять точность. Все структуры
//! допускают отсутствие полей (`#[serde(default)]`), поэтому дополнительные или пропущенные поля не
//! ломают разбор.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Объект платежа (инвойса).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Payment {
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub order_id: String,
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub payment_amount: Option<String>,
    #[serde(default)]
    pub amount_paid: String,
    #[serde(default)]
    pub amount_remaining: String,
    #[serde(default)]
    pub payer_amount: String,
    #[serde(default)]
    pub payer_currency: String,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub address_qr_code: String,
    #[serde(default)]
    pub payment_status: String,
    #[serde(default)]
    pub is_multi: bool,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub expired_at: i64,
    #[serde(default)]
    pub is_final: bool,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub additional_data: String,
    #[serde(default)]
    pub payer_email: String,
    #[serde(default)]
    pub url_return: String,
    #[serde(default)]
    pub url_success: String,
    #[serde(default)]
    pub rate_expires_at: i64,
    #[serde(default)]
    pub confirmations: i64,
    #[serde(default)]
    pub required_confirmations: i64,
    #[serde(default)]
    pub txid: String,
}

/// Пагинация в списковых ответах.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Paginate {
    #[serde(default)]
    pub count: i64,
    #[serde(default)]
    pub per_page: i64,
    #[serde(default)]
    pub offset: i64,
}

/// Страница списка платежей.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PaymentList {
    #[serde(default)]
    pub items: Vec<Payment>,
    #[serde(default)]
    pub paginate: Paginate,
}

/// Лимиты метода.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Limit {
    #[serde(default)]
    pub min_amount: String,
    #[serde(default)]
    pub max_amount: String,
}

/// Комиссия метода.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Commission {
    #[serde(default)]
    pub fee_amount: String,
    #[serde(default)]
    pub percent: String,
}

/// Доступный метод приёма/выплат.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ServiceMethod {
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub is_available: bool,
    #[serde(default)]
    pub limit: Limit,
    #[serde(default)]
    pub commission: Commission,
}

/// Статический кошелёк.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Wallet {
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub order_id: String,
    #[serde(default)]
    pub url: String,
}

/// Баланс по одной валюте.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MerchantBalance {
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub balance: String,
}

/// Доступные балансы мерчанта.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Balance {
    #[serde(default)]
    pub merchant: Vec<MerchantBalance>,
}

/// Реферальная статистика.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ReferralInfo {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub link: String,
    #[serde(default)]
    pub tier_bps: Vec<i64>,
    #[serde(default)]
    pub referred_count: i64,
    #[serde(default)]
    pub earnings_by_asset: HashMap<String, String>,
}

/// Данные конвертации (при `from_currency`).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PayoutConvert {
    #[serde(default)]
    pub from_currency: String,
    #[serde(default)]
    pub to_currency: String,
    #[serde(default)]
    pub from_amount: String,
    #[serde(default)]
    pub rate: String,
}

/// Объект выплаты.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Payout {
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub order_id: String,
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub txid: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub is_final: bool,
    #[serde(default)]
    pub approval_required: bool,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub convert: Option<PayoutConvert>,
}

/// Элемент результата массовой выплаты.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MassPayoutItem {
    #[serde(default)]
    pub order_id: String,
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub is_final: bool,
    #[serde(default)]
    pub approval_required: bool,
    #[serde(default)]
    pub message: String,
}

/// Результат массовой выплаты.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MassPayoutResult {
    #[serde(default)]
    pub items: Vec<MassPayoutItem>,
}

/// Страница истории выплат.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PayoutList {
    #[serde(default)]
    pub items: Vec<Payout>,
    #[serde(default)]
    pub paginate: Paginate,
}

/// Предрасчёт выплаты.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PayoutCalculation {
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub commission: String,
    #[serde(default)]
    pub merchant_amount: String,
    #[serde(default)]
    pub to_amount: String,
}

/// Котировка валюты к USDT.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ExchangeRate {
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub to: String,
    #[serde(default)]
    pub course: String,
}

/// Сеть в публичном каталоге `GET /v1/currencies`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CurrencyNetwork {
    #[serde(default)]
    pub network: String,
    /// `native` (монета сети) или `token`.
    #[serde(default)]
    pub kind: String,
    /// Адрес контракта токена (для `token`).
    #[serde(default)]
    pub contract: String,
    #[serde(default)]
    pub min_confirmations: u32,
    /// Доступен ли приём (синоним `deposit_available`).
    #[serde(default)]
    pub available: bool,
    #[serde(default)]
    pub deposit_available: bool,
    #[serde(default)]
    pub payout_available: bool,
}

/// Актив в публичном каталоге `GET /v1/currencies`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Currency {
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub decimals: u32,
    #[serde(default)]
    pub networks: Vec<CurrencyNetwork>,
}

/// Результат регистрации endpoint вебхуков.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct WebhookRegistration {
    #[serde(default)]
    pub endpoint_id: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub secret: String,
}

/// Запись журнала доставок.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Delivery {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub event_type: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub attempts: i64,
    #[serde(default)]
    pub last_error: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

/// Принимаемая пара валюта+сеть.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AcceptedMethod {
    pub currency: String,
    pub network: String,
}

/// Правило автовывода.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AutoWithdrawRule {
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub min_minor: String,
}
