//! Клиент Oblodai API.

use crate::error::{Error, Result};
use crate::http::{parse_response, HttpResponse, HttpTransport};
use crate::logging::{env_logger, LogLevel, Logger};
use crate::signing::{now_ts, sign_request};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};

const DEFAULT_BASE_URL: &str = "https://api.oblodai.com";

/// Таймаут одной HTTP-попытки по умолчанию.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Настройки повторов с экспоненциальным backoff.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Максимум попыток (включая первую).
    pub max_attempts: u32,
    /// Начальная задержка.
    pub initial_delay: Duration,
    /// Потолок задержки.
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 4,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
        }
    }
}

/// Конфигурация клиента.
///
/// Примечание: тип НЕ выводит `Clone`/`Debug`. Поле [`Config::logger`] — это `Option<Arc<dyn Fn>>`;
/// оно клонируемо (`Arc: Clone`), но НЕ реализует `Debug`. Если позже понадобится `#[derive(Debug)]`,
/// это поле придётся пропускать/оборачивать вручную.
pub struct Config {
    /// `public_id` — несекретный идентификатор ключа (обязателен).
    pub public_id: String,
    /// Секрет API-ключа: подписывает ИСХОДЯЩИЕ запросы SDK (обязателен).
    ///
    /// Это НЕ секрет для проверки вебхуков — тот отдельный и возвращается
    /// [`crate::resources::Webhooks::register`] в поле `secret`.
    pub secret: String,
    /// Базовый URL API (по умолчанию `https://api.oblodai.com`).
    ///
    /// Схема обязана быть `https://`. Единственное исключение — loopback (`localhost`,
    /// `127.0.0.0/8`, `::1`): по нему поднимают локальные стенды, и там `http://` разрешён.
    /// На любом другом хосте `http://` отвергается ошибкой [`Error::Config`]: подпись запроса
    /// (`X-Signature`) и `X-Public-Id` ушли бы по открытому каналу.
    pub base_url: String,
    /// Настройки повторов. `None` — без повторов.
    pub retry: Option<RetryConfig>,
    /// Таймаут ОДНОЙ HTTP-попытки (по умолчанию [`DEFAULT_TIMEOUT`] — 30 секунд).
    ///
    /// Это таймаут попытки, а не всего вызова: при включённых повторах суммарное время ожидания
    /// равно `timeout * max_attempts` плюс задержки backoff. Значение применяет встроенный
    /// reqwest-транспорт; свой [`HttpTransport`] волен трактовать его по-своему.
    pub timeout: Duration,
    /// Опциональный логгер (по умолчанию `None` — логирование выключено). Если `None`, но задана
    /// переменная окружения `OBLODAI_LOG`, при создании клиента будет установлен встроенный
    /// stderr-логгер. Логи НИКОГДА не содержат секрет/подпись/тела — только метаданные запроса.
    pub logger: Option<Logger>,
}

impl Config {
    /// Создаёт конфиг с обязательными полями и значениями по умолчанию для остальных.
    pub fn new(public_id: impl Into<String>, secret: impl Into<String>) -> Self {
        Self {
            public_id: public_id.into(),
            secret: secret.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            retry: Some(RetryConfig::default()),
            timeout: DEFAULT_TIMEOUT,
            logger: None,
        }
    }

    /// Задаёт базовый URL. Схема — только `https://`, кроме loopback (см. [`Config::base_url`]).
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Задаёт таймаут одной HTTP-попытки (по умолчанию 30 секунд).
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Задаёт настройки повторов (или `None`).
    pub fn retry(mut self, retry: Option<RetryConfig>) -> Self {
        self.retry = retry;
        self
    }

    /// Задаёт колбэк-логгер. Он получает уровень и уже отформатированное сообщение и НИКОГДА не
    /// увидит секрет/подпись/тело — только method/path/status/ms/attempt/delay/код ошибки.
    /// Переопределяет включение через `OBLODAI_LOG`.
    pub fn logger(mut self, logger: Logger) -> Self {
        self.logger = Some(logger);
        self
    }

