# Oblodai Rust SDK

Официальный Rust SDK для платёжного шлюза **Oblodai**: приём платежей, выплаты, статические кошельки,
вебхуки. Автоподпись запросов, разбор ответов в типизированные структуры, обработка ошибок и
автоматические повторы.

> **Базовый URL.** По умолчанию — `https://api.oblodai.com`. При необходимости переопределите через `Config::base_url(...)` и свои ключи.

## Установка

```toml
[dependencies]
oblodai = "1"
serde_json = "1"
```

Требуется Rust 1.75+.

## Учётные данные

Храните ключи в переменных окружения (см. `.env.example`):

```bash
export OBLODAI_PUBLIC_ID=oblodai_...
export OBLODAI_SECRET=oblodai_live_...
# необязательно: export OBLODAI_BASE_URL=https://api.oblodai.com
```

```rust
// читает OBLODAI_PUBLIC_ID / OBLODAI_SECRET / OBLODAI_BASE_URL
let client = oblodai::Client::from_env()?;
```

## Быстрый старт

```rust
use oblodai::{Client, Config};
use serde_json::json;

fn main() -> oblodai::Result<()> {
    // либо явно (эквивалент Client::from_env выше):
    let client = Client::new(
        Config::new("oblodai_...", "oblodai_live_...")
            .base_url("https://api.oblodai.com"),
    )?;

    let payment = client.payments().create(json!({
        "amount": "10",
        "currency": "USD",
        "order_id": "order-1",
        "to_currency": "USDT",
        "network": "tron",
    }))?;

    println!("{}", payment.address); // адрес для оплаты
    println!("{}", payment.url);     // hosted-страница оплаты
    Ok(())
}
```

## Проверка вебхуков

Подпись вебхука отличается от подписи запроса — SDK делает и то, и другое. Для входящих вебхуков
берите **сырое тело** и заголовки `X-Webhook-Timestamp` / `X-Webhook-Signature`.

```rust
use oblodai::{construct_event, verify_webhook, VerifyOptions, WebhookHeaders, Error};
use serde_json::Value;

fn handle(secret: &str, raw_body: &[u8], ts: &str, sig: &str) {
    let headers = WebhookHeaders { timestamp: ts, signature: sig };

    // По умолчанию проверяется подпись И свежесть (replay-защита, окно 5 минут).
    match construct_event::<Value>(secret, raw_body, &headers, &VerifyOptions::default()) {
        Ok(event) => {
            if event["type"] == "payment" && event["status"] == "paid" {
                // пометить заказ event["order_id"] оплаченным (идемпотентно по uuid + status)
            }
        }
        Err(Error::Signature(_)) => { /* вернуть 403 */ }
        Err(_) => { /* прочее */ }
    }
}
```

Чтобы отключить replay-защиту: `VerifyOptions { max_age_seconds: 0, now: None }`.

## Обработка ошибок

Все ошибки — enum [`Error`]. Ошибки API несут машиночитаемый код.

```rust
use oblodai::Error;

match client.payouts().create(params) {
    Ok(payout) => { /* ... */ }
    Err(Error::Api { code, status, message, .. }) => {
        match code.as_str() {
            "payout.insufficient_funds" => { /* недостаточно средств */ }
            "payout.funds_maturing"    => { /* средства ещё дозревают — временно */ }
            _ => {}
        }
        eprintln!("{code} (HTTP {status}): {message}");
    }
    Err(e) => eprintln!("{e}"),
}
```

`err.is_retriable()` подскажет, временная ли ошибка; `err.code()` вернёт код для `Error::Api`.

### Варианты ошибок

| Вариант | Когда |
|---|---|
| `Error::Api { code, status, .. }` | API вернул конверт `error`. |
| `Error::Connection(_)` | Сеть недоступна или таймаут. |
| `Error::Signature(_)` | Не прошла проверка подписи вебхука. |
| `Error::Serialization(_)` | Ошибка (де)сериализации. |
| `Error::Config(_)` | Некорректная конфигурация. |

## Повторы (retry)

