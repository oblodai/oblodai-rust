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
    /// `secret` для подписи запросов (обязателен).
    pub secret: String,
    /// Базовый URL API (по умолчанию `https://api.oblodai.com`).
    pub base_url: String,
    /// Настройки повторов. `None` — без повторов.
    pub retry: Option<RetryConfig>,
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
            logger: None,
        }
    }

    /// Задаёт базовый URL.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
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
            logger: None,
        })
    }
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

/// Клиент Oblodai API.
///
/// Ресурсы доступны как методы: [`Client::payments`], [`Client::payouts`], [`Client::wallets`],
/// [`Client::account`], [`Client::webhooks`], [`Client::settings`], [`Client::rates`].
pub struct Client {
    public_id: String,
    secret: String,
    base_url: String,
    retry: Option<RetryConfig>,
    transport: Arc<dyn HttpTransport>,
    logger: Option<Logger>,
}

impl Client {
    /// Создаёт клиента со встроенным reqwest-транспортом (фича `reqwest-client`).
    #[cfg(feature = "reqwest-client")]
    pub fn new(config: Config) -> Result<Self> {
        let transport = Arc::new(ReqwestTransport::new()?);
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
        // Логгер из конфига имеет приоритет; иначе — встроенный env-логгер по `OBLODAI_LOG`
        // (читается один раз при создании клиента).
        let logger = config.logger.or_else(env_logger);
        Ok(Self {
            public_id: config.public_id,
            secret: config.secret,
            base_url: config.base_url.trim_end_matches('/').to_string(),
            retry: config.retry,
            transport,
            logger,
        })
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
    /// Баланс, рефералы, перевод, VRCS.
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

    // ── Внутреннее ──

    /// Подписанный запрос с разбором результата в тип `T`.
    pub(crate) fn request<T: DeserializeOwned>(&self, path: &str, payload: &Value) -> Result<T> {
        let value = self.execute("POST", path, payload, true)?;
        serde_json::from_value(value).map_err(|e| Error::Serialization(e.to_string()))
    }

    /// Публичный (неподписанный) запрос с разбором результата в тип `T`.
    pub(crate) fn request_public<T: DeserializeOwned>(
        &self,
        path: &str,
        payload: &Value,
    ) -> Result<T> {
        let value = self.execute("POST", path, payload, false)?;
        serde_json::from_value(value).map_err(|e| Error::Serialization(e.to_string()))
    }

    /// Публичный GET-запрос без подписи (напр. `GET /v1/currencies`).
    pub(crate) fn request_public_get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let value = self.execute("GET", path, &Value::Null, false)?;
        serde_json::from_value(value).map_err(|e| Error::Serialization(e.to_string()))
    }

    fn execute(&self, method: &str, path: &str, payload: &Value, signed: bool) -> Result<Value> {
        let attempts = self.retry.as_ref().map(|r| r.max_attempts).unwrap_or(1);
        let mut last: Option<Error> = None;

        for attempt in 1..=attempts {
            match self.once(method, path, payload, signed, attempt, attempts) {
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
                            Some(d) => d.min(cfg.max_delay),
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
    // Детерминированный «джиттер» без внешних зависимостей: доля от initial_delay.
    let jitter = (cfg.initial_delay.as_millis() as f64) * 0.25;
    Duration::from_millis((capped + jitter) as u64)
}

// ─────────────────────────── reqwest-транспорт ───────────────────────────

/// Встроенный транспорт на reqwest (blocking). Доступен под фичей `reqwest-client`.
#[cfg(feature = "reqwest-client")]
pub struct ReqwestTransport {
    client: reqwest::blocking::Client,
}

#[cfg(feature = "reqwest-client")]
impl ReqwestTransport {
    /// Создаёт транспорт с таймаутом 30с.
    pub fn new() -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| Error::Config(format!("не удалось создать HTTP-клиент: {e}")))?;
        Ok(Self { client })
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