    /// Собирает конфиг из переменных окружения: `OBLODAI_PUBLIC_ID` и `OBLODAI_SECRET` (обязательны),
    /// `OBLODAI_BASE_URL` (необязательна). Возвращает [`Error::Config`], если обязательная переменная
    /// не задана.
    pub fn from_env() -> Result<Self> {
        let public_id = env_var(ENV_PUBLIC_ID)?;
        let secret = env_var(ENV_SECRET)?;
        let base_url = std::env::var(ENV_BASE_URL)
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        Ok(Self {
            public_id,
            secret,
            base_url,
            retry: Some(RetryConfig::default()),
            timeout: DEFAULT_TIMEOUT,
            logger: None,
        })
    }
}

// ─────────────────────────── Проверка базового URL ───────────────────────────

/// Проверяет схему базового URL: разрешён только `https://`, кроме loopback-хостов
/// (`localhost`, `127.0.0.0/8`, `::1`), где допустим и `http://` — там работают локальные стенды.
///
/// Причина запрета: SDK кладёт в заголовки `X-Public-Id`, `X-Timestamp` и `X-Signature`. По
/// открытому HTTP их видит любой посредник, а подпись пригодна для повтора запроса.
fn validate_base_url(raw: &str) -> Result<()> {
    let trimmed = raw.trim();
    let Some((scheme, rest)) = trimmed.split_once("://") else {
        return Err(Error::Config(format!(
            "base_url должен начинаться с https:// (получено: {trimmed:?})"
        )));
    };
    let scheme = scheme.to_ascii_lowercase();
    if scheme == "https" {
        return Ok(());
    }
    if scheme != "http" {
        return Err(Error::Config(format!(
            "неподдерживаемая схема base_url: {scheme:?}; ожидается https://"
        )));
    }
    let host = host_of(rest);
    if is_loopback(&host) {
        return Ok(());
    }
    Err(Error::Config(format!(
        "base_url должен использовать https://, получен http:// на хосте {host:?}: \
         по открытому каналу уходят X-Public-Id и подпись запроса X-Signature. \
         http:// допустим только для loopback (localhost, 127.0.0.1, ::1) — локальных стендов"
    )))
}

/// Достаёт хост из части URL после `://`: отбрасывает userinfo, путь/запрос/фрагмент и порт.
/// IPv6 в скобках (`[::1]:8095`) разворачивается в `::1`.
fn host_of(rest: &str) -> String {
    let authority = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .rsplit('@')
        .next()
        .unwrap_or_default();
    let host = if let Some(end) = authority.strip_prefix('[').and_then(|s| s.find(']')) {
        &authority[1..=end]
    } else {
        authority.split(':').next().unwrap_or_default()
    };
    host.trim_end_matches(']').to_ascii_lowercase()
}

/// `true` для loopback-хостов: `localhost`, любой адрес из `127.0.0.0/8`, IPv6 `::1`.
fn is_loopback(host: &str) -> bool {
    if host == "localhost" || host == "::1" || host == "0:0:0:0:0:0:0:1" {
        return true;
    }
    let mut octets = host.split('.');
    let first = octets.next().unwrap_or_default();
    if first != "127" {
        return false;
    }
    let rest: Vec<&str> = octets.collect();
    rest.len() == 3 && rest.iter().all(|o| o.parse::<u8>().is_ok())
}

/// Имена переменных окружения для [`Config::from_env`].
pub const ENV_PUBLIC_ID: &str = "OBLODAI_PUBLIC_ID";
pub const ENV_SECRET: &str = "OBLODAI_SECRET";
pub const ENV_BASE_URL: &str = "OBLODAI_BASE_URL";

fn env_var(name: &str) -> Result<String> {
    match std::env::var(name) {
        Ok(v) if !v.is_empty() => Ok(v),
        _ => Err(Error::Config(format!(
            "переменная окружения {name} не задана"
        ))),
    }
}

/// `true`, если ключ — тестовый (песочница): `public_id` с префиксом `test_` либо секрет с
/// префиксом `oblodai_test_`. Тестовые и боевые ключи работают с одними и теми же бизнес-методами;
/// методы [`Client::sandbox`] доступны ТОЛЬКО тестовому ключу (боевой получит 403 `sandbox.live_key`).
pub fn is_test_key(public_id: &str) -> bool {
    public_id.starts_with("test_") || public_id.starts_with("oblodai_test_")
}

