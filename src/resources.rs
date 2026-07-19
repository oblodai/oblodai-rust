//! Ресурсы (группы методов). Каждый ресурс — тонкая обёртка над [`Client`].
//!
//! Тела запросов принимаются как [`serde_json::Value`] — удобно строить макросом `json!`.

use crate::client::Client;
use crate::error::Result;
use crate::models::*;
use serde_json::{json, Value};

/// Извлекает (и УДАЛЯЕТ из тела) явный ключ идемпотентности `idempotency_key`.
///
/// Создающие методы шлют заголовок `Idempotency-Key`: либо переданный вами через поле
/// `idempotency_key` в параметрах, либо (по умолчанию) сгенерированный UUID v4 — один раз до
/// повторов. В тело запроса это поле НЕ уходит; `order_id` при этом передаётся как есть и больше
/// НЕ подставляется автоматически.
fn take_idempotency_key(params: Value) -> (Value, Option<String>) {
    match params {
        Value::Object(mut m) => {
            let key = m
                .remove("idempotency_key")
                .and_then(|v| v.as_str().map(|s| s.trim().to_string()))
                .filter(|s| !s.is_empty());
            (Value::Object(m), key)
        }
        other => (other, None),
    }
}

/// Собирает тело пачки: `{"<field>": [...items], "on_error": "continue"|"stop"}`.
fn batch_body(field: &str, items: Vec<Value>, on_error: Option<&str>) -> Value {
    let mut body = json!({ field: items });
    if let Some(mode) = on_error {
        body["on_error"] = json!(mode);
    }
    body
}

