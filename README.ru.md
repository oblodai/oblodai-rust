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
вебхуки. Подпись запросов, разбор ответов, типизированные ошибки, идемпотентность и повторы — из
коробки. Rust 2021, **MSRV 1.86**, асинхронный клиент на `reqwest` + `tokio` с rustls (без
OpenSSL); синхронный клиент включается фичей, а с `--no-default-features` собираются типы контракта,
помощники для сумм, проверка вебхуков и точка расширения `HttpBackend` — вообще без `reqwest` в
дереве зависимостей.

> **Базовый URL.** По умолчанию `https://api.oblodai.com`. При необходимости переопределите
> `base_url` и передайте свои ключи при инициализации. Схема должна быть `https://`; обычный
> `http://` допускается только для локального хоста (`http://127.0.0.1:8095`) или с явной опцией
> `allow_insecure_base_url`.

## Установка

```toml
[dependencies]
oblodai = "1.3"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
futures-util = "0.3"   # only for `.stream()` — the `StreamExt` adapters live there
```

Нужен Rust **1.86** или новее; CI проверяет сборку ровно на этом тулчейне. Нижняя граница задана
деревом зависимостей (`reqwest` → `url` → `idna`/`icu`), а не кодом самого SDK; её подъём — это
изменение минорной версии.

Фичи:

| фича              | что добавляет                                                                                                                                                          |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `reqwest-client`  | *(по умолчанию)* асинхронный `Client` на `reqwest` с rustls                                                                                                            |
| `blocking`        | синхронный `blocking::Client` поверх того же чистого ядра                                                                                                              |
| `native-roots`    | дополнительно доверять системному хранилищу сертификатов — нужно за TLS-инспектирующим прокси или для шлюза с приватным CA; без неё используются только вшитые корни webpki |

### Синхронный клиент

```toml
oblodai = { version = "1.3", features = ["blocking"] }
```

```rust
let client = oblodai::blocking::Client::from_env()?;
let invoice = client.payments().create(params).send()?;
for payout in client.payouts().history(Default::default()).iter() {
    println!("{}", payout?.uuid);
}
```

Это то же самое дерево методов поверх того же чистого ядра (подпись, конверты, решения о повторах);
отличается только ввод-вывод.

## Где взять ключи