/// Клиент Oblodai API.
///
/// # Клиент БЛОКИРУЮЩИЙ
///
/// Встроенный транспорт — `reqwest::blocking`, а паузы между повторами — `std::thread::sleep`.
/// Каждый вызов блокирует поток, из которого сделан, на время запроса (до [`Config::timeout`]),
/// а при включённых повторах — ещё и на время задержек backoff.
///
/// **Внутри async-приложения (tokio/async-std) не вызывайте методы SDK напрямую из задачи** —
/// заблокированный поток исполнителя останавливает и все остальные задачи на нём. Уносите вызов
/// на блокирующий пул:
///
/// ```ignore
/// // (не компилируется в doctest: tokio не является зависимостью SDK)
/// use oblodai::{Client, Config};
/// use serde_json::json;
///
/// let payment = tokio::task::spawn_blocking(|| {
///     let client = Client::new(Config::new("test_...", "oblodai_test_..."))?;
///     client.payments().create(json!({
///         "amount": "10", "currency": "USD", "order_id": "order-1",
///         "to_currency": "USDT", "network": "tron",
///     }))
/// })
/// .await??; // первый `?` — паника/отмена задачи, второй — ошибка SDK
/// ```
///
/// [`Client`] — `Send + Sync`, поэтому его можно положить в `Arc` и переиспользовать из
/// нескольких `spawn_blocking` вместо создания на каждый вызов.
///
/// # Ресурсы
///
/// Доступны как методы: [`Client::payments`], [`Client::payouts`], [`Client::wallets`],
/// [`Client::account`], [`Client::webhooks`], [`Client::settings`], [`Client::rates`],
/// [`Client::batches`], [`Client::payment_links`], [`Client::splits`], [`Client::payout_links`],
/// [`Client::sandbox`].
pub struct Client {
    public_id: String,
    secret: String,
    base_url: String,
    retry: Option<RetryConfig>,
    timeout: Duration,
    transport: Arc<dyn HttpTransport>,
    logger: Option<Logger>,
}

impl Client {
    /// Создаёт клиента со встроенным reqwest-транспортом (фича `reqwest-client`).
    ///
    /// Возвращает [`Error::Config`], если `base_url` не `https://` (кроме loopback —
    /// см. [`Config::base_url`]).
    #[cfg(feature = "reqwest-client")]
    pub fn new(config: Config) -> Result<Self> {
        let transport = Arc::new(ReqwestTransport::with_timeout(config.timeout)?);
        Self::with_transport(config, transport)
    }

    /// Создаёт клиента из переменных окружения (`OBLODAI_PUBLIC_ID`, `OBLODAI_SECRET`,
    /// опционально `OBLODAI_BASE_URL`) со встроенным reqwest-транспортом.
    #[cfg(feature = "reqwest-client")]
    pub fn from_env() -> Result<Self> {
        Self::new(Config::from_env()?)
    }

    /// Создаёт клиента с произвольным транспортом.
    pub fn with_transport(config: Config, transport: Arc<dyn HttpTransport>) -> Result<Self> {
        if config.public_id.is_empty() {
            return Err(Error::Config("public_id обязателен".into()));
        }
        if config.secret.is_empty() {
            return Err(Error::Config("secret обязателен".into()));
        }
        // Подпись запроса не должна уходить по открытому каналу; loopback — исключение для стендов.
        validate_base_url(&config.base_url)?;
        // Логгер из конфига имеет приоритет; иначе — встроенный env-логгер по `OBLODAI_LOG`
        // (читается один раз при создании клиента).
        let logger = config.logger.or_else(env_logger);
        Ok(Self {
            public_id: config.public_id,
            secret: config.secret,
            base_url: config.base_url.trim().trim_end_matches('/').to_string(),
            retry: config.retry,
            timeout: config.timeout,
            transport,
            logger,
        })
    }

    /// Таймаут одной HTTP-попытки, с которым создан клиент (см. [`Config::timeout`]).
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// No-op, если логгер не задан; иначе вызывает колбэк. Env-логгер сам фильтрует по мин. уровню.
    fn log(&self, level: LogLevel, msg: &str) {
        if let Some(logger) = &self.logger {
            logger(level, msg);
        }
    }

    // ── Ресурсы ──