/// Тело пагинации `{"limit","offset"}` (поля добавляются только если заданы).
fn page(limit: Option<i64>, offset: Option<i64>) -> Value {
    let mut m = serde_json::Map::new();
    if let Some(l) = limit {
        m.insert("limit".into(), json!(l));
    }
    if let Some(o) = offset {
        m.insert("offset".into(), json!(o));
    }
    Value::Object(m)
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
    ///
    /// Идемпотентен: шлётся заголовок `Idempotency-Key` (свой ключ — поле `idempotency_key`
    /// в `params`, оно не попадает в тело). `order_id` уходит как есть и не подставляется.
    pub fn create(&self, params: Value) -> Result<Payment> {
        let (body, key) = take_idempotency_key(params);
        self.client.request_idempotent("/v1/payment", &body, key)
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
    ///
    /// Идемпотентен: шлётся заголовок `Idempotency-Key` (свой ключ — поле `idempotency_key`).
    pub fn refund(&self, params: Value) -> Result<Value> {
        let (body, key) = take_idempotency_key(params);
        self.client
            .request_idempotent("/v1/payment/refund", &body, key)
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

    /// Пачка платежей (до 5000) одним запросом. `POST /v1/payment/batch`
    ///
    /// Каждый элемент — обычное тело `create`; `order_id` обязателен на каждом элементе.
    /// `on_error`: `"continue"` (по умолчанию) или `"stop"`. Обработка фоновая — результаты
    /// забираются через [`Batches::info`] по `batch_id`. Идемпотентна (`Idempotency-Key`
    /// генерируется на весь вызов).
    pub fn create_batch(
        &self,
        payments: Vec<Value>,
        on_error: Option<&str>,
    ) -> Result<BatchSubmission> {
        self.client.request_idempotent(
            "/v1/payment/batch",
            &batch_body("payments", payments, on_error),
            None,
        )
    }

    /// Пачка возвратов (до 5000). `POST /v1/refund/batch`
    ///
    /// На каждом элементе обязательны `reference` и `uuid`/`order_id` инвойса.
    /// Идемпотентна (`Idempotency-Key`). Результаты — через [`Batches::info`].
    pub fn refund_batch(
        &self,
        refunds: Vec<Value>,
        on_error: Option<&str>,
    ) -> Result<BatchSubmission> {
        self.client.request_idempotent(
            "/v1/refund/batch",
            &batch_body("refunds", refunds, on_error),
            None,
        )
    }

    /// Отправить счёт на e-mail (письмо с кнопкой «Оплатить»). `POST /v1/payment/send-email`
    ///
    /// `email` `None` — письмо уйдёт на `payer_email` платежа. Лимит: 10 писем/час на адрес
    /// получателя (`email.rate_limited`).
    pub fn send_email(
        &self,
        uuid: Option<&str>,
        order_id: Option<&str>,
        email: Option<&str>,
    ) -> Result<Value> {
        let mut body = lookup(uuid, order_id);
        if let Some(e) = email {
            body["email"] = json!(e);
        }
        self.client.request("/v1/payment/send-email", &body)
    }

    /// Решить судьбу недоплаченного платежа (`wrong_amount`). `POST /v1/payment/resolve`
    ///
    /// [`ResolveAction::Accept`] — оставить частичную оплату (глушит авто-возврат);
    /// [`ResolveAction::Refund`] — вернуть плательщику. В `params` — `uuid` или `order_id`
    /// платежа; для refund опционально `address`/`network`/`reference`. Идемпотентен:
    /// шлётся заголовок `Idempotency-Key` (свой ключ — поле `idempotency_key`).
    pub fn resolve(&self, action: ResolveAction, params: Value) -> Result<Resolution> {
        let (mut body, key) = take_idempotency_key(params);
        if let Value::Object(m) = &mut body {
            m.insert("action".into(), json!(action.as_str()));
        }
        self.client
            .request_idempotent("/v1/payment/resolve", &body, key)
    }
}

// ─────────────────────────────── Payouts ───────────────────────────────

/// Методы выплат и возвратов.
pub struct Payouts<'a> {
    pub(crate) client: &'a Client,
}

impl Payouts<'_> {
    /// Создать выплату. `POST /v1/payout`
    ///
    /// Идемпотентен: шлётся заголовок `Idempotency-Key` (свой ключ — поле `idempotency_key`
    /// в `params`). `order_id` обязателен всегда (`payout.order_id_required`) и уходит как есть.
    pub fn create(&self, params: Value) -> Result<Payout> {
        let (body, key) = take_idempotency_key(params);
        self.client.request_idempotent("/v1/payout", &body, key)
    }
    /// Массовая выплата (до 100). `POST /v1/payout/mass`
    ///
    /// Идемпотентна: заголовок `Idempotency-Key` генерируется на весь вызов.
    pub fn create_mass(
        &self,
        payouts: Vec<Value>,
        source: Option<&str>,
    ) -> Result<MassPayoutResult> {
        let mut body = json!({ "payouts": payouts });
        if let Some(s) = source {
            body["source"] = json!(s);
        }
        self.client
            .request_idempotent("/v1/payout/mass", &body, None)
    }
    /// Пачка выплат (до 5000) одним запросом. `POST /v1/payout/batch`
    ///
    /// Каждый элемент — обычное тело `create`; `order_id` обязателен на каждом элементе.
    /// `on_error`: `"continue"` (по умолчанию) или `"stop"`. Результаты — через
    /// [`Batches::info`]. Идемпотентна (`Idempotency-Key`).
    pub fn create_batch(
        &self,
        payouts: Vec<Value>,
        on_error: Option<&str>,
    ) -> Result<BatchSubmission> {
        self.client.request_idempotent(
            "/v1/payout/batch",
            &batch_body("payouts", payouts, on_error),
            None,
        )
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
    ///
    /// Идемпотентен: шлётся заголовок `Idempotency-Key` (свой ключ — поле `idempotency_key`).
    pub fn refund(&self, params: Value) -> Result<Value> {
        let (body, key) = take_idempotency_key(params);
        self.client
            .request_idempotent("/v1/payment/refund", &body, key)
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
    ///
    /// Идемпотентен: шлётся заголовок `Idempotency-Key` (свой ключ — поле `idempotency_key`
    /// в `params`). `order_id` уходит как есть и не подставляется.
    pub fn transfer_to_personal(&self, params: Value) -> Result<Value> {
        let (body, key) = take_idempotency_key(params);
        self.client
            .request_idempotent("/v1/transfer/to-personal", &body, key)
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

// ─────────────────────────────── Batches ───────────────────────────────

/// Состояние массовых операций (пачек). Постановка — в [`Payments::create_batch`],
/// [`Payments::refund_batch`], [`Payouts::create_batch`].
pub struct Batches<'a> {
    pub(crate) client: &'a Client,
}

impl Batches<'_> {
    /// Состояние пачки и результаты элементов. `POST /v1/batch/info`
    ///
    /// `limit` вне (0, 500] → 100 (дефолт бэкенда). Элементы index-aligned с запросом постановки.
    pub fn info(
        &self,
        batch_id: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<BatchInfo> {
        let mut body = page(limit, offset);
        body["batch_id"] = json!(batch_id);
        self.client.request("/v1/batch/info", &body)
    }
}

// ─────────────────────────────── Payment links ───────────────────────────────

/// Платёжные ссылки: переиспользуемая ссылка, по которой платят много людей;
/// каждый платёж — отдельный инвойс со своим адресом.
pub struct PaymentLinks<'a> {
    pub(crate) client: &'a Client,
}

impl PaymentLinks<'_> {
    /// Создать платёжную ссылку. `POST /v1/payment/link`
    ///
    /// Поля: `title`, `description`, `amount_mode` (`fixed|open|range`), `currency`,
    /// `amount_fixed`/`amount_min`/`amount_max`, `pinned_currency`, `pinned_network`,
    /// `expires_in` (секунды; 0 — бессрочно).
    pub fn create(&self, params: Value) -> Result<PaymentLink> {
        self.client.request("/v1/payment/link", &params)
    }
    /// Список ссылок. `POST /v1/payment/link/list`
    pub fn list(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<PaymentLink>> {
        #[derive(serde::Deserialize)]
        struct Wrap {
            #[serde(default)]
            items: Vec<PaymentLink>,
        }
        let w: Wrap = self
            .client
            .request("/v1/payment/link/list", &page(limit, offset))?;
        Ok(w.items)
    }
    /// Детали ссылки (с платежами по ней). `POST /v1/payment/link/info`
    pub fn info(&self, link_id: &str) -> Result<PaymentLink> {
        self.client
            .request("/v1/payment/link/info", &json!({ "link_id": link_id }))
    }
    /// Включить/выключить ссылку. `POST /v1/payment/link/toggle`
    pub fn toggle(&self, link_id: &str, active: bool) -> Result<PaymentLink> {
        self.client.request(
            "/v1/payment/link/toggle",
            &json!({ "link_id": link_id, "active": active }),
        )
    }
    /// Публичные детали ссылки (без подписи). `GET /v1/link/{id}`
    pub fn public_get(&self, link_id: &str) -> Result<Value> {
        self.client
            .request_public_get(&format!("/v1/link/{link_id}"))
    }
    /// Публичный checkout по ссылке (без подписи): создаёт инвойс. `POST /v1/link/{id}/checkout`
    ///
    /// Поля: `amount`, `currency`, `network`, `payer_email` (pinned-валюта/сеть ссылки побеждают).
    /// Лимит: 30 инвойсов/мин на ссылку (`paylink.rate_limited`).
    pub fn checkout(&self, link_id: &str, params: Value) -> Result<Payment> {
        self.client
            .request_public(&format!("/v1/link/{link_id}/checkout"), &params)
    }
}

// ─────────────────────────────── Splits ───────────────────────────────

/// Сплит-платежи: доля каждого входящего платежа автоматически уходит партнёру.
pub struct Splits<'a> {
    pub(crate) client: &'a Client,
}

impl Splits<'_> {
    /// Создать правило сплита. `POST /v1/split/rule`
    ///
    /// Получатель — либо `{"address","network"}` (внешний, необратимо), либо `{"merchant_id"}`
    /// (на платформе, обратимо). `percent` — доля в процентах (шаг 0.01, суммарно ≤ 100).
    pub fn create_rule(&self, params: Value) -> Result<SplitRule> {
        self.client.request("/v1/split/rule", &params)
    }
    /// Правило на внешний адрес (необратимо). Обёртка над [`create_rule`](Self::create_rule).
    pub fn split_to_address(
        &self,
        address: &str,
        network: &str,
        percent: f64,
        note: Option<&str>,
    ) -> Result<SplitRule> {
        let mut params = serde_json::json!({
            "address": address, "network": network, "percent": percent,
        });
        if let Some(n) = note {
            params["note"] = Value::String(n.to_owned());
        }
        self.create_rule(params)
    }
    /// Правило на мерчанта платформы (обратимо). Обёртка над [`create_rule`](Self::create_rule).
    pub fn split_to_merchant(
        &self,
        merchant_id: &str,
        percent: f64,
        note: Option<&str>,
    ) -> Result<SplitRule> {
        let mut params = serde_json::json!({
            "merchant_id": merchant_id, "percent": percent,
        });
        if let Some(n) = note {
            params["note"] = Value::String(n.to_owned());
        }
        self.create_rule(params)
    }
    /// Список правил. `POST /v1/split/rule/list`
    pub fn list_rules(&self) -> Result<Vec<SplitRule>> {
        #[derive(serde::Deserialize)]
        struct Wrap {
            #[serde(default)]
            items: Vec<SplitRule>,
        }
        let w: Wrap = self.client.request("/v1/split/rule/list", &json!({}))?;
        Ok(w.items)
    }
    /// Удалить правило. `POST /v1/split/rule/delete`
    pub fn delete_rule(&self, rule_id: &str) -> Result<Value> {
        self.client
            .request("/v1/split/rule/delete", &json!({ "rule_id": rule_id }))
    }
    /// Настройки сплитов — чтение. `POST /v1/split/config/get`
    pub fn get_config(&self) -> Result<SplitConfig> {
        self.client.request("/v1/split/config/get", &json!({}))
    }
    /// Настройки сплитов — запись окна удержания (часы). `POST /v1/split/config/set`
    pub fn set_config(&self, refund_hold_hours: i64) -> Result<Value> {
        self.client.request(
            "/v1/split/config/set",
            &json!({ "refund_hold_hours": refund_hold_hours }),
        )
    }
}

