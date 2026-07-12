//! Ресурсы (группы методов). Каждый ресурс — тонкая обёртка над [`Client`].
//!
//! Тела запросов принимаются как [`serde_json::Value`] — удобно строить макросом `json!`.

use crate::client::Client;
use crate::error::Result;
use crate::models::*;
use serde_json::{json, Value};

/// Гарантирует наличие непустого `order_id` в теле запроса.
///
/// Бэкенд дедуплицирует платежи/переводы по `order_id`. Клиент повторяет неидемпотентные POST'ы,
/// переподписывая каждую попытку, поэтому без стабильного `order_id` таймаут+повтор может создать
/// дубль. Если поле не задано (или пустое), подставляем стабильный ключ `idem-<hex>` ОДИН РАЗ —
/// до отправки — так все повторы внутри одного вызова используют один и тот же `order_id`.
fn ensure_order_id(params: Value) -> Value {
    match params {
        Value::Object(mut m) => {
            // Считаем `order_id` заданным ТОЛЬКО если это непустая строка после trim.
            // Отсутствие/null/""/пробелы, а также нестроковое значение (число/bool) → подставляем ключ.
            let has = matches!(
                m.get("order_id").and_then(|v| v.as_str()),
                Some(s) if !s.trim().is_empty()
            );
            if !has {
                m.insert("order_id".into(), json!(format!("idem-{}", crate::random::hex16())));
            }
            Value::Object(m)
        }
        other => other,
    }
}

fn lookup(uuid: Option<&str>, order_id: Option<&str>) -> Value {
    let mut m = serde_json::Map::new();
    if let Some(u) = uuid {
        m.insert("uuid".into(), json!(u));
    }
    if let Some(o) = order_id {
        m.insert("order_id".into(), json!(o));
    }
    Value::Object(m)
}

// ─────────────────────────────── Payments ───────────────────────────────

/// Методы приёма платежей.
pub struct Payments<'a> {
    pub(crate) client: &'a Client,
}

impl Payments<'_> {
    /// Создать платёжный счёт (инвойс). `POST /v1/payment`
    pub fn create(&self, params: Value) -> Result<Payment> {
        self.client.request("/v1/payment", &ensure_order_id(params))
    }
    /// Информация о счёте. `POST /v1/payment/info`
    pub fn info(&self, uuid: Option<&str>, order_id: Option<&str>) -> Result<Payment> {
        self.client
            .request("/v1/payment/info", &lookup(uuid, order_id))
    }
    /// Список платежей. `POST /v1/payment/history`
    pub fn history(&self, params: Value) -> Result<PaymentList> {
        self.client.request("/v1/payment/history", &params)
    }
    /// Доступные методы приёма. `POST /v1/payment/services`
    pub fn services(&self) -> Result<Vec<ServiceMethod>> {
        self.client.request("/v1/payment/services", &json!({}))
    }
    /// QR-код депозит-адреса счёта. `POST /v1/payment/qr`
    pub fn qr(&self, uuid: Option<&str>, order_id: Option<&str>) -> Result<Value> {
        self.client
            .request("/v1/payment/qr", &lookup(uuid, order_id))
    }
    /// Переотправить вебхук платежа. `POST /v1/payment/resend`
    pub fn resend(&self, uuid: Option<&str>, order_id: Option<&str>) -> Result<Value> {
        self.client
            .request("/v1/payment/resend", &lookup(uuid, order_id))
    }
    /// Возврат средств платежа. `POST /v1/payment/refund`
    pub fn refund(&self, params: Value) -> Result<Value> {
        self.client.request("/v1/payment/refund", &params)
    }
    /// Набор принимаемых валют. `POST /v1/payment/accepted/list`
    pub fn list_accepted(&self) -> Result<Value> {
        self.client.request("/v1/payment/accepted/list", &json!({}))
    }
    /// Заменить набор принимаемых валют. `POST /v1/payment/accepted/set`
    pub fn set_accepted(&self, accepted: Vec<AcceptedMethod>) -> Result<Value> {
        self.client
            .request("/v1/payment/accepted/set", &json!({ "accepted": accepted }))
    }
    /// Прочитать допуск недоплаты. `POST /v1/payment/accuracy/get`
    pub fn get_accuracy(&self) -> Result<Value> {
        self.client.request("/v1/payment/accuracy/get", &json!({}))
    }
    /// Задать допуск недоплаты. `POST /v1/payment/accuracy/set`
    pub fn set_accuracy(&self, params: Value) -> Result<Value> {
        self.client.request("/v1/payment/accuracy/set", &params)
    }
    /// Прочитать настройки автовозврата. `POST /v1/payment/autorefund/get`
    pub fn get_autorefund(&self) -> Result<Value> {
        self.client
            .request("/v1/payment/autorefund/get", &json!({}))
    }
    /// Задать настройки автовозврата. `POST /v1/payment/autorefund/set`
    pub fn set_autorefund(&self, params: Value) -> Result<Value> {
        self.client.request("/v1/payment/autorefund/set", &params)
    }
    /// Задать скидку/наценку. `POST /v1/payment/discount/set`
    pub fn set_discount(&self, params: Value) -> Result<Value> {
        self.client.request("/v1/payment/discount/set", &params)
    }

    /// Настроенные скидки/наценки. `POST /v1/payment/discount/list`
    pub fn list_discounts(&self) -> Result<Value> {
        self.client.request("/v1/payment/discount/list", &json!({}))
    }
}

