//! HTTP-транспорт (абстракция) и разбор ответов.
//!
//! Ядро SDK не зависит от конкретного HTTP-клиента: оно работает через трейт [`HttpTransport`].
//! Встроенная реализация на reqwest доступна под фичей `reqwest-client` (включена по умолчанию).
//! Для тестов или своего клиента реализуйте трейт самостоятельно.

use crate::error::{Error, Result};
use std::time::Duration;

/// Ответ HTTP-транспорта: статус, тело и (если был) заголовок `Retry-After`.
pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
    /// Значение заголовка `Retry-After` в виде длительности (для 429). `None`, если заголовка нет.
    pub retry_after: Option<Duration>,
}

impl HttpResponse {
    /// Создаёт ответ без `Retry-After` (удобно для транспортов/тестов, которым заголовок не нужен).
    pub fn new(status: u16, body: Vec<u8>) -> Self {
        Self {
            status,
            body,
            retry_after: None,
        }
    }
}

/// Абстракция HTTP-клиента. Реализуйте, чтобы подставить свой транспорт.
pub trait HttpTransport: Send + Sync {
    /// Выполняет POST-запрос. `headers` — пары (имя, значение). Возвращает статус и тело либо
    /// [`Error::Connection`] при сетевом сбое.
    fn post(&self, url: &str, headers: &[(String, String)], body: &[u8]) -> Result<HttpResponse>;

    /// Выполняет GET-запрос (для публичных эндпоинтов вроде `GET /v1/currencies`). По умолчанию
    /// не поддерживается — реализуйте, если нужны GET-методы SDK.
    fn get(&self, url: &str, headers: &[(String, String)]) -> Result<HttpResponse> {
        let _ = (url, headers);
        Err(Error::Config(
            "GET не поддерживается этим транспортом; реализуйте HttpTransport::get".into(),
        ))
    }
}

/// Разбирает заголовок `Retry-After` в длительность. Поддерживает форму «секунды» (как отдаёт шлюз
/// на 429: `Retry-After: 60`). HTTP-date форму не используем.
pub fn parse_retry_after(header: Option<&str>) -> Option<Duration> {
    let secs: u64 = header?.trim().parse().ok()?;
    Some(Duration::from_secs(secs))
}

/// Разбирает ответ: возвращает сырой `result` из конверта или [`Error::Api`].
///
/// Обрабатывает и ответ без конверта (единственное исключение — `POST /v1/webhooks`, `201 Created`).
pub(crate) fn parse_response(
    status: u16,
    body: &[u8],
    retry_after: Option<Duration>,
) -> Result<serde_json::Value> {
    let text = String::from_utf8_lossy(body);

    // Пробуем разобрать как JSON.
    let parsed: serde_json::Value = if body.is_empty() {
        serde_json::Value::Object(Default::default())
    } else {
        match serde_json::from_slice(body) {
            Ok(v) => v,
            Err(_) => {
                if (200..300).contains(&status) {
                    // Успех, но не JSON — вернём null (вызывающий разберётся).
                    return Ok(serde_json::Value::Null);
                }
                return Err(Error::Api {
                    code: "response.not_json".into(),
                    message: format!("ответ не является JSON (HTTP {status})"),
                    status,
                    raw: text.into_owned(),
                    retry_after,
                });
            }
        }
    };

    // Конверт ошибки.
    if let Some(err) = parsed.get("error") {
        let code = err
            .get("code")
            .and_then(|c| c.as_str())
            .unwrap_or("unknown")
            .to_string();
        let message = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Неизвестная ошибка")
            .to_string();
        return Err(Error::Api {
            code,
            message,
            status,
            raw: text.into_owned(),
            retry_after,
        });
    }

    // Не-2xx без конверта ошибки. Сюда попадает и 429 (тело {"state":1,"message":"rate limit exceeded"}
    // без ключа "error") — достаём message из тела и учитываем Retry-After.
    if !(200..300).contains(&status) {
        let message = parsed
            .get("message")
            .and_then(|m| m.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("HTTP {status}"));
        return Err(Error::Api {
            code: format!("http.{status}"),
            message,
            status,
            raw: text.into_owned(),
            retry_after,
        });
    }

    // Успешный конверт { state: 0, result: ... }.
    if let Some(result) = parsed.get("result") {
        return Ok(result.clone());
    }

    // Ответ без конверта (например, /v1/webhooks).
    Ok(parsed)
}
