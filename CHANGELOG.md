# Changelog

## 1.3.0 — 2026-08-25

Rewrite generated from the gateway's contract snapshot. See MIGRATION-1.3.md.

- **Fixed:** requests are signed with the five-field recipe over path+query
  (`ts \n METHOD \n path+query \n Idempotency-Key \n body`). The 1.x line signed four fields, which
  the gateway stopped accepting — every call returned 401.
- **Fixed:** models, statuses, pagination and parameter names match the current API vocabulary, and
  are checked field by field against golden response bodies recorded from a live gateway.
- **Added:** every merchant route (107) — cancel/validate, batches, documents, fee configs, split
  opt-in, secret rotation, payer-facing checkout and claim endpoints.
- **Added:** `Pager` (await one page, `.stream()` every item, `.all(max)`), authoritative
  `retryable`-driven retries with a safe-to-repeat rule, automatic idempotency keys, clock-skew
  correction, dual key pairs, a per-call deadline.
- **Added:** `oblodai::webhooks` — rotation-aware `verify_webhook`, `verify_webhook_delivery`,
  `parse_webhook`, `is_stale_event`; no client and no API key needed.
- **Added:** a `blocking` feature: the same method tree, the same pure core, synchronous I/O.
- **Added:** `HttpBackend` / `BlockingHttpBackend` so the HTTP layer can be replaced (a proxy-aware
  client, a recording stub in tests).
- **Added:** contract tests against the golden bodies and real signed webhook deliveries,
  `python3 scripts/codegen.py --check` as a drift gate, and a live journey against a real gateway.
- **Changed:** async by default on `reqwest` + `tokio` with rustls (no OpenSSL); MSRV 1.75.
- **Changed:** every method returns a builder that is also a future — per-call `idempotency_key`,
  `timeout`, `deadline`, `prefer_payout_key` instead of client-wide settings.
- **Changed:** `Error` is one type with `code`, `http_status`, `retryable`, `retry_after`,
  `request_id`, `field`, `synthetic` and a `kind()`; the raw body is never printed or serialized.