// ─────────────────────────────── Payouts ───────────────────────────────

/// Методы выплат и возвратов.
pub struct Payouts<'a> {
    pub(crate) client: &'a Client,
}

impl Payouts<'_> {
    /// Создать выплату. `POST /v1/payout`
    pub fn create(&self, params: Value) -> Result<Payout> {
        self.client.request("/v1/payout", &params)
    }
    /// Массовая выплата (до 100). `POST /v1/payout/mass`
    pub fn create_mass(
        &self,
        payouts: Vec<Value>,
        source: Option<&str>,
    ) -> Result<MassPayoutResult> {
        let mut body = json!({ "payouts": payouts });
        if let Some(s) = source {
            body["source"] = json!(s);
        }
        self.client.request("/v1/payout/mass", &body)
    }
    /// Информация о выплате. `POST /v1/payout/info`
    pub fn info(&self, uuid: Option<&str>, order_id: Option<&str>) -> Result<Payout> {
        self.client
            .request("/v1/payout/info", &lookup(uuid, order_id))
    }
    /// История выплат. `POST /v1/payout/history`
    pub fn history(&self, params: Value) -> Result<PayoutList> {
        self.client.request("/v1/payout/history", &params)
    }
    /// Доступные методы выплат. `POST /v1/payout/services`
    pub fn services(&self) -> Result<Vec<ServiceMethod>> {
        self.client.request("/v1/payout/services", &json!({}))
    }
    /// Предрасчёт выплаты. `POST /v1/payout/calculate`
    pub fn calculate(&self, params: Value) -> Result<PayoutCalculation> {
        self.client.request("/v1/payout/calculate", &params)
    }
    /// Подтвердить выплату в статусе pending. `POST /v1/payout/approve`
    pub fn approve(&self, uuid: &str) -> Result<Value> {
        self.client
            .request("/v1/payout/approve", &json!({ "uuid": uuid }))
    }
    /// Возврат средств платежа. `POST /v1/payment/refund`
    pub fn refund(&self, params: Value) -> Result<Value> {
        self.client.request("/v1/payment/refund", &params)
    }
    /// Кто платит сетевую комиссию выплаты — чтение. `POST /v1/payout/fee-config/get`
    pub fn get_fee_config(&self) -> Result<Value> {
        self.client.request("/v1/payout/fee-config/get", &json!({}))
    }
    /// Кто платит сетевую комиссию выплаты — запись. `POST /v1/payout/fee-config/set`
    pub fn set_fee_config(&self, fee_on_recipient: bool) -> Result<Value> {
        self.client.request(
            "/v1/payout/fee-config/set",
            &json!({ "fee_on_recipient": fee_on_recipient }),
        )
    }
    /// Кто несёт нашу комиссию при возврате — чтение. `POST /v1/payout/refund-fee-config/get`
    pub fn get_refund_fee_config(&self) -> Result<Value> {
        self.client
            .request("/v1/payout/refund-fee-config/get", &json!({}))
    }
    /// Кто несёт нашу комиссию при возврате — запись. `POST /v1/payout/refund-fee-config/set`
    pub fn set_refund_fee_config(&self, fee_on_customer: bool) -> Result<Value> {
        self.client.request(
            "/v1/payout/refund-fee-config/set",
            &json!({ "fee_on_customer": fee_on_customer }),
        )
    }
}

