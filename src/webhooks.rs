//! Проверка входящих вебхуков.

use crate::error::{Error, Result};
use crate::logging::{log_env, LogLevel};
use crate::signing::compute_webhook_signature;
use serde::de::DeserializeOwned;
use std::time::{SystemTime, UNIX_EPOCH};

/// Заголовки доставки вебхука.
pub struct WebhookHeaders<'a> {
    /// `X-Webhook-Timestamp`.
    pub timestamp: &'a str,
    /// `X-Webhook-Signature`.
    pub signature: &'a str,
}

/// Параметры проверки вебхука.
pub struct VerifyOptions {
    /// Окно свежести для replay-защиты в секундах. `0` отключает проверку. По умолчанию 300.
    pub max_age_seconds: u64,
    /// Фиксированное «сейчас» в unix-секундах (для тестов). `None` = системное время.
    pub now: Option<u64>,
}

impl Default for VerifyOptions {
    fn default() -> Self {
        Self { max_age_seconds: 300, now: None }
    }
}

/// Проверяет подпись и свежесть вебхука.
///
/// ВАЖНО: `raw_body` должен быть СЫРЫМ телом запроса — тем же, что пришло по сети. Подпись считается
/// по байтам; пересериализованный JSON не подойдёт.
///
/// Пробные тела (`"is_test": true`) НЕ подписаны — их этой функцией проверять не нужно.
pub fn verify_webhook(
    secret: &str,
    raw_body: &[u8],
    headers: &WebhookHeaders,
    opts: &VerifyOptions,
) -> Result<()> {
    if headers.timestamp.is_empty() || headers.signature.is_empty() {
        // Не логируем сами значения — только причину отказа.
        log_env(LogLevel::Warn, "oblodai: webhook verify failed: missing timestamp or signature");
        return Err(Error::Signature("отсутствует timestamp или signature вебхука".into()));
    }

    let expected = compute_webhook_signature(secret, headers.timestamp, raw_body);

    // Сравнение в постоянном времени. НИКОГДА не логируем ожидаемую/полученную подпись.
    if !constant_time_eq(expected.as_bytes(), headers.signature.as_bytes()) {
        log_env(LogLevel::Warn, "oblodai: webhook verify failed: signature mismatch");
        return Err(Error::Signature("подпись вебхука не совпадает".into()));
    }

    if opts.max_age_seconds > 0 {
        let ts: u64 = match headers.timestamp.parse() {
            Ok(ts) => ts,
            Err(_) => {
                log_env(LogLevel::Warn, "oblodai: webhook verify failed: invalid timestamp");
                return Err(Error::Signature("некорректный timestamp вебхука".into()));
            }
        };
        let now = opts.now.unwrap_or_else(|| {
            SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
        });
        let age = now.abs_diff(ts);
        if age > opts.max_age_seconds {
            log_env(
                LogLevel::Warn,
                &format!("oblodai: webhook verify failed: too old ({age}s > {}s, replay)", opts.max_age_seconds),
            );
            return Err(Error::Signature("вебхук слишком старый (replay-защита)".into()));
        }
    }

    log_env(LogLevel::Debug, "oblodai: webhook signature ok");
    Ok(())
}

/// Проверяет вебхук и десериализует тело в тип `T`.
pub fn construct_event<T: DeserializeOwned>(
    secret: &str,
    raw_body: &[u8],
    headers: &WebhookHeaders,
    opts: &VerifyOptions,
) -> Result<T> {
    verify_webhook(secret, raw_body, headers, opts)?;
    serde_json::from_slice(raw_body).map_err(|e| Error::Serialization(e.to_string()))
}

/// Сравнение байтовых срезов в постоянном времени (без зависимостей).
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
