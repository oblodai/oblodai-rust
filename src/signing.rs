//! Подпись запросов и вебхуков (HMAC-SHA256).
//!
//! ВНИМАНИЕ: подпись ЗАПРОСА и подпись ВЕБХУКА — разные алгоритмы.
//! - Запрос: `{timestamp}\n{METHOD}\n{path}\n{body}`.
//! - Вебхук: `{timestamp}.{сырое_тело}` (точка-разделитель, без метода и пути).

use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// Текущее время в unix-секундах строкой.
pub(crate) fn now_ts() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        .to_string()
}

/// Считает подпись запроса: `hex(HMAC-SHA256(secret, "{ts}\n{method}\n{path}\n{body}"))`.
pub(crate) fn sign_request(secret: &str, method: &str, path: &str, body: &str, ts: &str) -> String {
    let signing = format!("{ts}\n{method}\n{path}\n{body}");
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC key of any size");
    mac.update(signing.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Считает ожидаемую подпись вебхука: `hex(HMAC-SHA256(secret, "{ts}." + raw_body))`.
pub fn compute_webhook_signature(secret: &str, timestamp: &str, raw_body: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC key of any size");
    mac.update(timestamp.as_bytes());
    mac.update(b".");
    mac.update(raw_body);
    hex::encode(mac.finalize().into_bytes())
}
