# Oblodai Rust SDK

Официальный Rust SDK для платёжного шлюза **Oblodai**: приём платежей, выплаты, массовые операции
(пачки), платёжные и payout-ссылки, сплит-платежи, счета на e-mail, статические кошельки, вебхуки,
песочница разработчика.
Автоподпись запросов, разбор ответов в типизированные структуры, обработка ошибок и автоматические
повторы с защитой от дублей (`Idempotency-Key`).

> **Клиент БЛОКИРУЮЩИЙ.** Транспорт — `reqwest::blocking`, паузы между повторами —
> `std::thread::sleep`. Каждый вызов блокирует поток, из которого сделан. **Async-приложениям
> (tokio, async-std) нельзя вызывать SDK напрямую из задачи** — заблокированный поток исполнителя
> останавливает и все прочие задачи на нём. Уносите вызов на блокирующий пул: см.
> [Async-приложения](#async-приложения-tokio).

> **Базовый URL — только HTTPS.** По умолчанию `https://api.oblodai.com`; переопределяется через
> `Config::base_url(...)`. `http://` на внешнем хосте отвергается ошибкой (SDK шлёт `X-Public-Id` и
> подпись `X-Signature` — открытым текстом их видит любой посредник). Исключение — loopback
> (`localhost`, `127.0.0.1`, `::1`) для локальных стендов.

## Установка

```toml
[dependencies]
oblodai = "1"
serde_json = "1"
```

Требуется Rust 1.75+.

## Где взять ключи

Ключи выдаются в кабинете Oblodai — [oblodai.com](https://oblodai.com), раздел API-ключей. Пара
состоит из двух значений:

| Значение | Что это | Переменная окружения |
| --- | --- | --- |
| `public_id` | Несекретный идентификатор ключа, уходит в заголовке `X-Public-Id` | `OBLODAI_PUBLIC_ID` |
| Секрет | Подписывает исходящие запросы (`X-Signature`), в запрос сам никогда не уходит | `OBLODAI_SECRET` |

Важное:

- **Секрет показывается один раз — в момент создания ключа.** Сохраните его сразу; если потеряли,
  ключ не «подсматривают», а выпускают заново.
- **Для песочницы заводится отдельный тестовый ключ:** `public_id` вида `test_…`, секрет вида
  `oblodai_test_…`. Боевые — без этих префиксов. Проверить, какой ключ в руках, можно хелпером
  `oblodai::is_test_key(public_id)`.
- Секрет API-ключа — **не** секрет вебхуков. Второй отдельный и возвращается при регистрации
  эндпоинта; см. [Проверка вебхуков](#проверка-вебхуков).
- Секрет не место в репозитории: держите его в переменных окружения или менеджере секретов.

## Учётные данные

Храните ключи в переменных окружения (см. `.env.example`):

```bash
export OBLODAI_PUBLIC_ID=test_...
export OBLODAI_SECRET=oblodai_test_...
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
        Config::new("test_...", "oblodai_test_...")
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
    println!("{}", payment.url);     // hosted-страница оплаты (пустая, если у шлюза не задан
                                     // публичный базовый URL — см. «Замечания»)
    Ok(())
}
```

**Тот же код работает с боевым ключом — меняется только ключ.** Здесь подставлен тестовый
(`test_…` / `oblodai_test_…`), чтобы начать в песочнице; бизнес-методы, пути и модели у боевого
ключа те же самые.

### Async-приложения (tokio)

Клиент блокирующий. В async-рантайме уносите вызовы на блокирующий пул, иначе запрос (до 30 секунд
по умолчанию, плюс задержки повторов) встанет колом на потоке исполнителя:

```rust
use oblodai::{Client, Config};
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Клиент Send + Sync — создаётся один раз и переиспользуется.
    let client = Arc::new(Client::from_env()?);

    let c = Arc::clone(&client);
    let payment = tokio::task::spawn_blocking(move || {
        c.payments().create(json!({
            "amount": "10", "currency": "USD", "order_id": "order-1",
            "to_currency": "USDT", "network": "tron",
        }))
    })
    .await??; // первый `?` — паника/отмена задачи, второй — ошибка SDK

    println!("{}", payment.address);
    Ok(())
}
```

Асинхронного (`async fn`) варианта клиента в SDK нет — это осознанная граница пакета.

### Таймаут

Таймаут ОДНОЙ HTTP-попытки настраивается через `Config::timeout` (по умолчанию 30 секунд):

```rust
use std::time::Duration;

let client = Client::new(
    Config::new("test_...", "oblodai_test_...")
        .timeout(Duration::from_secs(10)),
)?;
```

Это таймаут попытки, а не всего вызова: при включённых повторах верхняя граница ожидания —
`timeout × max_attempts` плюс задержки backoff. Значение применяет встроенный reqwest-транспорт;
свой `HttpTransport` (см. [Свой HTTP-транспорт](#свой-http-транспорт)) трактует его сам.

## Песочница / тестирование

**Интеграционный код не меняется между тестом и продом — меняется только ключ.** Все бизнес-методы
SDK работают с тестовым ключом идентично боевому: тестовый `public_id` начинается с `test_`,
тестовый секрет — с `oblodai_test_`. Хелпер `oblodai::is_test_key(public_id)` вернёт `true` для
тестового ключа.

Новое — пять вспомогательных методов `client.sandbox()`. Боевого аналога у них нет: они заменяют
«клиент заплатил он-чейн». Боевой ключ на них получает 403 `sandbox.live_key`. **Вызовы песочницы —
строго ТЕСТОВЫЙ код**; не вплетайте их в боевую интеграцию.

```rust
use oblodai::{Client, Config};
use serde_json::json;

fn main() -> oblodai::Result<()> {
    // Тестовый ключ — тот же конструктор, тот же код.
    let client = Client::new(Config::new("test_...", "oblodai_test_..."))?;

    // 1. Создаём инвойс обычным бизнес-методом.
    let payment = client.payments().create(json!({
        "amount": "10", "currency": "USD", "order_id": "order-1",
        "to_currency": "USDT", "network": "tron",
    }))?;

    // 2. Симулируем он-чейн оплату (без полей — ровно причитающееся, сразу подтверждено).
    client.sandbox().simulate_deposit(&payment.uuid, json!({}))?;

    // 3. Опрашиваем инвойс как в проде — он станет paid.
    let paid = client.payments().info(Some(&payment.uuid), None)?;
    println!("{}", paid.payment_status);

    // 4. Баланс «из воздуха» — и обычная выплата с него.
    client.sandbox().faucet("USDT", "1000", Some("seed-1"))?;
    client.payouts().create(json!({
        "amount": "25", "currency": "USDT", "network": "tron",
        "address": "T...", "order_id": "w-1",
    }))?;
    Ok(())
}
```

Сценарии посложнее:

```rust
// Недоплата, «висящая» на 1 подтверждении...
client.sandbox().simulate_deposit(&payment.uuid, json!({
    "amount": "5", "confirmations": 1, "txid": "tx-a",
}))?;
// ...повтор с тем же txid и бОльшим confirmations углубляет ТОТ ЖЕ депозит (идемпотентно).
client.sandbox().simulate_deposit(&payment.uuid, json!({
    "amount": "5", "confirmations": 12, "txid": "tx-a",
}))?;

client.sandbox().reset()?;                      // обнулить балансы + отменить неоплаченные инвойсы
let deliveries = client.sandbox().list_webhooks()?; // до 50 доставок, новые первыми, с payload
client.sandbox().replay_webhook(&deliveries[0].id)?; // поставить доставку на повтор
```

Нюансы:

- **Неглубокие подтверждения САМИ не дозревают.** Симулированный депозит не переэмитится и цепочки
  за ним нет, поэтому инвойс с `confirmations` меньше требуемого висит неограниченно долго — в
  `confirm_check` (депозит увиден, ждём подтверждений), а если прислали меньше суммы — в
  `wrong_amount_waiting`. **Не `check`:** `check` означает, что оплаты не видели вовсе.
  Единственный способ довести инвойс до `paid` — повторить `simulate_deposit` с **тем же
  `txid`** и бОльшим `confirmations` (это углубляет ТОТ ЖЕ депозит, а не создаёт новый).
- **~10 минут — это про другое:** про maturity-холд на **выплате**. Средства с только что
  зачисленного депозита нельзя вывести сразу — попытка даёт `payout.funds_maturing`. В песочнице
  такой холд снимается по возрасту (дефолт 10 минут, `GATEWAY_SANDBOX_MATURITY_MINUTES` на стороне
  шлюза) — либо сразу, если повторить тот же `txid` с большой глубиной. К подтверждениям инвойса
  этот механизм отношения не имеет.
- **`reset` — это НЕ «чистый лист».** Обнуляются балансы, но отменяются только инвойсы в статусах
  `check` (внутренне `created`) и `select` — те, где оплаты ещё не видели. Инвойс с уже
  прошедшим депозитом (`confirm_check`, `wrong_amount_waiting`) **сознательно остаётся жить**:
  отмена дала бы депозиту дозреть в отменённый счёт. Нужен действительно чистый прогон — заводите
  новые инвойсы, а не рассчитывайте, что `reset` уберёт все прежние.
- **UTXO-сети (Bitcoin и т.п.):** нет авто-возврата переплаты и нет адреса плательщика — поведение
  идентично проду.
- `faucet`: потолок 1000000 за вызов; `idempotency_key` у него — поле **тела** запроса (не заголовок
  `Idempotency-Key`, в отличие от создающих бизнес-методов).
- `list_webhooks` — единственный **подписанный GET**: подпись считается от той же канонической
  строки с пустым телом (`{ts}\nGET\n/v1/sandbox/webhooks\n`).

## Проверка вебхуков

Подпись вебхука отличается от подписи запроса — SDK делает и то, и другое. Для входящих вебхуков
берите **сырое тело** и заголовки `X-Webhook-Timestamp` / `X-Webhook-Signature`.

> ### ⚠ Секретов ДВА, и они разные
>
> | Что | Откуда берётся | Для чего |
> |---|---|---|
> | **Секрет API-ключа** (`OBLODAI_SECRET`, `Config::secret`) | из кабинета вместе с `public_id` | подписывает **исходящие** запросы SDK |
> | **Секрет эндпоинта** (`endpoint_secret`) | поле `secret` в ответе `client.webhooks().register(url)` | проверяет **входящие** вебхуки |
>
> Везде в этом README под «секретом» без уточнения имеется в виду ПЕРВЫЙ — ключ API. В
> `verify_webhook` / `construct_event` нужен ВТОРОЙ. Подставив ключ API, вы отвергнете 100%
> вебхуков с `Error::Signature`. Секрет эндпоинта выдаётся при первой регистрации и переживает
> смену URL — сохраните его тогда же.

```rust
use oblodai::{construct_event, verify_webhook, VerifyOptions, WebhookHeaders, Error};
use serde_json::Value;

// endpoint_secret — НЕ ключ API: это `secret` из client.webhooks().register(url).
fn handle(endpoint_secret: &str, raw_body: &[u8], ts: &str, sig: &str) {
    let headers = WebhookHeaders { timestamp: ts, signature: sig };

    // По умолчанию проверяется подпись И свежесть (replay-защита, окно 5 минут).
    match construct_event::<Value>(endpoint_secret, raw_body, &headers, &VerifyOptions::default()) {
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

Секрет эндпоинта берётся один раз при регистрации и кладётся в ваш конфиг/хранилище:

```rust
let reg = client.webhooks().register("https://shop.example/hooks/oblodai")?;
// reg.secret — сохраните ЭТО значение; проверять вебхуки нужно им.
```

Чтобы отключить replay-защиту: `VerifyOptions { max_age_seconds: 0, now: None }`.

### Регистрация — это upsert ЕДИНСТВЕННОГО эндпоинта

У проекта ровно **один** эндпоинт вебхуков. Повторный `register()` с ДРУГИМ URL не добавляет второй
приёмник, а **перенаправляет** доставки: возвращается тот же `endpoint_id`, старый URL молча
замолкает. Секрет при этом сохраняется (уже поставленные в очередь доставки подписаны им; перевыпуск
оборвал бы их). Нужен веер на несколько потребителей — принимайте вебхук одним своим приёмником и
разводите дальше сами.

### Статус в теле вебхука

Поле `status` события платежа — из той же таблицы, что и `payment_status`, но **без**
`wrong_amount_waiting`: вебхуки шлют неуточнённый статус, поэтому частичная оплата приезжает как
`confirm_check`. Уточнённое значение отдаёт только `payments().info(...)`.

## Статусы

Статусы приходят строками (`payment.payment_status`, `payout.status`), но в SDK для них есть
перечисления — [`PaymentStatus`], [`PayoutStatus`], [`PayoutLinkStatus`]. Поля моделей намеренно
остались `String` (новое значение на стороне шлюза не сломает разбор ответа), а типизированный
разбор доступен через методы:

```rust
use oblodai::PaymentStatus;

let payment = client.payments().info(Some(&uuid), None)?;
match payment.status() {                       // -> PaymentStatus
    PaymentStatus::Paid | PaymentStatus::PaidOver => { /* отгружаем заказ */ }
    PaymentStatus::WrongAmount => { /* закрылся недоплаченным: можно resolve */ }
    PaymentStatus::Cancel => { /* истёк или отменён */ }
    _ => { /* ещё в работе — продолжаем поллинг */ }
}

// Незнакомое значение -> PaymentStatus::Unknown, разбор ответа при этом не падает.
assert_eq!(PaymentStatus::from_api("confirm_check"), PaymentStatus::ConfirmCheck);
assert_eq!(PaymentStatus::Paid.as_str(), "paid");
```

### Статусы платежа

| Значение | `PaymentStatus` | Что значит | Терминальный |
|---|---|---|---|
| `check` | `Check` | Счёт создан, оплаты ещё не видели. | нет |
| `confirm_check` | `ConfirmCheck` | Оплата увидена в сети, ждём подтверждений. | нет |
| `wrong_amount_waiting` | `WrongAmountWaiting` | Увидена **частичная** оплата, ждём доплату. | **нет** |
| `wrong_amount` | `WrongAmount` | Счёт закрылся недоплаченным. | да |
| `paid` | `Paid` | Оплачен полностью. | да |
| `paid_over` | `PaidOver` | Переплачен (излишек — в авто-возврат, если он включён и сеть его поддерживает). | да |
| `cancel` | `Cancel` | Истёк или отменён. | да |
| `select` | `Select` | Валюто-агностичный счёт: покупатель ещё не выбрал валюту и сеть. | нет |

Терминальность дублируется полем `is_final` в ответе — на незнакомых значениях опирайтесь на него,
а не на `Unknown`.

> **`wrong_amount_waiting` ≠ `wrong_amount`.** Первый — ЖИВОЙ счёт: плательщик прислал меньше,
> но ещё может добить сумму, и счёт станет `paid`. Второй — уже закрытый недоплаченным.
> `payments().resolve(...)` работает **только** со вторым: на `wrong_amount_waiting` шлюз отвечает
> `409 resolution.not_underpaid`. Проверить можно через `payment.status().is_resolvable()`.

### Статусы выплаты

| Значение | `PayoutStatus` | Что значит | Терминальный |
|---|---|---|---|
| `check` | `Check` | Создана, ждёт одобрения (`payouts().approve`). | нет |
| `process` | `Process` | Одобрена: транзакция формируется, вещается или уже ушла в сеть. | нет |
| `paid` | `Paid` | Подтверждена в сети. | да |
| `fail` | `Fail` | Не удалась. | да |
| `cancel` | `Cancel` | Отменена. | да |

Тот же словарь у статуса рефанд-выплаты в ответе `resolve` (`Resolution::payout_status()`) и у
элементов массовой выплаты (`MassPayoutItem::status()`).

> **Исключение:** `PaymentLinkPayment::status` (список платежей по платёжной ссылке) отдаёт
> **внутренние** литералы инвойса — `created`, `expired`, `cancelled` вместо `check` и `cancel`.
> За каноническим статусом ходите в `payments().info(uuid, None)`.

## Обработка ошибок

Все ошибки — enum [`Error`]. Ошибки API несут машиночитаемый код.

```rust
use oblodai::Error;

match client.payouts().create(params) {
    Ok(payout) => { /* ... */ }
    Err(Error::Api { code, status, message, .. }) => {
        match code.as_str() {
            "payout.insufficient_funds" => { /* недостаточно средств */ }
            // Средства ещё дозревают. Ошибка ТЕРМИНАЛЬНА: is_retriable() == false, SDK её не
            // повторяет — немедленный повтор не поможет. Подождите и вызовите заново сами.
            "payout.funds_maturing"    => { /* ... */ }
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
немедленный повтор не поможет) считаются терминальными и не повторяются. Из идемпотентных кодов это
означает: `503 idempotency.unavailable` SDK переиграет сам (с тем же ключом), а `409
idempotency.in_progress`, `409 payoutlink.duplicate_reference`, `400 idempotency.key_reused`
и `400 idempotency.bad_key` вернутся вам как есть. Заголовок `Retry-After`
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
> заголовку `Idempotency-Key` (см. ниже): если операция уже создана — сервер вернёт её же, дубля
> не будет.

## Идемпотентность (v1.1.0, ломающее изменение)

Создающие вызовы (`payments().create/refund/resolve/create_batch/refund_batch`,
`payouts().create/create_mass/create_batch/refund`, `account().transfer_to_personal`,
`account().transfer_to_user/transfer_batch`) шлют заголовок
**`Idempotency-Key`** — UUID v4, сгенерированный **один раз до повторов**: все внутренние ретраи
одного вызова уходят с одним и тем же ключом, поэтому таймаут+повтор не создаёт дубль. Заголовок в
подпись запроса не входит.

- **`order_id` больше НЕ подставляется автоматически** (поведение v1.0.x удалено). Он уходит как
  есть — это ваш бизнес-идентификатор, задавайте его явно, чтобы потом находить операцию через
  `info`. Для выплат `order_id` обязателен всегда (`payout.order_id_required`).
- **Свой ключ идемпотентности** (дедупликация между вызовами/процессами): передайте поле
  `idempotency_key` в параметрах создающего вызова — оно уйдёт в заголовок и **не попадёт в тело**:

```rust
client.payments().create(json!({
    "amount": "10", "currency": "USD", "order_id": "ord-1",
    "idempotency_key": "3f8a2c1e-...-ваш-uuid",
}))?;
```

- **`payout_links().create/create_batch`** тоже шлют заголовок: обе операции резервируют баланс, и
  без него потерянный ответ + авто-повтор профинансировали бы вторую ссылку. Повтор с тем же ключом
  реплеит первый ответ (та же ссылка, тот же `claim_token`), и баланс дебетуется ровно один раз;
  ответ помечается заголовком `Idempotent-Replayed: true`. **Без** заголовка (например, если вы
  вызываете API мимо SDK) два одинаковых вызова создадут ДВЕ ссылки. Дополнительно
  дедуплицируйте создание через per-link `reference`. Нюанс пачки: частично неудачная пачка
  реплеится как есть — упавшие элементы под тем же ключом не переотправятся, шлите их новым вызовом.

### Коды ответов идемпотентного слоя

Идемпотентным маршрутам (в т.ч. `/v1/payout/link` и `/v1/payout/link/batch`) шлюз может ответить:

| Код | HTTP | Что значит | Повторяется SDK? |
|---|---|---|---|
| `idempotency.key_reused` | 400 | Тот же ключ с ДРУГИМ телом. | нет (терминальная) |
| `idempotency.bad_key` | 400 | Некорректный ключ (напр. длиннее 255 символов). | нет (терминальная) |
| `idempotency.in_progress` | 409 | Параллельный повтор, пока первый запрос ещё выполняется. | нет — повторите сами позже |
| `idempotency.unavailable` | 503 | Стор идемпотентности недоступен; шлюз fail-closed и НЕ выполняет операцию. | да, автоматически |

Классификация ровно как у обычных ошибок: `4xx` терминальны (`is_retriable() == false`), `5xx`
повторяются. То есть 503 SDK переиграет сам с ТЕМ ЖЕ ключом, а на 409 `idempotency.in_progress`
вернёт ошибку вам — подождите и повторите вызов, передав тот же `idempotency_key` явно.

> **Батчи и лимит кэша.** Ответ больше 256 КБ шлюз не кэширует — повтор с тем же ключом тогда
> выполнится ЗАНОВО. Это реально достижимо на больших пачках payout-ссылок, поэтому проставляйте
> per-item `reference`: уникальный индекс на стороне шлюза остаётся вторым, durable слоем защиты и
> работает даже без заголовка и даже когда ответ не влез в кэш. Дубль `reference` возвращает
> `409 payoutlink.duplicate_reference` (раньше был `500`, который SDK повторял; теперь — терминальный
> конфликт, повтора не будет).
- **`wallets().blocked_address_refund` — исключение:** заголовок не шлётся и не нужен. Возврат
  идемпотентен по состоянию: сервер выводит ссылку выплаты из id кошелька и ищет её под per-wallet
  блокировкой, поэтому повтор отдаёт ТУ ЖЕ выплату. Это сильнее заголовка — параллельные повторы
  сериализуются, а не получают конфликт.
- **`payouts().approve` — исключение:** это переход статуса, а не создание. Повторный approve уже
  одобренной выплаты отвечает `409 payout.not_pending`; читайте это как «уже одобрено» и сверяйтесь
  через `payouts().info(...)`.

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

## Переводы пользователям (v1.2.0)

Внутренний перевод **без комиссии** с баланса мерчанта на личный кошелёк пользователя платформы
(payout-ключ). `to_user_id` — id пользователя (**UUID, не username**): username резолвится в id
через публичный профиль кабинета. Идемпотентность — как у остальных денежных методов: заголовок
`Idempotency-Key` (свой ключ — поле `idempotency_key`); на бэкенде лестница
«заголовок → `order_id` → подпись».

```rust
let res = client.account().transfer_to_user(json!({
    "to_user_id": "5c3f8a2c-9b1d-4e6f-8a2c-1e9b7d5f3a10",
    "amount": "25", "currency": "USDT", "order_id": "salary-7",
}))?;
println!("{}", res.recipient_balance);

// Пачка (payroll, до 5000): результаты — через batches().info(batch_id, ...)
let sub = client.account().transfer_batch(vec![
    json!({ "to_user_id": "…", "amount": "25", "currency": "USDT", "order_id": "s-1" }),
    json!({ "to_user_id": "…", "amount": "30", "currency": "USDT", "order_id": "s-2" }),
], Some("continue"))?;
println!("{}", sub.batch_id);
```

## Публичный pay для своей checkout-страницы (v1.2.0)

`GET /v1/pay/{id}` и `POST /v1/pay/{id}/select` — публичные (без подписи), как `/v1/link/{id}` и
`/v1/claim/{token}`: их можно дёргать прямо со страницы оплаты, секрет не нужен. `public_get`
возвращает клиентские поля инвойса — адрес, сумму, QR, статус, срок (приватные `additional_data` /
`payer_email` бэкенд наружу не отдаёт); для валюто-агностичного счёта в статусе `select` — ещё и
список методов на выбор. `public_select` фиксирует валюту+сеть, курс и депозит-адрес.

```rust
let state = client.payments().public_get(&payment.uuid)?;   // статус для поллинга
if state.payment_status == "select" {
    let finalized = client.payments().public_select(&payment.uuid, "USDT", "tron")?;
    println!("{}", finalized.address);
}
```

## Обзор методов

> **Имена ресурсов едины во всех SDK Oblodai.** Канон для платёжных ссылок — `payment_links`
> (`paymentLinks` в JS/PHP, `PaymentLinks` в Go). Короткое имя `links` оставлено как
> **документированный алиас**: `client.links()` — тот же ресурс, что `client.payment_links()`, с
> теми же методами. Оба имени поддерживаются, ничего не устарело; алиас существует, чтобы код
> переносился между языками без переименований.

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
client.payments().create_batch(payments, on_error)      // пачка платежей (до 5000)
client.payments().refund_batch(refunds, on_error)       // пачка возвратов
client.payments().send_email(uuid, order_id, email)     // счёт на e-mail
client.payments().resolve(ResolveAction::Accept, params) // судьба недоплаты (ТОЛЬКО wrong_amount)
client.payments().public_get(uuid)                      // публично, без подписи (свой checkout)
client.payments().public_select(uuid, currency, network) // публично: выбор валюты+сети

// Выплаты
client.payouts().create(params)
client.payouts().create_mass(payouts, source)
client.payouts().create_batch(payouts, on_error)        // пачка выплат (до 5000)
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
client.account().transfer_to_user(params)               // перевод пользователю (без комиссии)
client.account().transfer_batch(transfers, on_error)    // пачка переводов (до 5000)
client.account().vrcs(enabled)

// Вебхуки
client.webhooks().register(url)                         // upsert ЕДИНСТВЕННОГО эндпоинта проекта
client.webhooks().deliveries()
client.webhooks().test_payment(params)

// Настройки
client.settings().list_auto_withdraw() / set_auto_withdraw(params) / delete_auto_withdraw(currency)
client.settings().list_allowlist() / add_allowlist(cidr) / remove_allowlist(cidr) / enable_allowlist(bool)

// Массовые операции: состояние пачки
client.batches().info(batch_id, limit, offset)

// Платёжные ссылки (переиспользуемые, «донатные»); client.links() — алиас того же ресурса
client.payment_links().create(params)
client.payment_links().list(limit, offset) / info(link_id) / toggle(link_id, active)
client.payment_links().public_get(link_id)              // публично, без подписи
client.payment_links().checkout(link_id, params)        // публично, без подписи

// Сплит-платежи
client.splits().create_rule(params)                     // {address,network} ИЛИ {merchant_id} + percent
client.splits().list_rules() / delete_rule(rule_id)
client.splits().get_config() / set_config(refund_hold_hours)

// Payout-ссылки («крипто-чеки», payout-ключ)
client.payout_links().create(params)                    // задавайте expires_in_hours явно!
client.payout_links().create_batch(links)               // до 500 ссылок
client.payout_links().list(limit, offset) / info(link_id) / cancel(link_id)
client.payout_links().claim_info(token)                 // публично, без подписи
client.payout_links().claim(token, address, memo)       // публично, без подписи

// Курсы (публично, без ключа)
client.rates().list(Some("ETH"))

// Песочница (ТОЛЬКО тестовый ключ; боевой получит 403 sandbox.live_key)
client.sandbox().simulate_deposit(invoice_id, params) // симуляция он-чейн депозита
client.sandbox().faucet(asset, amount, idem)          // тестовый баланс (потолок 1000000)
client.sandbox().reset()                              // обнулить балансы + отменить неоплаченные
client.sandbox().list_webhooks()                      // до 50 доставок (подписанный GET)
client.sandbox().replay_webhook(delivery_id)          // поставить доставку на повтор
```

### Payout-ссылки: коротко

Вы резервируете средства в ссылку, **не зная кошелька получателя**; получатель открывает публичную
страницу `claim_url`, вводит адрес — порождается обычная выплата. `claim_token`/`claim_url`
возвращаются **только в ответе `create`** — сохраните их сразу. **Задавайте `expires_in_hours`
(1–720) явно**: при 0/отсутствии бэкенд клампит срок к **1 часу**. Непорученная ссылка возвращает
резерв по истечении срока или при `cancel`.

```rust
let link = client.payout_links().create(json!({
    "currency": "USDT", "network": "tron", "amount": "25",
    "reference": "bonus-42",            // ваш ключ дедупликации; дубль → 409 payoutlink.duplicate_reference
    "expires_in_hours": 168,            // 7 дней; без этого поля — всего 1 час!
    "email": "user@example.com",        // опционально: claim-письмо получателю
}))?;
println!("{}", link.claim_url); // отдайте получателю
```

> **`claim_url` и `payment.url` может прийти ПУСТОЙ строкой.** Шлюз собирает их из своего публичного
> базового URL (`GATEWAY_PUBLIC_BASE_URL`). В проде без него шлюз не стартует, но на локальном или
> тестовом стенде его часто не задают — и обе ссылки приезжают пустыми. Это не баг SDK и не баг
> шлюза. Если тестируете локально, стройте ссылку сами из токена/идентификатора:
> `{ваш_базовый_url}/claim/{claim_token}` и `{ваш_базовый_url}/pay/{uuid}`.

## Замечания

- **Суммы — строки** в единицах валюты (`"25.00"`), не числа. Так сохраняется точность.
- **Дубли исключает заголовок `Idempotency-Key`** (см. раздел «Идемпотентность»). `order_id` —
  ваш бизнес-идентификатор для поиска операции, задавайте его явно; SDK его не подставляет.
- **Секрет — только на сервере.** SDK серверный; не встраивайте ключ в клиентские приложения.
- **Секретов два.** Ключ API подписывает исходящие запросы; вебхуки проверяются **секретом
  эндпоинта** из `webhooks().register(...).secret` — см. «Проверка вебхуков».
- **Статусы** типизированы перечислениями `PaymentStatus` / `PayoutStatus` / `PayoutLinkStatus`
  (поля остаются строками) — см. «Статусы».
- **`payment.url` и `claim_url`** собираются шлюзом из его публичного базового URL и на стенде без
  него приходят пустыми: стройте ссылку сами из `uuid` / `claim_token`.
- **Тела запросов** принимаются как `serde_json::Value` — стройте макросом `json!`.

## Лицензия

MIT
