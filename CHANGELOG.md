# Changelog

Значимые изменения этого пакета. Формат — [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/),
версии — [SemVer](https://semver.org/lang/ru/).

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