    /// Приём платежей.
    pub fn payments(&self) -> crate::resources::Payments<'_> {
        crate::resources::Payments { client: self }
    }
    /// Выплаты и возвраты.
    pub fn payouts(&self) -> crate::resources::Payouts<'_> {
        crate::resources::Payouts { client: self }
    }
    /// Статические кошельки.
    pub fn wallets(&self) -> crate::resources::Wallets<'_> {
        crate::resources::Wallets { client: self }
    }
    /// Баланс, рефералы, переводы (на личный кошелёк и пользователям), VRCS.
    pub fn account(&self) -> crate::resources::Account<'_> {
        crate::resources::Account { client: self }
    }
    /// Управление вебхуками.
    pub fn webhooks(&self) -> crate::resources::Webhooks<'_> {
        crate::resources::Webhooks { client: self }
    }
    /// Автовывод и IP-allowlist.
    pub fn settings(&self) -> crate::resources::Settings<'_> {
        crate::resources::Settings { client: self }
    }
    /// Публичные курсы.
    pub fn rates(&self) -> crate::resources::Rates<'_> {
        crate::resources::Rates { client: self }
    }
    /// Массовые операции (пачки платежей/возвратов/выплат).
    pub fn batches(&self) -> crate::resources::Batches<'_> {
        crate::resources::Batches { client: self }
    }
    /// Платёжные ссылки (переиспользуемые, «донатные»). Канонические имя ресурса во всех SDK
    /// Oblodai; короткий алиас — [`Client::links`].
    pub fn payment_links(&self) -> crate::resources::PaymentLinks<'_> {
        crate::resources::PaymentLinks { client: self }
    }
    /// Алиас [`Client::payment_links`]: тот же ресурс под коротким именем `links`.
    ///
    /// Существует, чтобы код переносился между SDK Oblodai без переименований — в разных языках
    /// исторически прижились оба имени. Канон — `payment_links`.
    pub fn links(&self) -> crate::resources::PaymentLinks<'_> {
        self.payment_links()
    }
    /// Сплит-платежи (правила и настройки).
    pub fn splits(&self) -> crate::resources::Splits<'_> {
        crate::resources::Splits { client: self }
    }
    /// Payout-ссылки («крипто-чеки»): резерв средств под claim по публичной ссылке.
    pub fn payout_links(&self) -> crate::resources::PayoutLinks<'_> {
        crate::resources::PayoutLinks { client: self }
    }
    /// Песочница (только тестовые ключи): симуляция депозитов, faucet, reset, журнал вебхуков.
    pub fn sandbox(&self) -> crate::resources::Sandbox<'_> {
        crate::resources::Sandbox { client: self }
    }

    // ── Внутреннее ──

    /// Подписанный запрос с разбором результата в тип `T`.
    pub(crate) fn request<T: DeserializeOwned>(&self, path: &str, payload: &Value) -> Result<T> {
        let value = self.execute("POST", path, payload, true, None)?;
        serde_json::from_value(value).map_err(|e| Error::Serialization(e.to_string()))
    }

    /// Подписанный запрос НА СОЗДАЮЩИЙ эндпоинт: добавляет заголовок `Idempotency-Key`.
    ///
    /// Ключ (`key`, либо сгенерированный UUID v4, если `key == None`) вычисляется ОДИН РАЗ — до
    /// повторов — и идентичен во всех внутренних попытках вызова, поэтому таймаут+повтор не создаёт
    /// дубль операции. Заголовок в подпись запроса НЕ входит (подписываются только
    /// timestamp/метод/путь/тело).
    pub(crate) fn request_idempotent<T: DeserializeOwned>(
        &self,
        path: &str,
        payload: &Value,
        key: Option<String>,
    ) -> Result<T> {
        let key = key.unwrap_or_else(crate::random::uuid4);
        let value = self.execute("POST", path, payload, true, Some(&key))?;
        serde_json::from_value(value).map_err(|e| Error::Serialization(e.to_string()))
    }

    /// Публичный (неподписанный) запрос с разбором результата в тип `T`.
    pub(crate) fn request_public<T: DeserializeOwned>(
        &self,
        path: &str,
        payload: &Value,
    ) -> Result<T> {
        let value = self.execute("POST", path, payload, false, None)?;
        serde_json::from_value(value).map_err(|e| Error::Serialization(e.to_string()))
    }

    /// Публичный GET-запрос без подписи (напр. `GET /v1/currencies`).
    pub(crate) fn request_public_get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let value = self.execute("GET", path, &Value::Null, false, None)?;
        serde_json::from_value(value).map_err(|e| Error::Serialization(e.to_string()))
    }

    /// Подписанный GET-запрос (напр. `GET /v1/sandbox/webhooks`). Каноническая строка подписи —
    /// та же, что у POST, с ПУСТЫМ телом: `{ts}\nGET\n{path}\n`.
    pub(crate) fn request_get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let value = self.execute("GET", path, &Value::Null, true, None)?;
        serde_json::from_value(value).map_err(|e| Error::Serialization(e.to_string()))
    }

    fn execute(
        &self,
        method: &str,
        path: &str,
        payload: &Value,
        signed: bool,
        idempotency_key: Option<&str>,
    ) -> Result<Value> {
        let attempts = self.retry.as_ref().map(|r| r.max_attempts).unwrap_or(1);
        let mut last: Option<Error> = None;

        for attempt in 1..=attempts {
            match self.once(
                method,
                path,
                payload,
                signed,
                idempotency_key,
                attempt,
                attempts,
            ) {
                Ok(v) => return Ok(v),
                Err(e) => {
                    let retriable = e.is_retriable();
                    // Уважаем Retry-After от сервера (напр. 429), иначе — собственный backoff.
                    let advised = e.retry_after();
                    last = Some(e);
                    if !retriable || attempt == attempts {
                        // Финальная/неповторяемая ошибка: логируем только status и код (без тела).
                        let (status, code) = error_status_code(last.as_ref().unwrap());
                        self.log(
                            LogLevel::Warn,
                            &format!("oblodai: {method} {path} failed: {status} {code}"),
                        );
                        return Err(last.unwrap());
                    }
                    if let Some(cfg) = &self.retry {
                        let delay = match advised {
                            // Уважаем совет сервера: НЕ обрезаем до max_delay (иначе `Retry-After: 60`
                            // при max_delay=30 ждал бы лишь 30с и снова упёрся бы в лимит). Ограничиваем
                            // лишь абсолютным потолком, чтобы не зависнуть на абсурдных значениях.
                            Some(d) => d.min(Duration::from_secs(300)),
                            None => backoff(attempt, cfg),
                        };
                        let reason = retry_reason(last.as_ref().unwrap());
                        let delay_ms = delay.as_millis();
                        let next = attempt + 1;
                        self.log(
                            LogLevel::Warn,
                            &format!(
                                "oblodai: retrying {method} {path} in {delay_ms}ms ({reason}; attempt {next})"
                            ),
                        );
                        std::thread::sleep(delay);
                    }
                }
            }
        }
        Err(last.unwrap())
    }

    #[allow(clippy::too_many_arguments)]
    fn once(
        &self,
        method: &str,
        path: &str,
        payload: &Value,
        signed: bool,
        idempotency_key: Option<&str>,
        attempt: u32,
        attempts: u32,
    ) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let mut headers: Vec<(String, String)> =
            vec![("Content-Type".into(), "application/json".into())];

        self.log(
            LogLevel::Debug,
            &format!("oblodai: -> {method} {path} (attempt {attempt}/{attempts})"),
        );
        let started = Instant::now();

        let resp = if method == "GET" {
            if signed {
                // Подписанный GET: та же каноническая строка, что у POST, но тело — ПУСТАЯ строка.
                let ts = now_ts();
                let sig = sign_request(&self.secret, method, path, "", &ts);
                headers.push(("X-Public-Id".into(), self.public_id.clone()));
                headers.push(("X-Timestamp".into(), ts));
                headers.push(("X-Signature".into(), sig));
            }
            self.transport.get(&url, &headers)?
        } else {
            let body =
                serde_json::to_string(payload).map_err(|e| Error::Serialization(e.to_string()))?;
            if signed {
                let ts = now_ts();
                let sig = sign_request(&self.secret, method, path, &body, &ts);
                headers.push(("X-Public-Id".into(), self.public_id.clone()));
                headers.push(("X-Timestamp".into(), ts));
                headers.push(("X-Signature".into(), sig));
            }
            // Ключ идемпотентности стабилен между повторами (вычислен ДО цикла ретраев)
            // и в подпись не входит.
            if let Some(key) = idempotency_key {
                headers.push(("Idempotency-Key".into(), key.to_string()));
            }
            self.transport.post(&url, &headers, body.as_bytes())?
        };

        let ms = started.elapsed().as_millis();
        let status = resp.status;
        self.log(
            LogLevel::Debug,
            &format!("oblodai: <- {status} {method} {path} {ms}ms"),
        );

        parse_response(resp.status, &resp.body, resp.retry_after)
    }
}