Ключи выдаются в личном кабинете [my.oblodai.com](https://my.oblodai.com) → **API-ключи**. Ключ —
это публичный идентификатор плюс секрет:

| ключ     | публичный идентификатор | секрет                |
| -------- | ----------------------- | --------------------- |
| боевой   | `oblodai_<hex>`         | `oblodai_live_<hex>`  |
| песочный | `test_oblodai_<hex>`    | `oblodai_test_<hex>`  |

У мерчанта **один API-ключ**, и он подписывает все маршруты, доступные SDK: счета и платёжные
ссылки, выплаты, возвраты и чеки, настройки, кошельки, отчёты. Выбирать нечего:

```rust
let client = oblodai::Client::builder()
    .public_id(public_id)
    .secret(secret)
    .build()?;
```

**Песочный ключ** работает с копией шлюза без блокчейна — фейковый баланс из крана, симулированные
депозиты, настоящие вебхуки — и выдаётся песочным онбордингом (`merchants().sandbox(…)` или личный
кабинет). Интегрируйтесь сначала на нём; боевые и песочные ключи разделены, и ни один не видит
данных другого.

Второй вид доступа, **админ-токен онбординга**, существует только на self-hosted шлюзе: он уходит
в заголовке `X-Admin-Token` на маршрутах провижининга `merchants()` и больше нигде. Задаётся через
`.admin_token(…)` или `OBLODAI_ADMIN_TOKEN`.

Только мерчант, заведённый задолго до перехода на один ключ, может ещё держать старую разделённую
пару (`oblodai_pk_…` для приёма, `oblodai_wk_…` для вывода); такая пара на маршрутах другой половины
отбивается 403 `merchant.wrong_key_kind`. Замените её на API-ключ мерчанта.

## Быстрый старт

Создайте клиента из окружения и примите платёж:

```rust
use oblodai::contract::requests::PaymentRequest;
use oblodai::Client;

let client = Client::from_env()?;
let invoice = client
    .payments()
    .create(PaymentRequest {
        amount: "25".into(),                 // amounts are decimal strings, never floats
        currency: "USDT".into(),             // what you price in: a fiat or an asset
        network: Some("tron".into()),        // omit to let the payer choose on the pay page
        order_id: Some("order-1001".into()), // your reference; idempotent per order_id
        url_callback: Some("https://shop.example/oblodai/webhook".into()),
        ..Default::default()
    })
    .await?;
println!(
    "pay at {} — {} {}",
    invoice.url, invoice.address, invoice.status
);
```

`Client::from_env()` читает `OBLODAI_PUBLIC_ID` / `OBLODAI_SECRET` (и остальные `OBLODAI_*` ниже), но
не требует их: клиент без учётных данных собирается нормально и падает на первом подписанном вызове
с `sdk.missing_credentials`.

Чтобы выставлять цену в фиате, берите одну валюту, а получайте другую: `amount: "25".into(),
currency: "USD".into(), to_currency: Some("USDT".into())` — `currency` это то, в чём вы выставили
счёт, а `to_currency` — актив, который отправляет плательщик.

Вывод денег идёт тем же ключом:

```rust
use oblodai::contract::requests::PayoutRequest;

let payout = client
    .payouts()
    .create(PayoutRequest {
        amount: "10".into(),
        currency: "USDT".into(),
        network: Some("tron".into()),
        address: "TQrY8bkbpXKPt2LZbU8jqfnpFbUSF15sbx".into(),
        order_id: "payout-1001".into(),
        ..Default::default()
    })
    .idempotency_key("payout-1001") // makes the retry safe across restarts too
    .await?;
println!("{} {}", payout.uuid, payout.status);
```

Больше запускаемых программ — в [`examples/`](examples): счёт, доведённый до оплаты, выплата с
предварительным расчётом и «сухим прогоном», и приёмник вебхуков.

## Песочница и тестирование

С ключом `test_` шлюз держит полноценного мерчанта, который никогда не выходит в сеть блокчейна.
Начислите себе денег, симулируйте депозит, прогоните вебхук и всё обнулите:

```rust
use oblodai::contract::requests::{
    SandboxDepositRequest, SandboxFaucetRequest, TestWebhookPaymentRequest,
};
use oblodai::WebhookKind;

// test money to pay out from (`test_` keys only)
client
    .sandbox()
    .faucet(SandboxFaucetRequest {
        amount: "1000".into(),
        asset: "USDT".into(),
        ..Default::default()
    })
    .await?;

// "pay" an invoice; repeat the same txid with more confirmations to walk the pending→paid path
client
    .sandbox()
    .deposit(SandboxDepositRequest {
        invoice_id: invoice.uuid.clone(),
        ..Default::default()
    })
    .await?;

// a rehearsal delivery: signed exactly like a live one, and marked `test: true`
client
    .webhooks()
    .test(
        WebhookKind::Payment,
        TestWebhookPaymentRequest {
            url_callback: "https://shop.example/oblodai/webhook".into(),
            ..Default::default()
        },
    )
    .await?;

// what was delivered, with payloads — then a clean slate
let deliveries = client
    .sandbox()
    .webhooks(Default::default())
    .all(None)
    .await?;
client.sandbox().reset().await?;
```

`sandbox().replay(delivery_id)` повторно отправляет доставку, уже дошедшую до терминального
состояния. `sandbox().reset()` отменяет открытые счета и обнуляет балансы.
Репетиционная доставка несёт `test: true` в подписанном теле и `X-Webhook-Test: true` в заголовках —
это видно как `delivery.is_test`; никогда не засчитывайте по ней заказ.

## Обзор методов

Шестнадцать неймспейсов покрывают все **107 маршрутов** мерчантского API.

| неймспейс         | методы                                                                                                                                                                                                   | маршруты                                                                                                                                                                                                                                                  |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `payments()`      | create · info/get · cancel · history/list · batch · qr · services · send_email · resend · public_view · select · public_qr                                                                                | `POST /v1/payment` · `/payment/info` · `/payment/cancel` · `/payment/history` · `/payment/batch` · `/payment/qr` · `/payment/services` · `/payment/send-email` · `/payment/resend` · `GET /v1/pay/{id}` · `POST /v1/pay/{id}/select` · `GET /v1/pay/{id}/qr` |
| `refunds()`       | create · resolve · batch                                                                                                                                                                                 | `POST /v1/payment/refund` · `/payment/resolve` · `/refund/batch`                                                                                                                                                                                           |
| `payouts()`       | create · validate · calculate · info/get · cancel · approve · history/list · mass · batch · services · get/set_fee_config · get/set_refund_fee_config                                                     | `POST /v1/payout` · `/payout/validate` · `/payout/calculate` · `/payout/info` · `/payout/cancel` · `/payout/approve` · `/payout/history` · `/payout/mass` · `/payout/batch` · `/payout/services` · `/payout/fee-config/{get,set}` · `/payout/refund-fee-config/{get,set}` |
| `payout_links()`  | create · info/get · list · cancel · batch · cheque · claim_preview · claim                                                                                                                                | `POST /v1/payout/link` · `/payout/link/info` · `/payout/link/list` · `/payout/link/cancel` · `/payout/link/batch` · `/payout/link/cheque` · `GET /v1/claim/{token}` · `POST /v1/claim/{token}`                                                              |
| `payment_links()` | create · info/get · list · toggle · public_view · checkout                                                                                                                                                | `POST /v1/payment/link` · `/payment/link/info` · `/payment/link/list` · `/payment/link/toggle` · `GET /v1/link/{id}` · `POST /v1/link/{id}/checkout`                                                                                                        |
| `batches()`       | info                                                                                                                                                                                                     | `POST /v1/batch/info`                                                                                                                                                                                                                                     |
| `transfers()`     | to_personal · to_user · batch                                                                                                                                                                            | `POST /v1/transfer/to-personal` · `/transfer/to-user` · `/transfer/batch`                                                                                                                                                                                 |
| `wallets()`       | create · qr · block · refund_blocked_deposit                                                                                                                                                             | `POST /v1/wallet` · `/wallet/qr` · `/wallet/block` · `/wallet/blocked-address-refund`                                                                                                                                                                      |
| `webhooks()`      | register · rotate_secret · deliveries · test · test_legacy *(устарел)*                                                                                                                                   | `POST /v1/webhooks` · `/webhooks/rotate-secret` · `/webhooks/deliveries` · `/test-webhook/{payment,payout,wallet}` · `/payment/testing-webhook`                                                                                                             |
| `documents()`     | statement · ledger · balance_certificate · fee_schedule · split_report · batch_report · link_report · wallet_statement · referrals_report · create_job · job_info · job_file · download                    | `GET /v1/documents/statement` · `/documents/ledger` · `/documents/balance` · `/documents/fees` · `/documents/split` · `/documents/batch` · `/documents/link` · `/documents/wallet/statement` · `/documents/referrals` · `POST /v1/documents/jobs` · `/documents/jobs/info` · `GET /v1/documents/jobs/file` · `/documents/{kind}/{id}` |
| `splits()`        | create_rule · list_rules · delete_rule · get/set_config · get/set_opt_in                                                                                                                                 | `POST /v1/split/rule` · `/split/rule/list` · `/split/rule/delete` · `/split/config/{get,set}` · `/split/recipient/optin/get` · `/split/recipient/optin`                                                                                                     |
| `settings()`      | set_discount · list_discounts · get/set_accuracy · get/set_auto_refund · list/set_accepted · get/set_payment_fee_config · list/set/delete_auto_withdraw · list/add/remove/enable_api_allowlist             | `POST /v1/payment/discount/{set,list}` · `/payment/accuracy/{get,set}` · `/payment/autorefund/{get,set}` · `/payment/accepted/{list,set}` · `/payment/fee-config/{get,set}` · `/auto-withdraw/{list,set,delete}` · `/api-allowlist/{list,add,remove,enable}` |
| `account()`       | balance · referral · vrcs/set_vrcs *(в референсном SDK это `vrcs(enabled?)`; здесь два метода, потому что в Rust нет необязательных аргументов)*                                                          | `POST /v1/balance` · `/referral/info` · `/vrcs`                                                                                                                                                                                                            |
| `catalog()`       | currencies · exchange_rates                                                                                                                                                                              | `GET /v1/currencies` · `POST /v1/exchange-rate/list`                                                                                                                                                                                                       |
| `sandbox()`       | faucet · deposit · webhooks · replay · reset                                                                                                                                                             | `POST /v1/sandbox/faucet` · `/sandbox/deposit` · `GET /v1/sandbox/webhooks` · `POST /v1/sandbox/webhooks/replay` · `/sandbox/reset`                                                                                                                         |
| `merchants()`     | create · create_sandbox (провижининг; `admin_token` на self-hosted шлюзе)                                                                                                                                 | `POST /v1/merchants` · `/merchants/{id}/sandbox`                                                                                                                                                                                                           |

Каждый метод возвращает билдер, который одновременно является future: сделайте `.await` или сначала
задайте опции конкретного вызова.

Для поиска подходит голый uuid или `Lookup`: `payments().info("uuid")`,
`payments().info(Lookup::order_id("order-1001"))`. Там, где референсный SDK принимает
`string | model`, методы на Rust принимают `impl Into<IdRef>`, поэтому работают и
`payout_links().info(&link)`, и `payout_links().info("lnk_1")`.

`documents()` в основном отвечает `FileResult { bytes, content_type, filename }`; два маршрута задач
— обычный JSON: `create_job` и `job_info` возвращают `DocumentJob`, и только `job_file` отдаёт байты.

### Списки

Списочные методы возвращают `Pager`. Пока вы его не потребите, ни одного запроса не уходит.

```rust
use futures_util::StreamExt;

// one page
let page = client
    .payments()
    .history(Default::default())
    .limit(50)
    .await?;
println!("{} of {}", page.items.len(), page.paginate.total);

// every item, one page fetched at a time
let mut payouts = client.payouts().history(Default::default()).stream();
while let Some(payout) = payouts.next().await {
    println!("{}", payout?.uuid);
}

// or collect, with a cap
let refunds = client
    .payouts()
    .history(PayoutHistoryRequest {
        kind: Some(PayoutKind::Refund),
        ..Default::default()
    })
    .all(Some(1000))
    .await?;
```

### Статусы

- Платёж: `select → created → confirm_check → paid | paid_over | wrong_amount | expired | cancelled`.
  `is_payment_paid(&status)` истинно для `paid`/`paid_over`; `wrong_amount` (недоплата) ждёт
  `refunds().resolve(…)`; остальное покрывает `is_payment_final`.
- Выплата: `pending → approved → awaiting_cosign → broadcasting → sent → confirmed | failed | cancelled`,
  для неё есть `is_payout_final`.

Смену состояний лучше ловить вебхуками, а `info` опрашивать только как запасной вариант. У каждого
перечисления есть вариант `Other(String)`, поэтому значение, введённое более новым шлюзом, всё равно
разберётся.

### Суммы

`add_amounts`, `subtract_amounts`, `compare_amounts`, `amounts_equal`, `is_zero_amount` — точная
десятичная арифметика над строковыми суммами, которыми оперирует API. Никогда не разбирайте `Money`
как `f64`.

`Money` намеренно не реализует ни `Ord`, ни `PartialOrd`: производные реализации сравнивали бы
десятичные *строки*, и тогда `"9.00" > "10.00"` было бы истиной. `if amount > threshold` не
компилируется — используйте `compare_amounts`. Производный `PartialEq` — это точное равенство строк,
поэтому при возможных различиях в хвостовых нулях берите `amounts_equal`. Всё, что не является
десятичной строкой длиной не более 64 символов, даёт `AmountError`; ни один ввод не приводит к
панике.

## Вебхуки

Зарегистрируйте эндпоинт через `webhooks().register(url)` — секрет подписи показывается один раз — и
проверяйте каждую доставку по **сырым** байтам, до любого разбора:

```rust
use oblodai::webhooks::{verify_webhook_delivery, Headers, VerifyOptions};

let headers = Headers::from_pairs(request_headers); // any (name, value) pairs
let delivery = verify_webhook_delivery(raw_body, &headers, &VerifyOptions::new(secret))?;

if delivery.is_test {
    return Ok(()); // a rehearsal: signed like a live one, but nothing moved
}

match &delivery.event {
    oblodai::WebhookEvent::Payment(p) if p.status == oblodai::PaymentStatus::Paid => {
        mark_order_paid(p.order_id.as_deref())
    }
    _ => {}
}
```

- **Репетиции.** `webhooks().test()` и песочница подписывают доставки ровно так же, как боевые, и
  ставят `test: true` (плюс `X-Webhook-Test: true`). Проверяйте `delivery.is_test` (или
  `is_test_event(&delivery.event)` / `event.is_test()`) и никогда не реагируйте на такую доставку
  так, будто деньги двинулись.
- **Дубли и порядок.** `delivery.id` (`X-Webhook-Id`) не меняется между повторами — дедуплицируйте
  по нему. `event.sequence()` (это `Option<i64>`) задаёт порядок событий; `is_stale_event` отбрасывает
  пришедшее не по порядку и никогда не истинно, если последовательность отсутствует.
- **Ротация.** После `webhooks().rotate_secret()` передавайте `.previous_secret(old)` не менее 26
  часов — пока не пройдёт `previous_secret_valid_until`.
- **Неизвестные типы событий.** `WebhookEvent` помечен `#[non_exhaustive]` и имеет вариант
  `Other(Value)`: событие типа новее этого снимка приходит целым, а не ломается, поэтому всегда
  добавляйте ветку `_`. `is_known_event(&delivery.event)` (или `event.is_known()`) говорит, знает ли
  снимок `type` этого события; `event.raw()` отдаёт тело того, которого не знает.
- **Плохое тело — не плохая подпись.** Доставка с верной подписью, тело которой прочитать не удалось,
  даёт `webhook.bad_payload` с `kind() == Contract` — намеренно *не* ошибку подписи. Отвечайте
  **401 только на ошибки подписи**, иначе приёмник, отбивающий подделки, отобьёт и подлинное событие
  и заработает 26 часов повторов.
- Пустой секрет или отрицательный допуск — это ошибка `Config`, поднятая до какого-либо хеширования;
  `tolerance_seconds(0)` отключает проверку свежести (по умолчанию ±300 с).

Модулю `oblodai::webhooks` не нужны ни клиент, ни API-ключ; он доступен и с
`--no-default-features`. Типы событий — `invoice.<status>`, `payout.<status>` и `wallet.paid`; поле
`type` в теле — `payment | payout | wallet`, а `WebhookEvent` — соответствующее перечисление.

## Ошибки

Любой сбой — это `oblodai::Error`, несущий конверт ошибки API: `code()`
(`payout.insufficient_funds`), `http_status()`, `retryable()`, `retry_after()`, `request_id()`,
`field()`, `synthetic()` (ответил прокси, а не API) и `kind()` для сопоставления:

| `kind()`                          | HTTP        | когда                                                       |
| --------------------------------- | ----------- | ----------------------------------------------------------- |
| `Validation`                      | 400         | запрос отклонён; `field()` называет виновное поле            |
| `Authentication`                  | 401         | плохая подпись, отсутствующий или неизвестный ключ           |
| `Permission`                      | 403         | ключу это не разрешено (функция выключена, IP не в списке)   |
| `NotFound`                        | 404         | такого объекта нет                                           |
| `Conflict` / `IdempotencyConflict` | 409         | конфликт состояния; ключ переиспользован с другим телом      |
| `RateLimit`                       | 429         | превышен лимит; учитывайте `retry_after()`                   |
| `Unavailable`                     | 503         | шлюз занят или на обслуживании — можно повторить             |
| `Internal`                        | прочие 5xx  | сбой шлюза                                                   |
| `Api`                             | любой другой | статус, который шлюз вернул и который никуда больше не лёг   |
| `Transport`                       | —           | ответа не было: таймаут, соединение, дедлайн                 |
| `Config`                          | —           | отклонено до того, как что-либо было отправлено              |
| `Contract`                        | —           | ответ не удалось прочитать как конверт                       |
| `Signature`                       | —           | не прошла проверка подписи вебхука                           |

`retryable()` — источник истины: SDK уже повторил то, что следовало; `retry_after()` говорит, сколько
ждать; `request_id()` называйте в поддержке. Сырое тело никогда не печатается в `Debug` и никогда не
сериализуется (`serde_json::to_value(&err)` сохраняет сообщение и выбрасывает тело).

Конверт разбирается поле за полем: одно испорченное поле (дробный `retry_after`, числовой
`request_id`) не отнимет у вас `code`, по которому вы ветвитесь, а перебить `retryable` шлюза может
только литеральные `true`/`false`.

Ветвитесь по коду, а не по сообщению:

```rust
match client.payouts().create(params).await {
    Ok(payout) => Ok(payout),
    Err(err) => match err.code() {
        // retryable — the balance may still arrive
        "payout.insufficient_funds" | "payout.funds_maturing" => {
            schedule_retry(err.retry_after().unwrap_or(60));
            Err(err)
        }
        _ => Err(err), // the SDK already retried what was safe to retry
    },
}
```

Коды, которые стоит обрабатывать: `payout.insufficient_funds` и `payout.funds_maturing` (оба
повторяемы), `idempotency.key_reused`, `invoice.not_payable`, `payment.not_found`,
`merchant.bad_signature`, `request.rate_limited`. Полный каталог — **469 кодов** — доступен как
`oblodai::ERROR_CODES`, и каждый метод, двигающий деньги, перечисляет свои коды в собственной
rustdoc.

Коды, которые SDK поднимает сам, а не шлюз: `sdk.missing_credentials`, `sdk.bad_config`,
`sdk.bad_header`, `sdk.bad_path_param`, `sdk.bad_idempotency_key`, `sdk.idempotency_unsupported`,
`sdk.bad_amount`, `sdk.bad_envelope`, `sdk.response_too_large`, `webhook.bad_payload` и семейство
`transport.timeout` / `transport.network` / `transport.deadline`.

## Повторы, идемпотентность и таймауты

- **Безопасность повтора** берётся из флага `safe` самого контракта, проставленного шлюзом вручную
  для каждого маршрута и доступного как `RouteSpec::safe`. Никаких эвристик по форме пути в SDK нет.
- **Ключи идемпотентности** проставляются автоматически на маршрутах создания — один на логический
  вызов, переиспользуемый при каждом повторе, — так что таймаут не может породить вторую выплату.
  Передайте свой через `.idempotency_key(…)`, чтобы повтор был безопасен и между перезапусками; на
  маршрутах, которые шлюз не дедуплицирует, SDK отклонит ключ с `sdk.idempotency_unsupported` ещё до
  отправки.
- **Когда происходит повтор.** Ошибка повторяется, только если API сказал `retryable: true`. Ответы
  без конверта API (502/503 от прокси) и транспортные сбои повторяются только на читающих маршрутах
  и на записях с ключом. `Retry-After` учитывается (в секундах или как HTTP-дата, с зажимом —
  никогда не отрицательный и без переполнения), иначе — экспоненциальная пауза с джиттером.
- **Настройки.** `ClientBuilder::retry(RetryOptions { max_retries, base_delay_ms, max_delay_ms,
  max_retry_after_ms })` — по умолчанию 2 / 250 мс / 4 с / 30 с, а `max_retries: 0` отключает
  повторы.
- **Расхождение часов** корректируется один раз, по заголовку `Date` из ответа с ошибкой подписи, и
  коррекция откатывается, если переподписанная попытка так и не прошла аутентификацию. Смещения
  больше ±24 ч считаются сломанным прокси и игнорируются.
- **Редиректы никогда не выполняются** — ответ с другого origin сообщается, а не принимается.
- **Тела ответов ограничены**: 8 МиБ на маршрутах с конвертом и 64 МиБ на «голых» (`bare`)
  документных маршрутах; всё, что больше, — `sdk.response_too_large`.

Опции конкретного вызова задаются на билдере до `.await`:

```rust
client
    .payouts()
    .create(params)
    .idempotency_key("payout-42")
    .timeout(Duration::from_secs(10)) // one attempt
    .deadline(Duration::from_secs(45)) // the whole call, retries and pauses included
    .header("X-Request-Trace", "abc123") // this call only
    .await?;
```

Набор опций билдера следует из того, что за маршрут он обслуживает:

| билдер                                   | `.timeout` | `.deadline` | `.header` | `.idempotency_key`                                        |
| ---------------------------------------- | ---------- | ----------- | --------- | --------------------------------------------------------- |
| `RequestBuilder` (любой обычный маршрут) | ✓          | ✓           | ✓         | ✓                                                         |
| `FileBuilder` (`documents()`, `cheque`)  | ✓          | ✓           | ✓         | устарел — ни один документный маршрут не дедуплицируется  |
| `Pager` (любой список)                   | ✓          | ✓           | ✓         | — ключ на страницу заставил бы шлюз повторять первую       |

**Ограничивайте вызов через `.deadline(…)`, а не сбрасывая future.** Сброс отменяет вызов, а
автоматически сгенерированный ключ идемпотентности живёт внутри этой future — запрос, уже ушедший в
сеть, может всё-таки дойти до шлюза, и повторная отправка выпустит *новый* ключ, с которым шлюз не
сможет его сопоставить. Если повтор должен пережить отмену или перезапуск процесса, передайте свой
`.idempotency_key(…)`.

## Конфигурация

`Client::new(public_id, secret)`, `Client::from_env()` или `Client::builder()`. Каждая опция билдера
имеет запасной вариант — переменную окружения:

| опция                           | по умолчанию               | смысл                                                                   |
| ------------------------------- | -------------------------- | ----------------------------------------------------------------------- |
| `.public_id(…)` / `.secret(…)`  | —                          | API-ключ (`X-Public-Id` и секрет подписи); задаётся целиком или никак    |
| `.base_url(…)`                  | `https://api.oblodai.com`  | origin API; префикс пути (`https://gw.corp/oblodai`) сохраняется         |
| `.timeout(…)`                   | 30 с                       | на одну попытку                                                         |
| `.deadline(…)`                  | 90 с                       | на весь вызов, включая повторы и паузы                                  |
| `.retry(RetryOptions { … })`    | 2 повтора                  | политика пауз; `max_retries: 0` отключает повторы                       |
| `.header(name, value)`          | —                          | дополнительный заголовок на каждом запросе; подписываемые не подменяются |
| `.admin_token(…)`               | —                          | `X-Admin-Token`, уходит на маршрутах `merchants()` и больше нигде        |
| `.allow_insecure_base_url(…)`   | `false`                    | разрешить `http`-базовый URL за пределами локального хоста              |
| `.logger(…)`                    | нет                        | структурированный логгер                                                |
| `.http_backend(…)` / `.blocking_http_backend(…)` | `reqwest` | заменить HTTP-слой (клиент с прокси, записывающая заглушка)             |
| `.clock(…)`                     | системные часы             | часы для подписи, для тестов                                            |
| `.env(…)`                       | окружение процесса         | брать запасные значения из карты, а не из окружения                     |

Читаются ровно эти шесть переменных и никакие другие:

| переменная                  | опция                         | смысл                                                            |
| --------------------------- | ----------------------------- | ---------------------------------------------------------------- |
| `OBLODAI_PUBLIC_ID`         | `.public_id(…)`               | API-ключ, публичная половина (`X-Public-Id`)                      |
| `OBLODAI_SECRET`            | `.secret(…)`                  | API-ключ, секретная половина                                      |
| `OBLODAI_ADMIN_TOKEN`       | `.admin_token(…)`             | `X-Admin-Token`, уходит на маршрутах `merchants()` и больше нигде |
| `OBLODAI_BASE_URL`          | `.base_url(…)`                | origin API; по умолчанию `https://api.oblodai.com`                |
| `OBLODAI_LOG`               | `.logger(…)`                  | `debug\|info\|warn\|error` — ставит логгер в stderr               |
| `OBLODAI_ALLOW_INSECURE`    | `.allow_insecure_base_url(…)` | `1` разрешает `http`-базовый URL за пределами локального хоста    |

### Self-hosted или локальный шлюз

`base_url("http://localhost:8095")` работает сразу; для остальных `http`-хостов нужен
`.allow_insecure_base_url(true)` (или `OBLODAI_ALLOW_INSECURE=1`). Префикс пути в `base_url`
сохраняется, и каждый маршрут дописывается к нему.

### Секреты и логирование

Значения, похожие на секреты, заменяются на `[redacted]` ещё до того, как попадут в *любой* логгер,
включая ваш собственный. Модели ответов, несущие одноразовый секрет (`WebhookEndpoint.secret`,
`WebhookSecretRotated.secret`, `ApiKeyPair.secret`,
`PayoutLink.claim_token`/`claim_url`/`passcode`), скрывают его и в `Debug`, так что
`tracing::info!(?response)` безопасен; при сериализации значения сохраняются — иначе вы не смогли бы
сохранить то, что шлюз показал один раз.

## Снимок контракта

`contract/` выгружается собственным тестовым набором шлюза: реестр маршрутов, схемы DTO запросов с
английскими описаниями полей, перечисления, все коды ошибок, векторы подписи, эталонные тела ответов,
записанные с живого шлюза, и настоящие подписанные доставки вебхуков. Этот снимок: **107 маршрутов
мерчантского API, 469 кодов ошибок**, выгружен из ядра `2cc44c16f516`. Файлы
`src/contract/{routes,enums,requests,version}.rs` сгенерированы из него, а `oblodai::ROUTES`,
`ERROR_CODES`, `NETWORKS`, `PAYMENT_STATUSES`, `PAYOUT_STATUSES` и `EVENT_TYPES` открывают его в
рантайме.

```sh
python3 scripts/codegen.py            # regenerate after refreshing contract/
python3 scripts/codegen.py --check    # CI gate: fail when the two disagree
```

`tests/contract_routes.rs` сверяет каждое поле каждого маршрута с `contract/contract.json`, так что
сгенерированный код и снимок не могут незаметно разойтись.

## Разработка

```sh
git clone https://github.com/oblodai/oblodai-rust.git
cd oblodai-rust

cargo fmt --all --check
python3 scripts/codegen.py --check
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --all-targets --no-default-features -- -D warnings   # the feature matrix CI runs
cargo test --all-features                                         # unit + contract tests, no network
RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features

# against a real gateway
OBLODAI_LIVE_URL=http://127.0.0.1:8095 cargo test --all-features -- --ignored --test-threads=1
```

Каждый Rust-блок в этом файле компилируется тестом `tests/doc_snippets.rs`; он же проверяет, что
README.md несёт те же блоки кода байт в байт.

Смотрите также [AGENTS.md](AGENTS.md) — сжатое руководство для кодовых агентов,
[CHANGELOG.md](CHANGELOG.md) — что менялось, и [MIGRATION-1.3.md](MIGRATION-1.3.md) — переход с 1.x.

## Лицензия

MIT — см. [LICENSE](LICENSE).
