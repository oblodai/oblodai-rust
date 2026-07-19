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

// ─────────────────────────── Массовые операции (v1.1.0) ───────────────────────────

/// Результат постановки пачки (`POST /v1/payment/batch`, `/v1/refund/batch`, `/v1/payout/batch`).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct BatchSubmission {
    #[serde(default)]
    pub batch_id: String,
    /// Вид пачки: `payment` / `refund` / `payout`.
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub count: i64,
    /// `pending` → `processing` → `completed`.
    #[serde(default)]
    pub status: String,
}

/// Элемент пачки в ответе `POST /v1/batch/info`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct BatchItem {
    #[serde(default)]
    pub idx: i64,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub order_id: String,
    /// Байт-в-байт сохранённый `result` единичного эндпоинта (если элемент успешен).
    #[serde(default)]
    pub result: serde_json::Value,
    /// Ошибка элемента (если он неуспешен).
    #[serde(default)]
    pub error: serde_json::Value,
}

/// Состояние пачки и её элементы. `POST /v1/batch/info`
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct BatchInfo {
    #[serde(default)]
    pub batch_id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub on_error: String,
    #[serde(default)]
    pub total: i64,
    #[serde(default)]
    pub succeeded: i64,
    #[serde(default)]
    pub failed: i64,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub items: Vec<BatchItem>,
}

// ─────────────────────────── Платёжные ссылки (v1.1.0) ───────────────────────────

/// Платёж, привязанный к платёжной ссылке (в ответе `info`).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PaymentLinkPayment {
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub order_id: String,
}

/// Платёжная ссылка (create/list/info/toggle).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PaymentLink {
    #[serde(default)]
    pub link_id: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    /// `fixed` / `open` / `range`.
    #[serde(default)]
    pub amount_mode: String,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub amount_fixed: String,
    #[serde(default)]
    pub amount_min: String,
    #[serde(default)]
    pub amount_max: String,
    #[serde(default)]
    pub pinned_currency: String,
    #[serde(default)]
    pub pinned_network: String,
    #[serde(default)]
    pub expires_at: String,
    /// Платежи по ссылке — только в ответе `info`.
    #[serde(default)]
    pub payments: Vec<PaymentLinkPayment>,
}

// ─────────────────────────── Сплит-платежи (v1.1.0) ───────────────────────────

/// Правило сплита (create/list). Получатель — либо внешний `address`+`network`
/// (необратимо), либо `merchant_id` на платформе (обратимо при возврате).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SplitRule {
    #[serde(default)]
    pub rule_id: String,
    /// Доля в процентах (шаг 0.01%).
    #[serde(default)]
    pub percent: f64,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub merchant_id: String,
    /// `true` — доля отзывается при возврате (получатель-мерчант на платформе).
    #[serde(default)]
    pub reversible: bool,
}

/// Настройки сплитов. `POST /v1/split/config/get|set`
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SplitConfig {
    /// Окно удержания (в часах) перед исходящей маршрутизацией долей после settle.
    #[serde(default)]
    pub refund_hold_hours: i64,
}

// ─────────────────────────── Payout-ссылки (v1.1.0) ───────────────────────────

/// Статус payout-ссылки («крипто-чека»).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PayoutLinkStatus {
    /// Создана, резерв удержан, ждёт claim.
    #[default]
    Funded,
    /// Claim в процессе: адрес зафиксирован, порождается выплата (транзиентный).
    Claiming,
    /// Выплата порождена (`payout_id` установлен) — терминальный.
    Claimed,
    /// Дедлайн прошёл без claim, резерв возвращён — терминальный.
    Expired,
    /// Отменена мерчантом до claim, резерв возвращён — терминальный.
    Cancelled,
    /// Неизвестный статус (совместимость с будущими версиями API).
    #[serde(other)]
    Unknown,
}

impl PayoutLinkStatus {
    /// Строковое представление, как в API.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Funded => "funded",
            Self::Claiming => "claiming",
            Self::Claimed => "claimed",
            Self::Expired => "expired",
            Self::Cancelled => "cancelled",
            Self::Unknown => "unknown",
        }
    }
}

/// Payout-ссылка («крипто-чек»). `claim_token`/`claim_url` приходят ТОЛЬКО в ответе create —
/// сохраните их сразу: в list/info их больше не будет.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PayoutLink {
    #[serde(default)]
    pub link_id: String,
    #[serde(default)]
    pub status: PayoutLinkStatus,
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub expires_at: String,
    #[serde(default)]
    pub created_at: String,
    /// Ваш ключ дедупликации (уникален per-merchant).
    #[serde(default)]
    pub reference: String,
    #[serde(default)]
    pub email: String,
    /// Секретный claim-токен — только в ответе create.
    #[serde(default)]
    pub claim_token: String,
    /// Публичная ссылка на страницу claim — только в ответе create.
    #[serde(default)]
    pub claim_url: String,
    /// Порождённая выплата (после claim).
    #[serde(default)]
    pub payout_id: String,
    /// Адрес получателя (после claim).
    #[serde(default)]
    pub claim_address: String,
    /// Общий id пачки (для ссылок из `create_batch`).
    #[serde(default)]
    pub batch_id: String,
}