// ─────────────────────────────── Wallets ───────────────────────────────

/// Методы статических кошельков.
pub struct Wallets<'a> {
    pub(crate) client: &'a Client,
}

impl Wallets<'_> {
    /// Создать статический адрес. `POST /v1/wallet`
    pub fn create(&self, params: Value) -> Result<Wallet> {
        self.client.request("/v1/wallet", &params)
    }
    /// Блокировать/разблокировать кошелёк. `POST /v1/wallet/block`
    /// `force_block`: `None` — заблокировать (дефолт API); `Some(false)` — разблокировать.
    pub fn block(&self, address: &str, force_block: Option<bool>) -> Result<Value> {
        let mut body = json!({ "address": address });
        if let Some(f) = force_block {
            body["is_force_block"] = json!(f);
        }
        self.client.request("/v1/wallet/block", &body)
    }
    /// Возврат средств с кошелька на адрес. `POST /v1/wallet/blocked-address-refund`
    pub fn blocked_address_refund(&self, uuid: &str, address: &str) -> Result<Value> {
        self.client.request(
            "/v1/wallet/blocked-address-refund",
            &json!({ "uuid": uuid, "address": address }),
        )
    }
    /// QR-код произвольного адреса. `POST /v1/wallet/qr`
    pub fn qr(&self, address: &str) -> Result<Value> {
        self.client
            .request("/v1/wallet/qr", &json!({ "address": address }))
    }
}

// ─────────────────────────────── Account ───────────────────────────────

/// Баланс, рефералы, перевод на личный кошелёк, VRCS.
pub struct Account<'a> {
    pub(crate) client: &'a Client,
}

impl Account<'_> {
    /// Доступные балансы мерчанта. `POST /v1/balance`
    pub fn balance(&self) -> Result<Balance> {
        #[derive(serde::Deserialize)]
        struct Wrapper {
            balance: Balance,
        }
        let w: Wrapper = self.client.request("/v1/balance", &json!({}))?;
        Ok(w.balance)
    }
    /// Реферальная статистика. `POST /v1/referral/info`
    pub fn referral(&self) -> Result<ReferralInfo> {
        self.client.request("/v1/referral/info", &json!({}))
    }
    /// Перевод на личный кошелёк владельца. `POST /v1/transfer/to-personal`
    pub fn transfer_to_personal(&self, params: Value) -> Result<Value> {
        self.client
            .request("/v1/transfer/to-personal", &ensure_order_id(params))
    }
    /// Включить/выключить VRCS. `enabled` None — чтение. `POST /v1/vrcs`
    pub fn vrcs(&self, enabled: Option<bool>) -> Result<Value> {
        let body = match enabled {
            Some(e) => json!({ "enabled": e }),
            None => json!({}),
        };
        self.client.request("/v1/vrcs", &body)
    }
}

// ─────────────────────────────── Webhooks ───────────────────────────────

/// Управление вебхуками и тестовые события.
pub struct Webhooks<'a> {
    pub(crate) client: &'a Client,
}