// ─────────────────────────────── Payout links ───────────────────────────────

/// Payout-ссылки («крипто-чеки»): резервируете средства, получатель сам вводит адрес на
/// публичной странице claim. Management-методы требуют payout-ключ; claim-методы — публичные.
///
/// Эти эндпоинты НЕ принимают заголовок `Idempotency-Key` — SDK его не шлёт. Дедупликация
/// `create` — через ваш per-link `reference` (уникален per-merchant).
pub struct PayoutLinks<'a> {
    pub(crate) client: &'a Client,
}

impl PayoutLinks<'_> {
    /// Создать payout-ссылку. `POST /v1/payout/link`
    ///
    /// Поля: `currency`, `network`, `amount` (строка) — обязательны; `reference` (ключ
    /// дедупликации), `title`, `note`, `email` (отправить claim-письмо), `expires_in_hours`.
    ///
    /// **Рекомендуется задавать `expires_in_hours` явно** (1–720): при отсутствии/0 бэкенд
    /// клампит срок к **1 часу**, а не к максимуму. Сохраните `claim_token`/`claim_url` из
    /// ответа сразу — повторно они не выдаются.
    ///
    /// Заголовок `Idempotency-Key` НЕ шлётся (эндпоинт его не поддерживает) — дедуплицируйте
    /// через `reference`.
    pub fn create(&self, params: Value) -> Result<PayoutLink> {
        self.client.request("/v1/payout/link", &params)
    }
    /// Пачка payout-ссылок (до 500). `POST /v1/payout/link/batch`
    ///
    /// Каждый элемент — обычное тело [`PayoutLinks::create`]; плохой элемент фейлит только себя.
    /// Ответ index-aligned; все созданные ссылки получают общий `batch_id`. Про
    /// `expires_in_hours` и отсутствие `Idempotency-Key` — см. [`PayoutLinks::create`].
    pub fn create_batch(&self, links: Vec<Value>) -> Result<PayoutLinkBatch> {
        self.client
            .request("/v1/payout/link/batch", &json!({ "links": links }))
    }
    /// Список ссылок (без `claim_token`). `POST /v1/payout/link/list`
    ///
    /// `limit` вне (0, 200] → 50 (дефолт бэкенда). Сортировка: новые первыми.
    pub fn list(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<PayoutLink>> {
        #[derive(serde::Deserialize)]
        struct Wrap {
            #[serde(default)]
            links: Vec<PayoutLink>,
        }
        let w: Wrap = self
            .client
            .request("/v1/payout/link/list", &page(limit, offset))?;
        Ok(w.links)
    }
    /// Детали ссылки (после claim — с `payout_id`/`claim_address`). `POST /v1/payout/link/info`
    pub fn info(&self, link_id: &str) -> Result<PayoutLink> {
        self.client
            .request("/v1/payout/link/info", &json!({ "link_id": link_id }))
    }
    /// Отменить непорученную (`funded`) ссылку — резерв вернётся. `POST /v1/payout/link/cancel`
    ///
    /// Уже полученную/истёкшую отменить нельзя (`payoutlink.not_funded`).
    pub fn cancel(&self, link_id: &str) -> Result<PayoutLink> {
        self.client
            .request("/v1/payout/link/cancel", &json!({ "link_id": link_id }))
    }
    /// ПУБЛИЧНЫЕ детали claim-страницы (без подписи, по секретному токену). `GET /v1/claim/{token}`
    pub fn claim_info(&self, token: &str) -> Result<ClaimInfo> {
        self.client
            .request_public_get(&format!("/v1/claim/{token}"))
    }
    /// ПУБЛИЧНЫЙ claim: получатель указывает адрес — порождается выплата. `POST /v1/claim/{token}`
    ///
    /// Запрос не подписывается (capability — сам токен). `memo` — dest tag/comment для сетей,
    /// где он нужен (TON и т.п.). Повторный claim с тем же адресом идемпотентен; с другим —
    /// `payoutlink.claim_in_progress`.
    pub fn claim(&self, token: &str, address: &str, memo: Option<&str>) -> Result<ClaimResult> {
        let mut body = json!({ "address": address });
        if let Some(m) = memo {
            body["memo"] = json!(m);
        }
        self.client
            .request_public(&format!("/v1/claim/{token}"), &body)
    }
}