/// Элемент результата пачки payout-ссылок (index-aligned с запросом).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PayoutLinkBatchItem {
    #[serde(default)]
    pub ok: bool,
    /// Созданная ссылка (при `ok == true`), с `claim_token`/`claim_url` и `batch_id`.
    #[serde(default)]
    pub link: Option<PayoutLink>,
    /// Код ошибки (при `ok == false`), напр. `payoutlink.insufficient_funds`.
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub message: String,
}

/// Результат `POST /v1/payout/link/batch`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PayoutLinkBatch {
    #[serde(default)]
    pub created: i64,
    #[serde(default)]
    pub total: i64,
    #[serde(default)]
    pub results: Vec<PayoutLinkBatchItem>,
}

/// Публичные детали claim-страницы. `GET /v1/claim/{token}` (без подписи).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ClaimInfo {
    #[serde(default)]
    pub status: PayoutLinkStatus,
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub expires_at: String,
    /// `true`, если ссылку ещё можно получить (`funded` и срок не истёк).
    #[serde(default)]
    pub claimable: bool,
}

/// Результат публичного claim. `POST /v1/claim/{token}` (без подписи).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ClaimResult {
    #[serde(default)]
    pub status: PayoutLinkStatus,
    #[serde(default)]
    pub payout_id: String,
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub currency: String,
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub address: String,
}

// ─────────────────────────── Песочница (v1.2.0) ───────────────────────────

/// Результат симуляции депозита. `POST /v1/sandbox/deposit` (только тестовый ключ).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SandboxDeposit {
    #[serde(default)]
    pub invoice_id: String,
    /// «Ончейн»-txid симуляции. Повторный вызов с тем же `txid` идемпотентен;
    /// с бОльшим `confirmations` — «углубляет» подтверждения того же депозита.
    #[serde(default)]
    pub txid: String,
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub confirmations: i64,
}

/// Результат faucet-начисления. `POST /v1/sandbox/faucet` (только тестовый ключ).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SandboxFaucet {
    #[serde(default)]
    pub asset: String,
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub journal_id: String,
}

/// Результат сброса песочницы. `POST /v1/sandbox/reset` (только тестовый ключ).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SandboxReset {
    #[serde(default)]
    pub invoices_cancelled: i64,
    #[serde(default)]
    pub balances_zeroed: i64,
}

/// Запись журнала доставок песочницы (с сырым `payload`). `GET /v1/sandbox/webhooks`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SandboxWebhookDelivery {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub event_type: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub attempts: i64,
    #[serde(default)]
    pub last_error: String,
    /// Сырое JSON-тело вебхука как его отправлял шлюз.
    #[serde(default)]
    pub payload: serde_json::Value,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

/// Результат постановки доставки на повтор. `POST /v1/sandbox/webhooks/replay`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SandboxReplay {
    #[serde(default)]
    pub delivery_id: String,
    #[serde(default)]
    pub requeued: bool,
}

// ─────────────────────────── Resolve недоплаты (v1.1.0) ───────────────────────────

/// Действие над недоплаченным платежом для [`crate::resources::Payments::resolve`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveAction {
    /// Оставить частичную оплату себе (глушит авто-возврат).
    Accept,
    /// Вернуть средства плательщику.
    Refund,
}

impl ResolveAction {
    /// Строковое представление, как в API (`accept` / `refund`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Refund => "refund",
        }
    }
}

/// Результат `POST /v1/payment/resolve`. При `resolution == "accepted"` заполнены
/// `amount_kept`/`currency`; при `"refunded"` — поля рефанд-выплаты (`uuid`, `amount`,
/// `address`, `status`, `is_final`).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Resolution {
    #[serde(default)]
    pub payment_uuid: String,
    #[serde(default)]
    pub order_id: String,
    /// `accepted` или `refunded`.
    #[serde(default)]
    pub resolution: String,
    #[serde(default)]
    pub amount_kept: String,
    #[serde(default)]
    pub currency: String,
    /// UUID рефанд-выплаты (только при `refunded`).
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub amount: String,
    #[serde(default)]
    pub address: String,
    /// Статус рефанд-выплаты: `check`/`process`/`paid`/`fail`/`cancel`.
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub is_final: bool,
}