Временные ошибки (`5xx`, `429`, сетевые сбои) повторяются автоматически с экспоненциальным backoff
и джиттером. Ошибки запроса (`4xx`), а также `payout.funds_maturing` (средства ещё дозревают —
немедленный повтор не поможет) считаются терминальными и не повторяются. Заголовок `Retry-After`
от сервера соблюдается как есть (не обрезается до `max_delay`, лишь до абсолютного потолка в 300 с).

```rust
use oblodai::{Config, RetryConfig};
use std::time::Duration;

let config = Config::new("...", "...").retry(Some(RetryConfig {
    max_attempts: 4,
    initial_delay: Duration::from_millis(500),
    max_delay: Duration::from_secs(30),
}));
// .retry(None) — отключить повторы
```

> **Важно про таймаут.** Таймаут не означает, что операция не прошла. Повтор безопасен благодаря
> идемпотентности по `order_id`: если операция уже создана — вернётся она же, дубля не будет. Для
> `payments().create(...)` и `account().transfer_to_personal(...)` SDK **автоматически** подставляет
> стабильный `order_id` (`idem-<hex>`), если вы его не задали, — один и тот же ключ переиспользуется
> на всех повторах одного вызова. Задайте свой `order_id` явно, чтобы дедуплицировать между вызовами.

## Свой HTTP-транспорт

По умолчанию используется встроенный клиент на `reqwest` (фича `reqwest-client`, включена). Чтобы
подставить свой транспорт (или для тестов без сети), отключите фичу и реализуйте трейт
[`HttpTransport`]:

```toml
[dependencies]
oblodai = { version = "1", default-features = false }
```

```rust
use oblodai::{Client, Config, HttpTransport, HttpResponse, Error};
use std::sync::Arc;

struct MyTransport;
impl HttpTransport for MyTransport {
    fn post(&self, url: &str, headers: &[(String, String)], body: &[u8])
        -> Result<HttpResponse, Error> {
        // ваш HTTP-вызов
        # let _ = (url, headers, body);
        Ok(HttpResponse { status: 200, body: b"{}".to_vec() })
    }
}

let client = Client::with_transport(Config::new("p", "s"), Arc::new(MyTransport)).unwrap();
```

## Обзор методов

```rust
// Платежи
client.payments().create(params)
client.payments().info(uuid, order_id)
client.payments().history(params)
client.payments().services()
client.payments().qr(uuid, order_id)
client.payments().resend(uuid, order_id)
client.payments().refund(params)
client.payments().set_accepted(methods) / list_accepted()
client.payments().set_accuracy(params) / get_accuracy()
client.payments().set_autorefund(params) / get_autorefund()

// Выплаты
client.payouts().create(params)
client.payouts().create_mass(payouts, source)
client.payouts().info(uuid, order_id)
client.payouts().history(params)
client.payouts().services()
client.payouts().calculate(params)
client.payouts().approve(uuid)
client.payouts().refund(params)
client.payouts().get_fee_config() / set_fee_config(bool)
client.payouts().get_refund_fee_config() / set_refund_fee_config(bool)

// Кошельки
client.wallets().create(params)
client.wallets().block(address, force_block)
client.wallets().blocked_address_refund(uuid, address)
client.wallets().qr(address)

// Аккаунт
client.account().balance()
client.account().referral()
client.account().transfer_to_personal(params)
client.account().vrcs(enabled)

// Вебхуки
client.webhooks().register(url)
client.webhooks().deliveries()
client.webhooks().test_payment(params)

// Настройки
client.settings().list_auto_withdraw() / set_auto_withdraw(params) / delete_auto_withdraw(currency)
client.settings().list_allowlist() / add_allowlist(cidr) / remove_allowlist(cidr) / enable_allowlist(bool)

// Курсы (публично, без ключа)
client.rates().list(Some("ETH"))
```

## Замечания

- **Суммы — строки** в единицах валюты (`"25.00"`), не числа. Так сохраняется точность.
- **`order_id`/`reference` — ваш ключ идемпотентности.** Задавайте всегда для платежей и выплат.
  Если для `payments().create` или `account().transfer_to_personal` вы его опустите, SDK подставит
  автоматический `idem-<hex>`, чтобы повтор после таймаута не создал дубль.
- **Секрет — только на сервере.** SDK серверный; не встраивайте ключ в клиентские приложения.
- **Тела запросов** принимаются как `serde_json::Value` — стройте макросом `json!`.

## Лицензия

MIT