/// Достаёт (status, code) из ошибки для лога, не раскрывая тело/секреты.
fn error_status_code(err: &Error) -> (u16, String) {
    match err {
        Error::Api { status, code, .. } => (*status, code.clone()),
        Error::Connection(_) => (0, "network".to_string()),
        Error::Serialization(_) => (0, "serialization".to_string()),
        Error::Signature(_) => (0, "signature".to_string()),
        Error::Config(_) => (0, "config".to_string()),
    }
}

/// Короткая причина повтора для лога.
fn retry_reason(err: &Error) -> &'static str {
    match err {
        Error::Api { status, .. } if *status == 429 => "429 rate limit",
        Error::Api { status, .. } if *status >= 500 => "5xx",
        Error::Connection(_) => "network",
        _ => "retriable",
    }
}

fn backoff(attempt: u32, cfg: &RetryConfig) -> Duration {
    let base = (cfg.initial_delay.as_millis() as f64) * 2f64.powi(attempt as i32 - 1);
    let capped = base.min(cfg.max_delay.as_millis() as f64);
    // Реальный джиттер: случайная добавка в [0, initial_delay), чтобы разнести повторы
    // конкурирующих клиентов (full jitter поверх ОС-RNG, без внешних тяжёлых зависимостей).
    let jitter = (cfg.initial_delay.as_millis() as f64) * crate::random::unit_f64();
    Duration::from_millis((capped + jitter) as u64)
}