impl Webhooks<'_> {
    /// Регистрация URL вебхуков. `POST /v1/webhooks`
    pub fn register(&self, url: &str) -> Result<WebhookRegistration> {
        self.client.request("/v1/webhooks", &json!({ "url": url }))
    }
    /// Журнал доставок. `POST /v1/webhooks/deliveries`
    pub fn deliveries(&self) -> Result<Vec<Delivery>> {
        #[derive(serde::Deserialize)]
        struct Wrapper {
            #[serde(default)]
            deliveries: Vec<Delivery>,
        }
        let w: Wrapper = self.client.request("/v1/webhooks/deliveries", &json!({}))?;
        Ok(w.deliveries)
    }
    /// Пробный вебхук платежа. `POST /v1/test-webhook/payment`
    pub fn test_payment(&self, params: Value) -> Result<Value> {
        self.client.request("/v1/test-webhook/payment", &params)
    }
    /// Пробный вебхук кошелька. `POST /v1/test-webhook/wallet`
    pub fn test_wallet(&self, params: Value) -> Result<Value> {
        self.client.request("/v1/test-webhook/wallet", &params)
    }
    /// Пробный вебхук выплаты. `POST /v1/test-webhook/payout`
    pub fn test_payout(&self, params: Value) -> Result<Value> {
        self.client.request("/v1/test-webhook/payout", &params)
    }
}

// ─────────────────────────────── Settings ───────────────────────────────

/// Автовывод и IP-allowlist.
pub struct Settings<'a> {
    pub(crate) client: &'a Client,
}

impl Settings<'_> {
    /// Правила автовывода. `POST /v1/auto-withdraw/list`
    pub fn list_auto_withdraw(&self) -> Result<Vec<AutoWithdrawRule>> {
        #[derive(serde::Deserialize)]
        struct Wrapper {
            #[serde(default)]
            rules: Vec<AutoWithdrawRule>,
        }
        let w: Wrapper = self.client.request("/v1/auto-withdraw/list", &json!({}))?;
        Ok(w.rules)
    }
    /// Включить автовывод для актива. `POST /v1/auto-withdraw/set`
    pub fn set_auto_withdraw(&self, params: Value) -> Result<Value> {
        self.client.request("/v1/auto-withdraw/set", &params)
    }
    /// Выключить автовывод для актива. `POST /v1/auto-withdraw/delete`
    pub fn delete_auto_withdraw(&self, currency: &str) -> Result<Value> {
        self.client
            .request("/v1/auto-withdraw/delete", &json!({ "currency": currency }))
    }
    /// Список доверенных IP и статус. `POST /v1/api-allowlist/list`
    pub fn list_allowlist(&self) -> Result<Value> {
        self.client.request("/v1/api-allowlist/list", &json!({}))
    }
    /// Добавить IP или CIDR. `POST /v1/api-allowlist/add`
    pub fn add_allowlist(&self, cidr: &str) -> Result<Value> {
        self.client
            .request("/v1/api-allowlist/add", &json!({ "cidr": cidr }))
    }
    /// Удалить IP или CIDR. `POST /v1/api-allowlist/remove`
    pub fn remove_allowlist(&self, cidr: &str) -> Result<Value> {
        self.client
            .request("/v1/api-allowlist/remove", &json!({ "cidr": cidr }))
    }
    /// Включить/выключить контроль. `POST /v1/api-allowlist/enable`
    pub fn enable_allowlist(&self, enabled: bool) -> Result<Value> {
        self.client
            .request("/v1/api-allowlist/enable", &json!({ "enabled": enabled }))
    }
}

// ─────────────────────────────── Rates ───────────────────────────────

/// Публичные курсы валют (подпись не требуется).
pub struct Rates<'a> {
    pub(crate) client: &'a Client,
}

impl Rates<'_> {
    /// Курсы к USDT. `currency_from` None — по всем валютам. `POST /v1/exchange-rate/list`
    pub fn list(&self, currency_from: Option<&str>) -> Result<Vec<ExchangeRate>> {
        let body = match currency_from {
            Some(c) => json!({ "currency_from": c }),
            None => json!({}),
        };
        self.client.request_public("/v1/exchange-rate/list", &body)
    }

    /// Публичный каталог активов и сетей. `GET /v1/currencies` (без подписи).
    pub fn currencies(&self) -> Result<Vec<Currency>> {
        #[derive(serde::Deserialize)]
        struct Wrap {
            #[serde(default)]
            currencies: Vec<Currency>,
        }
        let w: Wrap = self.client.request_public_get("/v1/currencies")?;
        Ok(w.currencies)
    }
}