Значимые изменения этого пакета. Формат — [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/),
версии — [SemVer](https://semver.org/lang/ru/).

## [1.2.0] — 2026-07-19

### Безопасность
- **Базовый URL обязан быть `https://`.** Раньше SDK молча принимал `http://` и отправлял по
  открытому каналу `X-Public-Id`, `X-Timestamp` и подпись `X-Signature` — их видел любой посредник,
  а подпись пригодна для повтора запроса. Теперь `Client::new` / `Client::with_transport`
  (а значит, и `Client::from_env`, и `OBLODAI_BASE_URL`) возвращают `Error::Config` с внятным
  текстом. **Исключение — loopback:** `localhost`, любой адрес из `127.0.0.0/8` и `::1` принимаются
  и по `http://`, чтобы не ломать локальные стенды (в т.ч. `http://localhost:8095`). Схемы, отличные
  от `http`/`https`, и URL без схемы тоже отвергаются.

### Изменено
- **README перекомпонован:** раздел «Песочница / тестирование» поднят сразу после быстрого старта
  (был закопан за идемпотентностью и своим транспортом — читающий сверху вниз подключал боевые
  ключи раньше, чем узнавал о безопасной площадке). В быстром старте плейсхолдер секрета заменён с
  `oblodai_live_…` на тестовый `oblodai_test_…` с явной оговоркой, что тот же код работает с боевым
  ключом и меняется только ключ.

### Добавлено
- **Настраиваемый таймаут.** Поле `Config::timeout` (билдер `Config::timeout(Duration)`, дефолт —
  30 секунд, константа `oblodai::DEFAULT_TIMEOUT`). Раньше 30 секунд были зашиты в
  `ReqwestTransport` и не настраивались. Это таймаут ОДНОЙ HTTP-попытки, а не всего вызова: при
  включённых повторах верхняя граница ожидания — `timeout × max_attempts` плюс задержки backoff.
  Сопутствующее: `ReqwestTransport::with_timeout(Duration)` и геттеры `ReqwestTransport::timeout()`
  / `Client::timeout()`.
- **Алиас `client.links()`** — то же самое, что `client.payment_links()` (тот же ресурс, те же
  методы). Канон в SDK Oblodai — `payment_links` (`paymentLinks` в JS/PHP, `PaymentLinks` в Go);
  короткое имя `links` документировано как алиас, чтобы код переносился между языками без
  переименований. Ничего не удалено и не помечено устаревшим.
- **README: раздел «Где взять ключи»** — первым блоком после установки: ключи выдаются в кабинете
  Oblodai (https://oblodai.com) в разделе API-ключей, секрет показывается один раз, для песочницы
  выдаётся отдельный тестовый ключ (`test_…` / `oblodai_test_…`).
- **README: раздел про блокирующую природу клиента** — в первом абзаце, плюс подраздел
  «Async-приложения (tokio)» с рецептом `tokio::task::spawn_blocking` и подраздел «Таймаут».
  То же предупреждение — в rustdoc `Client` и в доке крейта. Раньше слова «блокирующий», «async»
  и «tokio» в README не встречались вообще, хотя транспорт — `reqwest::blocking`, а паузы повторов —
  `std::thread::sleep`.
- **Типизированные статусы.** Перечисления `PaymentStatus` (`check`, `confirm_check`,
  `wrong_amount_waiting`, `wrong_amount`, `paid`, `paid_over`, `cancel`, `select`) и `PayoutStatus`
  (`check`, `process`, `paid`, `fail`, `cancel`) — с `from_api`/`as_str`/`is_final`, а у платежа ещё
  и `is_resolvable()`. Незнакомое значение даёт вариант `Unknown`, поэтому новое значение на стороне
  шлюза разбор ответа не сломает. Поля моделей намеренно остаются `String` (изменение
  неломающее); типизированный разбор — через `Payment::status()`, `Payout::status()`,
  `MassPayoutItem::status()`, `Resolution::payout_status()`.
- **Песочница разработчика:** ресурс `client.sandbox()` — тестовые методы, доступные ТОЛЬКО
  тестовому ключу (`public_id` с префиксом `test_`, секрет — `oblodai_test_`); боевой ключ
  получает 403 `sandbox.live_key`. Бизнес-методы с тестовым ключом работают без изменений —
  меняется только ключ.
  - `sandbox().simulate_deposit(invoice_id, params)` — симуляция он-чейн депозита в инвойс
    (`amount` — недоплата/переплата, `confirmations` — «висящий» депозит, повтор с тем же `txid` —
    идемпотентность/углубление подтверждений). Тип `SandboxDeposit`.
  - `sandbox().faucet(asset, amount, idempotency_key)` — тестовый баланс «из воздуха»
    (потолок 1000000 за вызов; `idempotency_key` — поле тела, не заголовок). Тип `SandboxFaucet`.
  - `sandbox().reset()` — отменить открытые инвойсы и обнулить балансы. Тип `SandboxReset`.
  - `sandbox().list_webhooks()` — недавние доставки вебхуков (до 50, новые первыми) с сырым
    `payload`. Тип `SandboxWebhookDelivery`.
  - `sandbox().replay_webhook(delivery_id)` — поставить доставку на повтор. Тип `SandboxReplay`.
- **Подписанный GET.** `GET /v1/sandbox/webhooks` подписывается той же канонической строкой,
  что и POST, с пустым телом: `{ts}\nGET\n{path}\n`.
- **Хелпер `oblodai::is_test_key(public_id)`** — `true` для тестовых ключей
  (`test_...` / `oblodai_test_...`).
- **Переводы пользователям:** `account().transfer_to_user(params)` — внутренний перевод без
  комиссии с баланса мерчанта на личный кошелёк пользователя платформы (`to_user_id` — **UUID
  пользователя, не username**). Идемпотентность — как у остальных денежных методов: заголовок
  `Idempotency-Key` (свой ключ — поле `idempotency_key`), на бэкенде лестница
  «заголовок → `order_id` → подпись». Тип `UserTransfer`.
- **Пачка переводов:** `account().transfer_batch(transfers, on_error)` (до 5000,
  `POST /v1/transfer/batch`) → `batch_id`; результаты — через `batches().info(...)`.
- **Публичный pay для своего checkout:** `payments().public_get(uuid)` (`GET /v1/pay/{id}`) —
  публичное состояние счёта, и `payments().public_select(uuid, currency, network)`
  (`POST /v1/pay/{id}/select`) — выбор валюты+сети валюто-агностичного счёта. Оба без подписи,
  как `/v1/link/{id}`; возвращают модель `Payment`.

### Исправлено
- **БЛОКЕР документации: вебхуки проверялись «секретом» из ключа API.** Пример в README (и параметры
  `verify_webhook` / `construct_event` / `compute_webhook_signature`) назывались просто `secret`,
  а «секрет» во всём README означает секрет API-ключа. Проверять вебхуки нужно **секретом
  эндпоинта** — полем `secret` из ответа `webhooks().register(url)`; это отдельное значение, и с
  ключом API отвергаются 100% вебхуков (`Error::Signature`). Параметры переименованы в
  `endpoint_secret`, добавлена таблица «секретов два» и пример получения секрета при регистрации.
- **`register()` — это upsert ЕДИНСТВЕННОГО эндпоинта проекта**, а не «добавить ещё один приёмник».
  Повторный вызов с ДРУГИМ URL возвращает тот же `endpoint_id` и **перенаправляет** доставки —
  старый URL молча замолкает; секрет при этом сохраняется (уже поставленные в очередь доставки
  подписаны им). Предупреждение добавлено в README и rustdoc.
- **Словарь статусов задокументирован полностью**, включая ранее не описанные `confirm_check`,
  `wrong_amount_waiting`, `paid_over` и `select`. Отдельно разведены `wrong_amount_waiting`
  (частичная оплата, счёт ЖИВ, плательщик ещё может добить) и `wrong_amount` (счёт закрылся
  недоплаченным). `payments().resolve(...)` работает только со вторым: на `wrong_amount_waiting`
  шлюз отвечает `409 resolution.not_underpaid`.
- **Песочница: «висящий» инвойс описывался как `check`.** На деле недобор подтверждений — это
  `confirm_check` (депозит увиден, ждём подтверждений) либо `wrong_amount_waiting` при неполной
  сумме; `check` означает, что оплаты не видели вовсе.
- **`sandbox().reset()` — не «чистый лист».** Балансы обнуляются, но отменяются только инвойсы в
  статусах `check` (внутренне `created`) и `select`. Инвойс с уже прошедшим депозитом
  (`confirm_check`, `wrong_amount_waiting`) сознательно не трогается: отмена дала бы депозиту
  дозреть в отменённый счёт.
- **`claim_url` и `payment.url` могут приходить пустой строкой.** Шлюз собирает их из своего
  публичного базового URL; в проде без него он не стартует, но на локальном стенде поле пустое.
  Оговорка добавлена в README и rustdoc: стройте ссылку сами из `claim_token` / `uuid`.
- **Статус в теле вебхука не бывает `wrong_amount_waiting`:** вебхуки шлют неуточнённый статус,
  поэтому частичная оплата приезжает как `confirm_check`; уточнённое значение отдаёт только
  `payments().info(...)`.
- **`PaymentLinkPayment::status` отдаёт ВНУТРЕННИЕ литералы инвойса** (`created`, `expired`,
  `cancelled`), а не словарь `payment_status` — задокументировано, чтобы это не сравнивали с
  `check`/`cancel`.
- **Дубли выплатных ссылок при авто-повторе.** `payout_links().create` и
  `payout_links().create_batch` резервируют баланс, но шли БЕЗ `Idempotency-Key`: потерянный ответ
  (таймаут/5xx) приводил к повтору, а повтор — ко второй профинансированной ссылке и второму
  резерву. Теперь оба метода идут идемпотентным путём — ключ считается один раз до повторов, свой
  ключ у `create` задаётся полем `idempotency_key` (в тело не попадает), как у `payouts().create`.
  Нюанс пачки: частично неудачная пачка реплеится как есть — упавшие элементы под тем же ключом не
  переотправятся, шлите их новым вызовом. Шлюз теперь честно дедуплицирует эти маршруты: повтор с
  тем же ключом реплеит первый ответ (та же ссылка, тот же `claim_token`, заголовок
  `Idempotent-Replayed: true`), баланс дебетуется ровно один раз.
- **Задокументированы коды идемпотентного слоя** на `/v1/payout/link` и `/v1/payout/link/batch`:
  `400 idempotency.key_reused` (тот же ключ с другим телом), `400 idempotency.bad_key`,
  `409 idempotency.in_progress` (параллельный повтор, пока первый ещё выполняется) и
  `503 idempotency.unavailable` (стор недоступен, шлюз fail-closed и операцию НЕ выполняет).
  Классификация повторов не менялась и остаётся верной: 503 SDK переигрывает сам с ТЕМ ЖЕ ключом,
  400/409 терминальны (`is_retriable() == false`).
- **Дубль `reference` у payout-ссылки теперь `409 payoutlink.duplicate_reference`** (раньше `500`,
  который SDK повторял как транзиентный). Повтора больше не будет — конфликт возвращается вызывающему.
- **Задокументирован 256-КБ лимит кэша идемпотентности:** ответ больше 256 КБ не кэшируется, и
  повтор с тем же ключом выполнится заново. Достижимо на больших пачках payout-ссылок, поэтому
  проставляйте per-item `reference` — уникальный индекс работает как второй, durable слой защиты.
- **Документация песочницы: убрано ложное «депозит дозревает сам за ~10 минут».** Симулированный
  депозит никто не переэмитит, поэтому инвойс с недобором подтверждений не станет `paid` никогда —
  нужно повторить `simulate_deposit` с ТЕМ ЖЕ `txid` и бОльшим `confirmations`. Эти самые ~10 минут
  относятся к другому механизму — maturity-холду на ВЫПЛАТЕ (`payout.funds_maturing`), который в
  песочнице снимается по возрасту.
- **Уточнено описание `payout.funds_maturing`** в разделе обработки ошибок: ошибка терминальна
  (`is_retriable() == false`), SDK её не повторяет — размытое «временно» читалось наоборот.
- **Задокументировано, почему `wallets().blocked_address_refund` и `payouts().approve` НЕ шлют
  `Idempotency-Key`:** первый идемпотентен по состоянию (ссылка выплаты выводится из id кошелька и
  ищется под per-wallet блокировкой, повтор отдаёт ту же выплату), второй — переход статуса
  (одобряется только pending, иначе `409 payout.not_pending`, так что повторный approve не может
  одобрить или двинуть деньги дважды). Поведение не изменилось; оборачивать возврат middleware было
  бы регрессом — конкурентный повтор получал бы `409 idempotency.in_progress` вместо ожидания и
  успеха. Оговорка: адрес не входит в ссылку выплаты, поэтому повтор с ДРУГИМ адресом вернёт первую
  выплату на ПЕРВЫЙ адрес.

## [1.1.0] — 2026-07-15

### Изменено (ЛОМАЮЩЕЕ): идемпотентность
- **Авто-подстановка `order_id` удалена.** SDK больше не вписывает `idem-<hex>` в
  `payments().create(...)` и `account().transfer_to_personal(...)` — `order_id` уходит строго как
  есть. Если вы полагались на сгенерированный ключ в ответе, задавайте `order_id` явно.
- **Защита от дублей теперь заголовком `Idempotency-Key`.** Все создающие вызовы
  (`/v1/payment`, `/v1/payment/refund`, `/v1/payment/resolve`, `/v1/payment/batch`,
  `/v1/refund/batch`, `/v1/payout`, `/v1/payout/mass`, `/v1/payout/batch`,
  `/v1/transfer/to-personal`) шлют UUID v4, сгенерированный один раз ДО повторов, — все внутренние
  ретраи вызова идут с одним ключом. Заголовок в подпись запроса не входит.
- **Свой ключ идемпотентности:** поле `idempotency_key` в параметрах создающего вызова уходит в
  заголовок и не попадает в тело запроса.
- Исключение: эндпоинты `/v1/payout/link*` заголовок не поддерживают — SDK его там не шлёт,
  дедупликация через per-link `reference`.
  *(Устарело: с 1.2.0 шлюз дедуплицирует `/v1/payout/link` и `/v1/payout/link/batch`, и SDK шлёт
  на них `Idempotency-Key` — см. запись 1.2.0.)*

### Добавлено
- **Массовые операции (до 5000 элементов):** `payments().create_batch(...)`,
  `payments().refund_batch(...)`, `payouts().create_batch(...)` и `batches().info(...)`
  (типы `BatchSubmission`, `BatchInfo`, `BatchItem`).
- **Платёжные ссылки:** `payment_links().create/list/info/toggle`, публичные (без подписи)
  `public_get` и `checkout` (тип `PaymentLink`).
- **Сплит-платежи:** `splits().create_rule/list_rules/delete_rule/get_config/set_config`
  (типы `SplitRule`, `SplitConfig`).
- **Счёт на e-mail:** `payments().send_email(uuid, order_id, email)`.
- **Payout-ссылки («крипто-чеки»):** `payout_links().create/create_batch(до 500)/list/info/cancel`
  и публичные (без подписи) `claim_info(token)` / `claim(token, address, memo)` — типы
  `PayoutLink`, `PayoutLinkStatus` (enum), `PayoutLinkBatch`, `ClaimInfo`, `ClaimResult`.
  Внимание: `expires_in_hours` задавайте явно — при 0/отсутствии бэкенд клампит срок к 1 часу.
- **Resolve недоплаты:** `payments().resolve(ResolveAction::Accept | ResolveAction::Refund, params)`
  (типы `ResolveAction`, `Resolution`).

## [1.0.2] — 2026-07-12

### Исправлено (устойчивость)
- **Нормализация проверки «отсутствующего» `order_id`.** Автоподстановка ключа идемпотентности
  теперь срабатывает не только на отсутствие/`null`/`""`, но и на строку из одних пробелов
  (`"   "`), а нестроковое значение `order_id` (число/bool) больше не проходит «как есть» — вместо
  него подставляется стабильный `idem-<hex>`. Реальный непустой `order_id` сохраняется без изменений.

## [1.0.1] — 2026-07-12

### Исправлено (безопасность денег)
- **Автоматический ключ идемпотентности.** `payments().create(...)` и
  `account().transfer_to_personal(...)` теперь подставляют стабильный `order_id` (`idem-<hex>`),
  если он не задан. Раньше повтор неидемпотентного POST после таймаута/`503` переподписывался и мог
  создать дубль (бэкенд дедуплицирует по `order_id`).
- **`Retry-After` больше не обрезается до `max_delay`.** Совет сервера (напр. `Retry-After: 60`)
  соблюдается как есть, с абсолютным потолком 300 с — иначе повтор упирался бы в тот же лимит.
- **Реальный джиттер повторов** вместо фиксированной доли (`0.25 * initial_delay`) — случайная
  добавка в `[0, initial_delay)` поверх ОС-RNG (`getrandom`).
- **`payout.funds_maturing` стал терминальной ошибкой** — немедленный повтор не помогает, пока
  средства дозревают.

## [1.0.0] — 2026-07-12

### Добавлено
- Первый релиз официального Rust SDK для платёжного шлюза Oblodai.
- Приём платежей, выплаты и массовые выплаты, статические кошельки, возвраты, вебхуки,
  публичные справочники (курсы валют, каталог монет и сетей).
- Подпись запросов HMAC-SHA256 и проверка подписи вебхуков (сравнение в постоянном времени,
  защита от replay).
- Конструктор из переменных окружения `Client::from_env()` — `OBLODAI_PUBLIC_ID` / `OBLODAI_SECRET` /
  `OBLODAI_BASE_URL`.
- Автоматические повторы с экспоненциальным backoff и учётом заголовка `Retry-After` на 429.
