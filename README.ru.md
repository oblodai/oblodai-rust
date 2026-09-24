<div align="center">

<a href="https://oblodai.com">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/oblodai/.github/main/brand/logo-white.svg">
    <img src="https://raw.githubusercontent.com/oblodai/.github/main/brand/logo-black.svg" alt="oblodai" height="52">
  </picture>
</a>

<h3>Официальный Rust SDK для платёжного шлюза <a href="https://oblodai.com">oblodai</a></h3>

Платежи, выплаты, платёжные ссылки, сплиты, статические кошельки, вебхуки — один API-ключ.

<a href="https://crates.io/crates/oblodai"><img src="https://img.shields.io/crates/v/oblodai?style=flat-square&label=crates.io" alt="crates.io"></a>
<a href="https://github.com/oblodai/oblodai-rust/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/oblodai/oblodai-rust/ci.yml?branch=main&style=flat-square&label=CI" alt="CI"></a>
<img src="https://img.shields.io/badge/MSRV-1.86-DEA584?style=flat-square" alt="MSRV 1.86">
<a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-000000?style=flat-square" alt="License: MIT"></a>

[Documentation](https://docs.oblodai.com) · [Dashboard](https://my.oblodai.com) · [Read in English →](README.md)

</div>

---

Официальный Rust SDK для платёжного шлюза **Oblodai**: приём платежей, выплаты, массовые операции
(батчи), платёжные ссылки, выплатные ссылки (крипточеки), сплиты, статические кошельки, переводы,
вебхуки, документы. Все пространства имён, методы и модели **сгенерированы из OpenAPI-контракта
шлюза** — 120 операций, по методу на каждую — поверх рукописного runtime, который подписывает
запросы, безопасно повторяет, держит один ключ идемпотентности на вызов и помечает каждый вызов
заголовком `X-Request-ID`. Rust 2021, **MSRV 1.86**, асинхронный клиент на `reqwest` + `tokio` с
rustls (без OpenSSL); синхронный клиент включается фичей, а с `--no-default-features` модели,
помощники для сумм, проверка вебхуков и точка расширения `HttpBackend` собираются вообще без
`reqwest`.

> **Базовый URL.** По умолчанию `https://api.oblodai.com`. При необходимости переопределите
> `base_url` и передайте свои ключи при инициализации. Схема должна быть `https://`; обычный
> `http://` допускается только для локального хоста (`http://127.0.0.1:8095`) или с явной опцией
> `allow_insecure_base_url`.

## Установка

```toml
[dependencies]
oblodai = "2.0"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
futures-util = "0.3"   # only for `.stream()` / `.by_page()` — the `StreamExt` adapters live there
```

Нужен Rust **1.86** или новее. Нижняя граница задаётся деревом зависимостей, а не кодом SDK; её
повышение — минорная версия.

| фича              | что добавляет                                                                                          |
| ----------------- | ------------------------------------------------------------------------------------------------------ |
| `reqwest-client`  | *(по умолчанию)* асинхронный `Client` на `reqwest` с rustls                                            |
| `blocking`        | синхронный `blocking::Client` на том же чистом ядре                                                    |
| `native-roots`    | доверять ещё и хранилищу сертификатов ОС (прокси с TLS-инспекцией, шлюз с частным УЦ)                  |

### Синхронный клиент

```toml
oblodai = { version = "2.0", features = ["blocking"] }
```

```rust
use oblodai::models::{HistoryRequest, PaymentRequest};

let client = oblodai::blocking::Client::from_env()?;
let invoice = client
    .payments()
    .create(PaymentRequest::new("25", "USDT"))
    .send()?;
for payout in client
    .payouts()
    .list_history(HistoryRequest::default())
    .iter()
{
    println!("{}", payout?.uuid);
}
```

То же сгенерированное дерево методов на том же чистом ядре; билдеры отправляются `.send()`, списки
перебираются `.iter()` / `.by_page()`.

## Где взять ключи

Ключи — в кабинете [my.oblodai.com](https://my.oblodai.com) → **API-ключи**. Ключ — это публичный
идентификатор и секрет:

| ключ      | публичный id         | секрет                |
| --------- | -------------------- | --------------------- |
| боевой    | `oblodai_<hex>`      | `oblodai_live_<hex>`  |
| песочница | `test_oblodai_<hex>` | `oblodai_test_<hex>`  |

У мерчанта **один API-ключ**, и он подписывает все маршруты, которые умеет вызывать SDK, — выбирать
ключ под вызов не нужно:

```rust
let client = oblodai::Client::builder()
    .public_id(public_id)
    .secret(secret)
    .build()?;
```

`Client::from_env()` читает `OBLODAI_PUBLIC_ID` / `OBLODAI_SECRET`. **Ключ песочницы** работает с
копией шлюза без блокчейна — баланс из крана, симулированные депозиты, настоящие вебхуки — и
берётся в кабинете или через `sandbox().onboard_store(merchant_id)`. Интегрируйтесь сначала на нём.

Второй вид учётных данных, **админ-токен онбординга**, есть только у самостоятельно развёрнутого
шлюза: он уходит заголовком `X-Admin-Token` на маршрут подключения магазина
(`sandbox().onboard_store`) и больше никуда. Задаётся `.admin_token(…)` или `OBLODAI_ADMIN_TOKEN`.

Только мерчант, подключённый задолго до перехода на один ключ, может ещё держать старую пару
(`oblodai_pk_…` / `oblodai_wk_…`); на маршрутах другой половины такая пара получает 403
`merchant.wrong_key_kind`. Замените её API-ключом мерчанта.

## Быстрый старт

```rust
use oblodai::models::PaymentRequest;
use oblodai::Client;

let client = Client::from_env()?;
let invoice = client
    .payments()
    .create(PaymentRequest {
        network: Some("tron".into()), // omit to let the payer choose on the pay page
        order_id: Some("order-1001".into()), // your reference; idempotent per order_id
        url_callback: Some("https://shop.example/oblodai/webhook".into()),
        ..PaymentRequest::new("25", "USDT") // amount (a decimal string) and currency
    })
    .await?;
println!(
    "pay at {} — {} {}",
    invoice.url, invoice.address, invoice.status
);
```

Тела запросов — типизированные модели (`oblodai::models`): `Model::new(обязательные…)` задаёт
обязательные поля, остальное — синтаксисом обновления структуры. Суммы — `Money`, десятичная
строка; `From<f64>` нет, поэтому float не компилируется (а пришедший через `oblodai::from_json`
отвергается с `sdk.float_amount` до отправки).

Выплата — тем же ключом:

```rust
use oblodai::models::PayoutRequest;

let payout = client
    .payouts()
    .create(PayoutRequest {
        network: Some("tron".into()),
        ..PayoutRequest::new(
            "TQrY8bkbpXKPt2LZbU8jqfnpFbUSF15sbx",
            "10",
            "USDT",
            "payout-1001",
        )
    })
    .idempotency_key("payout-1001") // makes the retry safe across restarts too
    .await?;
println!("{} {}", payout.uuid, payout.status);
```

Готовые программы — в [`examples/`](examples); каждая прогоняется в `tests/examples.rs` против
подставного шлюза.

## Опции вызова

Каждый метод возвращает билдер; ничего не уходит, пока его не дождались (`.send()` у синхронного
клиента). Опции задаются на билдере:

```rust
use std::time::Duration;

let balance = client
    .account()
    .get_balance()
    .timeout(Duration::from_secs(10)) // one attempt
    .deadline(Duration::from_secs(45)) // the whole call, retries and pauses included
    .max_retries(5)
    .extra_header("X-Tenant", "eu") // this call only
    .request_id("order-1001-balance") // X-Request-ID; a fresh UUID when not set
    .await?;
```

| опция                  | что делает                                                                                        |
| ---------------------- | ------------------------------------------------------------------------------------------------- |
| `.idempotency_key(k)`  | свой ключ (на маршрутах, которые шлюз дедуплицирует; иначе `sdk.idempotency_unsupported`)         |
| `.timeout(Duration)`   | одна попытка (по умолчанию 30 с)                                                                  |
| `.deadline(Duration)`  | весь вызов с повторами и паузами (по умолчанию 90 с)                                              |
| `.max_retries(n)`      | повторов после первой попытки для этого вызова (по умолчанию 2; `0` — одна отправка)              |
| `.extra_header(n, v)`  | заголовок только этого вызова; заголовки самого SDK не переопределяются                           |
| `.request_id(id)`      | `X-Request-ID` вызова — один на все попытки; без него — свежий UUID                               |

Идентификатор запроса связывает ваши логи с логами шлюза: он есть в тексте каждой ошибки,
`[payout.insufficient_funds] not enough USDT (request_id=…)`, и в `err.request_id()`.

## Обзор методов

`client.<ресурс>().<метод>(…)` — по методу на операцию OpenAPI; имя — `operationId` без имени
ресурса. `names.lock` фиксирует все имена; старые имена 1.x — в [MIGRATION-2.0.md](MIGRATION-2.0.md).
Таблицу ниже пишет генератор.

<!-- sdkgen:methods -->
16 ресурсов, 120 методов.

| Ресурс | Методы |
| --- | --- |
| `payments()` | `create` · `get_info` · `get_qr` · `list_history` · `list_services` · `cancel` · `send_email` · `set_checkout_config` · `get_checkout_config` · `get_aml_links` · `resolve` |
| `payment_links()` | `create` · `list` · `get` · `toggle` |
| `refunds()` | `payment` · `blocked_wallet` |
| `payouts()` | `create` · `create_mass` · `get_info` · `list_history` · `calculate` · `validate` · `cancel` · `approve` · `list_services` · `transfer_to_personal` · `transfer_to_user` · `create_transfer_batch` |
| `payout_links()` | `create` · `create_batch` · `list` · `get` · `cancel` · `get_payout_claim` · `claim_payout` |
| `batches()` | `create_payment` · `create_refund` · `create_payout` · `get_info` |
| `splits()` | `create_rule` · `list_rules` · `delete_rule` · `set_config` · `get_config` · `set_recipient_opt_in` · `get_recipient_opt_in` |
| `wallets()` | `create` · `block` · `get_qr` |
| `account()` | `get_balance` · `get_summary` · `list_exchange_rates` |
| `webhooks()` | `resend_payment` · `register` · `list_deliveries` · `requeue_delivery` · `send_legacy_test` · `send_test_payment` · `send_test_wallet` · `send_test_payout` · `send_test_conversion` · `rotate_secret` · `set_active` |
| `settings()` | `set_accuracy` · `get_accuracy` · `set_auto_refund` · `get_auto_refund` · `set_discount` · `list_discounts` · `list_api_log` · `get_auto_convert` · `set_auto_convert` · `set_accepted_currencies` · `list_accepted_currencies` · `set_payout_fee_config` · `get_payout_fee_config` · `set_refund_fee_config` · `get_refund_fee_config` · `set_payment_fee_config` · `get_payment_fee_config` · `set_auto_withdraw_rule` · `list_auto_withdraw_rules` · `delete_auto_withdraw_rule` · `configure_vrcs` |
| `api_allowlist()` | `list` · `add_entry` · `remove_entry` · `set_enabled` |
| `referrals()` | `get_info` |
| `documents()` | `get_signed` · `get_balance` · `get_fees` · `get_ledger` · `get_split` · `get_payout_link_cheque` · `get_statement` · `get_batch` · `get_payment_link` · `get_wallet_statement` · `get_referrals` · `create_job` · `get_job` · `download_job_file` |
| `checkout()` | `get_source_of_funds_form` · `submit_source_of_funds` · `get_public_payment_link` · `payment_link` · `list_currencies` · `get` · `select_method` · `start_onramp` · `get_onramp` · `get_qr` |
| `sandbox()` | `onboard_store` · `faucet` · `simulate_deposit` · `reset` · `list_webhooks` · `replay_webhook` |
<!-- /sdkgen:methods -->

Маршрут документа отвечает `FileResult { bytes, content_type, filename }`. Модели сохраняют
незнакомые этой версии SDK поля в `extra`, у каждого enum есть вариант `Other(String)` — ответ более
нового шлюза всё равно разбирается. `Debug` модели не печатает значения полей с «секретными»
именами (`secret`, `token`, `passcode`, `claim_url`, …).

### Списки

Методы списков возвращают `Pager`. Ничего не запрашивается, пока его не начали читать.

```rust
use futures_util::StreamExt;
use oblodai::enums::PayoutKind;
use oblodai::models::HistoryRequest;

// one page
let page = client
    .payments()
    .list_history(HistoryRequest::default())
    .limit(50)
    .await?;
println!("{} of {}", page.items.len(), page.paginate.total);

// every item, one page fetched at a time
let mut payouts = client
    .payouts()
    .list_history(HistoryRequest::default())
    .stream();
while let Some(payout) = payouts.next().await {
    println!("{}", payout?.uuid);
}

// page by page
let mut pages = client
    .payments()
    .list_history(HistoryRequest::default())
    .by_page();
while let Some(page) = pages.next().await {
    println!("a page of {}", page?.items.len());
}

// or collect, with a cap
let refunds = client
    .payouts()
    .list_history(HistoryRequest {
        kind: Some(PayoutKind::Refund),
        ..Default::default()
    })
    .all(Some(1000))
    .await?;
```

### Долгие операции

Пакеты и выгрузки документов доделываются в фоне. `.job()` отправляет создание и возвращает `Job`,
который умеет за ним следить:

```rust
use oblodai::models::{DocumentJobRequest, PayoutBatchRequest};
use oblodai::JobStatus;

// a batch: `job()` sends the create call, `wait()` polls batches().get_info() until it ends
let job = client
    .batches()
    .create_payout(PayoutBatchRequest::default())
    .job()
    .await?;
let batch = job.wait().await?; // status `completed` or `stopped`
println!(
    "{}: {} ok, {} failed",
    job.id(),
    batch.succeeded,
    batch.failed
);

// a document export: wait, then download the file
let job = client
    .documents()
    .create_job(DocumentJobRequest::new("statement"))
    .job()
    .await?;
if job.wait().await?.status() == "done" {
    let file = job.download().await?;
    println!("{} bytes of {}", file.bytes.len(), file.content_type);
}
```

`wait()` опрашивает каждые 2 с не дольше 5 минут (`wait_with(timeout, interval)` — чтобы изменить)
и возвращает конечный ответ — задача в `failed` возвращается, а не бросается; не уложились —
`sdk.job_timeout`. Какие операции долгие и когда задача закончена — из контракта (`x-sdk-poll`):
`oblodai::lro::LRO` генерируется.

### Сырой ответ, копии клиента, хуки

```rust
use oblodai::{ClientOptions, Hooks};
use std::time::Duration;

// status, headers and request id of a successful call; `parse()` gives the usual value
let raw = client.account().get_balance().with_raw_response().await?;
println!("{} {}", raw.status(), raw.request_id());
let balance = raw.parse()?;

// a copy of the client with other settings; the original is untouched
let patient = client.with_options(ClientOptions::new().timeout(Duration::from_secs(60)));

// hooks see every attempt (the signature is redacted)
let traced = client.with_options(
    ClientOptions::new().hooks(
        Hooks::new()
            .on_request(|r| println!("-> {} {} #{}", r.method, r.url, r.attempt))
            .on_response(|r| println!("<- {} in {:?}", r.status, r.elapsed)),
    ),
);
traced.account().get_balance().await?;
```

### Статусы и суммы

- Платёж: `select → created → confirm_check → paid | paid_over | wrong_amount | expired | cancelled`
  (между ними — `under_review`). `helpers::is_payment_paid(&status)` истинно для `paid`/`paid_over`;
  `wrong_amount` (недоплата) ждёт `payments().resolve(…)`.
- Выплата: `pending → approved → awaiting_cosign → broadcasting → sent → confirmed | failed | cancelled`.
- `add_amounts`, `subtract_amounts`, `compare_amounts`, `amounts_equal`, `is_zero_amount` — точная
  десятичная арифметика над `Money`. У `Money` нет `Ord`: порядок строк — не порядок чисел.

## Песочница и тестирование

С ключом `test_` шлюз держит полноценного мерчанта, который не касается блокчейна:

```rust
use oblodai::generated::resources::SandboxListWebhooksQuery;
use oblodai::models::{FaucetRequest, SimulateDepositRequest, TestWebhookKindRequest};

// test money to pay out from (`test_` keys only)
client
    .sandbox()
    .faucet(FaucetRequest::new("1000", "USDT"))
    .await?;

// "pay" an invoice; repeat the same txid with more confirmations to walk pending → paid
client
    .sandbox()
    .simulate_deposit(SimulateDepositRequest::new(invoice_uuid))
    .await?;

// a rehearsal delivery: signed exactly like a live one, and marked `test: true`
client
    .webhooks()
    .send_test_payment(TestWebhookKindRequest::new(
        "https://shop.example/oblodai/webhook",
    ))
    .await?;

// what was delivered, with payloads — then a clean slate
let deliveries = client
    .sandbox()
    .list_webhooks(SandboxListWebhooksQuery::default())
    .all(None)
    .await?;
client.sandbox().reset().await?;
```

Репетиционная доставка несёт `test: true` в подписанном теле и `X-Webhook-Test: true` в заголовках,
это `delivery.is_test` — никогда не зачисляйте по ней заказ.

## Вебхуки

Зарегистрируйте адрес через `webhooks().register(…)` — секрет подписи показывается один раз — и
проверяйте каждую доставку по **сырым** байтам, до любого разбора:

```rust
use oblodai::enums::PaymentStatus;
use oblodai::webhooks::{verify_webhook_delivery, Headers, VerifyOptions};
use oblodai::WebhookEvent;

let headers = Headers::from_pairs(request_headers); // any (name, value) pairs
let delivery = verify_webhook_delivery(raw_body, &headers, &VerifyOptions::new(secret))?;

if delivery.is_test {
    return Ok(()); // a rehearsal: signed like a live one, but nothing moved
}

match &delivery.event {
    WebhookEvent::Payment(p) if p.status == PaymentStatus::Paid => mark_order_paid(&p.order_id),
    _ => {}
}
```

- **Дубли и порядок.** `delivery.id` (`X-Webhook-Id`) не меняется между повторами — дедуплицируйте
  по нему. `event.sequence()` упорядочивает события; `is_stale_event` отбрасывает опоздавшее.
- **Ротация.** После `webhooks().rotate_secret()` передавайте `.previous_secret(old)`, пока не
  пройдёт `previous_secret_valid_until`.
- **Незнакомые типы событий** приходят как `WebhookEvent::Other(Value)` (enum `#[non_exhaustive]`);
  известные — `Payment`, `Payout`, `Wallet` и `Conversion` поверх сгенерированных моделей.
- **Плохое тело — не плохая подпись.** Проверенная доставка, тело которой не читается, —
  `webhook.bad_payload` (`kind() == Contract`). Отвечайте 401 только на провал подписи.

Модулю `oblodai::webhooks` не нужны ни клиент, ни ключ; он собирается с `--no-default-features`.

## Ошибки

Любой сбой — `oblodai::Error`: `code()` (`payout.insufficient_funds`), `http_status()`,
`retryable()`, `retry_after()`, `request_id()`, `field()`, `synthetic()` (ответил прокси, а не API)
и `kind()` для сопоставления (`Validation` 400, `Authentication` 401, `Permission` 403,
`NotFound` 404, `Conflict` / `IdempotencyConflict` 409, `RateLimit` 429, `Unavailable` 503,
`Internal` прочие 5xx, `Transport`, `Config`, `Contract`, `Signature`). Напечатанная, ошибка
выглядит как `[код] текст (request_id=…)`. Ветвитесь по коду, а не по тексту:

```rust
match client.payouts().create(params).await {
    Ok(payout) => Ok(payout),
    Err(err) => {
        // `[payout.insufficient_funds] … (request_id=…)`
        eprintln!("{err}");
        match err.code() {
            // retryable — the balance may still arrive
            "payout.insufficient_funds" | "payout.funds_maturing" => {
                schedule_retry(err.retry_after().unwrap_or(60));
                Err(err)
            }
            _ => Err(err), // the SDK already retried what was safe to retry
        }
    }
}
```

Коды, которые выдаёт сам SDK, а не шлюз: `sdk.missing_credentials`, `sdk.bad_config`,
`sdk.bad_header`, `sdk.bad_path_param`, `sdk.bad_idempotency_key`, `sdk.idempotency_unsupported`,
`sdk.float_amount`, `sdk.bad_params`, `sdk.bad_amount`, `sdk.bad_envelope`,
`sdk.response_too_large`, `sdk.job_timeout`, `sdk.no_download`, `webhook.bad_payload` и семейство
`transport.timeout` / `transport.network` / `transport.deadline`.

## Повторы, идемпотентность и таймауты

- **Безопасно ли повторить** — из контракта: `GET` и операции с `x-retry-safe` либо запись, которую
  шлюз дедуплицирует по `Idempotency-Key` (`x-idempotent`). Никаких эвристик по виду пути.
- **Ключи идемпотентности** ставятся сами на дедуплицируемых маршрутах — один на логический вызов,
  тот же на каждом повторе, — поэтому таймаут никогда не даст второй выплаты.
- **Когда бывает повтор.** Только когда API говорит `retryable: true`; ответы без конверта API
  (502/503 прокси) и сбои транспорта — только если повторять безопасно. `Retry-After` соблюдается,
  иначе экспоненциальная пауза с разбросом. По умолчанию: 2 повтора, 250 мс → 4 с.
- **Расхождение часов** правится один раз по заголовку `Date` сервера и откатывается, если не помогло.
- **Редиректы не выполняются**; тела ответов ограничены (8 МиБ JSON, 64 МиБ документы).
- **Ограничивайте вызов `.deadline(…)`, а не бросанием future**: автоматический ключ живёт в этом
  future. Если повтор должен пережить перезапуск, передайте свой `.idempotency_key(…)`.

## Настройка

`Client::new(public_id, secret)`, `Client::from_env()` или `Client::builder()`:

| опция                           | по умолчанию               | смысл                                                     |
| ------------------------------- | -------------------------- | --------------------------------------------------------- |
| `.public_id(…)` / `.secret(…)`  | —                          | API-ключ; оба или ни одного                               |
| `.base_url(…)`                  | `https://api.oblodai.com`  | адрес API; префикс пути сохраняется                       |
| `.timeout(…)` / `.deadline(…)`  | 30 с / 90 с                | одна попытка / весь вызов                                 |
| `.max_retries(n)`, `.retry(…)`  | 2 повтора                  | `0` отключает повторы                                     |
| `.header(name, value)`          | —                          | дополнительный заголовок на каждом запросе                |
| `.hooks(Hooks)`                 | нет                        | `on_request` / `on_response`, раз на попытку              |
| `.admin_token(…)`               | —                          | `X-Admin-Token` только на маршруте подключения            |
| `.allow_insecure_base_url(…)`   | `false`                    | разрешить обычный `http` не только для localhost          |
| `.logger(…)`                    | нет                        | структурный логгер; секреты скрываются до него            |
| `.http_backend(…)`              | `reqwest`                  | заменить HTTP-слой                                        |

Окружение: `OBLODAI_PUBLIC_ID`, `OBLODAI_SECRET`, `OBLODAI_BASE_URL`, `OBLODAI_ADMIN_TOKEN`,
`OBLODAI_LOG` (`debug|info|warn|error`), `OBLODAI_ALLOW_INSECURE` (`1`).

## Разработка

Код в `src/generated/` генерирует `tools/sdkgen` бэкенда из `services/core/api/openapi.json`;
руками его не править — перегенерировать `make sdk` в бэкенде. `make ci` прогоняет все ворота:
проверку дрейфа (генерация во временный каталог и сравнение), `cargo fmt`, clippy по комбинациям
фич, тесты (модульные, общий набор сценариев conformance бэкенда, примеры и этот README против
подставного шлюза), rustdoc, упаковку и проверку MSRV.

```sh
OBLODAI_BACKEND=../oblodai-backend make ci
OBLODAI_LIVE_URL=http://127.0.0.1:8095 make live   # против настоящего шлюза
```

[AGENTS.md](AGENTS.md) — сжатое руководство для агентов-программистов,
[CHANGELOG.md](CHANGELOG.md) — что изменилось, [MIGRATION-2.0.md](MIGRATION-2.0.md) — переход с 1.x.

## Лицензия

MIT — см. [LICENSE](LICENSE).