// ─────────────────────────── reqwest-транспорт ───────────────────────────

/// Встроенный транспорт на reqwest (blocking). Доступен под фичей `reqwest-client`.
#[cfg(feature = "reqwest-client")]
pub struct ReqwestTransport {
    client: reqwest::blocking::Client,
    timeout: Duration,
}

#[cfg(feature = "reqwest-client")]
impl ReqwestTransport {
    /// Создаёт транспорт с таймаутом по умолчанию ([`DEFAULT_TIMEOUT`] — 30 секунд).
    pub fn new() -> Result<Self> {
        Self::with_timeout(DEFAULT_TIMEOUT)
    }

    /// Создаёт транспорт с заданным таймаутом одной попытки.
    pub fn with_timeout(timeout: Duration) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| Error::Config(format!("не удалось создать HTTP-клиент: {e}")))?;
        Ok(Self { client, timeout })
    }

    /// Таймаут, с которым создан транспорт.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

#[cfg(feature = "reqwest-client")]
impl HttpTransport for ReqwestTransport {
    fn post(&self, url: &str, headers: &[(String, String)], body: &[u8]) -> Result<HttpResponse> {
        let mut req = self.client.post(url).body(body.to_vec());
        for (k, v) in headers {
            req = req.header(k.as_str(), v.as_str());
        }
        Self::finish(req)
    }

    fn get(&self, url: &str, headers: &[(String, String)]) -> Result<HttpResponse> {
        let mut req = self.client.get(url);
        for (k, v) in headers {
            req = req.header(k.as_str(), v.as_str());
        }
        Self::finish(req)
    }
}

#[cfg(feature = "reqwest-client")]
impl ReqwestTransport {
    fn finish(req: reqwest::blocking::RequestBuilder) -> Result<HttpResponse> {
        let resp = req.send().map_err(|e| Error::Connection(e.to_string()))?;
        let status = resp.status().as_u16();
        let retry_after = crate::http::parse_retry_after(
            resp.headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok()),
        );
        let bytes = resp.bytes().map_err(|e| Error::Connection(e.to_string()))?;
        Ok(HttpResponse {
            status,
            body: bytes.to_vec(),
            retry_after,
        })
    }
}