// ─────────────────────────────── Sandbox ───────────────────────────────

/// Песочница разработчика — вспомогательные методы, доступные ТОЛЬКО тестовому ключу
/// (`public_id` с префиксом `test_`, секрет — `oblodai_test_`). Они заменяют «клиент заплатил
/// он-чейн»: боевого аналога у них нет, боевой ключ получит 403 `sandbox.live_key`.
///
/// Все бизнес-методы SDK с тестовым ключом работают БЕЗ изменений — интеграционный код одинаков
/// для теста и прода, меняется только ключ. Вызовы песочницы держите строго в тестовом коде.
pub struct Sandbox<'a> {
    pub(crate) client: &'a Client,
}

impl Sandbox<'_> {
    /// Симулировать он-чейн депозит в инвойс. `POST /v1/sandbox/deposit`
    ///
    /// Поля `params` (все опциональны):
    /// - `amount` (строка) — не задано → оплатить ровно причитающееся; иное значение —
    ///   недоплата/переплата;
    /// - `confirmations` (число) — не задано/0 → сразу полностью подтверждён; малое значение →
    ///   депозит «ещё висит»; повтор с тем же `txid` и бОльшим числом «углубляет» подтверждения;
    /// - `txid` (строка) — не задано → новый; повторяйте для идемпотентности/углубления.
    pub fn simulate_deposit(&self, invoice_id: &str, params: Value) -> Result<SandboxDeposit> {
        let mut body = if params.is_object() {
            params
        } else {
            json!({})
        };
        body["invoice_id"] = json!(invoice_id);
        self.client.request("/v1/sandbox/deposit", &body)
    }

    /// Начислить тестовый баланс «из воздуха». `POST /v1/sandbox/faucet`
    ///
    /// `amount` — строка (потолок 1000000 за вызов). `idempotency_key` здесь — поле ТЕЛА запроса
    /// (не заголовок): повтор с тем же ключом не начислит дважды.
    pub fn faucet(
        &self,
        asset: &str,
        amount: &str,
        idempotency_key: Option<&str>,
    ) -> Result<SandboxFaucet> {
        let mut body = json!({ "asset": asset, "amount": amount });
        if let Some(k) = idempotency_key {
            body["idempotency_key"] = json!(k);
        }
        self.client.request("/v1/sandbox/faucet", &body)
    }

    /// Сбросить песочницу: отменить открытые инвойсы, обнулить балансы. `POST /v1/sandbox/reset`
    pub fn reset(&self) -> Result<SandboxReset> {
        self.client.request("/v1/sandbox/reset", &json!({}))
    }

    /// Недавние доставки вебхуков (до 50, новые первыми). `GET /v1/sandbox/webhooks`
    ///
    /// Подписанный GET с ПУСТЫМ телом: подпись считается от `{ts}\nGET\n/v1/sandbox/webhooks\n`.
    pub fn list_webhooks(&self) -> Result<Vec<SandboxWebhookDelivery>> {
        #[derive(serde::Deserialize)]
        struct Wrap {
            #[serde(default)]
            deliveries: Vec<SandboxWebhookDelivery>,
        }
        let w: Wrap = self.client.request_get("/v1/sandbox/webhooks")?;
        Ok(w.deliveries)
    }

    /// Поставить одну доставку на повторную отправку. `POST /v1/sandbox/webhooks/replay`
    pub fn replay_webhook(&self, delivery_id: &str) -> Result<SandboxReplay> {
        self.client.request(
            "/v1/sandbox/webhooks/replay",
            &json!({ "delivery_id": delivery_id }),
        )
    }
}
