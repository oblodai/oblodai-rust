//! Ошибки Oblodai SDK.
//!
//! Все ошибки API приходят в конверте `{"error": {"code", "message"}}`, где `code` — машиночитаемый
//! идентификатор вида `<домен>.<причина>` (например `payout.insufficient_funds`). Ветвитесь по
//! [`Error::Api`] и полю `code`, а не по тексту сообщения.

use std::time::Duration;
use thiserror::Error;

/// Ошибки SDK.
#[derive(Debug, Error)]
pub enum Error {
    /// Ошибка, вернувшаяся от API (конверт `error`).
    #[error("oblodai: API error {code} (HTTP {status}): {message}")]
    Api {
        /// Машиночитаемый код вида `<домен>.<причина>`.
        code: String,
        /// Человекочитаемое пояснение.
        message: String,
        /// HTTP-статус ответа.
        status: u16,
        /// Сырое тело ответа (для отладки).
        raw: String,
        /// Рекомендованная сервером пауза перед повтором (из заголовка `Retry-After`, напр. на 429
        /// шлюз отдаёт `Retry-After: 60`). `None` — заголовка не было.
        retry_after: Option<Duration>,
    },

    /// Сетевая ошибка или таймаут. Как правило, безопасно повторить с backoff.
    #[error("oblodai: connection error: {0}")]
    Connection(String),

    /// Ошибка проверки подписи вебхука.
    #[error("oblodai: signature error: {0}")]
    Signature(String),

    /// Ошибка сериализации/десериализации.
    #[error("oblodai: serialization error: {0}")]
    Serialization(String),

    /// Некорректная конфигурация клиента.
    #[error("oblodai: config error: {0}")]
    Config(String),
}

impl Error {
    /// Временная ли ошибка (стоит ли повторять с backoff).
    pub fn is_retriable(&self) -> bool {
        match self {
            Error::Api { status, code, .. } => {
                *status >= 500 || *status == 429 || code == "payout.funds_maturing"
            }
            Error::Connection(_) => true,
            _ => false,
        }
    }

    /// Код ошибки API, если это [`Error::Api`].
    pub fn code(&self) -> Option<&str> {
        match self {
            Error::Api { code, .. } => Some(code),
            _ => None,
        }
    }

    /// Рекомендованная сервером пауза перед повтором (заголовок `Retry-After`), если была.
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Api { retry_after, .. } => *retry_after,
            _ => None,
        }
    }
}

/// Результат операций SDK.
pub type Result<T> = std::result::Result<T, Error>;
