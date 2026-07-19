//! # Oblodai SDK
//!
//! Rust-клиент для платёжного шлюза Oblodai: приём платежей, выплаты, статические кошельки, вебхуки.
//!
//! **Клиент блокирующий** (`reqwest::blocking` + `std::thread::sleep` в повторах). В async-коде
//! вызывайте его через `tokio::task::spawn_blocking` — иначе заблокируете поток исполнителя.
//! Подробности и пример — в документации [`Client`].
//!
//! ## Базовый URL — только HTTPS
//!
//! [`Client::new`] отвергает `http://` на внешнем хосте: подпись запроса (`X-Signature`) и
//! `X-Public-Id` ушли бы открытым текстом. Исключение — loopback (`localhost`, `127.0.0.1`,
//! `::1`) для локальных стендов.
//!
//! ## Пример
//!
//! ```no_run
//! # #[cfg(feature = "reqwest-client")]
//! # fn demo() -> oblodai::Result<()> {
//! use oblodai::{Client, Config};
//! use serde_json::json;
//!
//! let client = Client::new(
//!     Config::new("test_...", "oblodai_test_...")
//!         .base_url("https://api.oblodai.com"),
//! )?;
//!
//! let payment = client.payments().create(json!({
//!     "amount": "10",
//!     "currency": "USD",
//!     "order_id": "order-1",
//!     "to_currency": "USDT",
//!     "network": "tron",
//! }))?;
//!
//! println!("{} {}", payment.address, payment.url);
//! # Ok(())
//! # }
//! ```
//!
//! ## Статусы
//!
//! Словарь статусов типизирован: [`PaymentStatus`] (платежи), [`PayoutStatus`] (выплаты),
//! [`PayoutLinkStatus`] (payout-ссылки). Поля моделей остаются строками, разбор — через
//! `payment.status()` / `payout.status()` или `PaymentStatus::from_api(...)`.
//!
//! ## Два разных секрета
//!
//! - **Секрет API-ключа** (`Config::secret`) подписывает ИСХОДЯЩИЕ запросы SDK.
//! - **Секрет эндпоинта** (поле `secret` из [`resources::Webhooks::register`]) проверяет ВХОДЯЩИЕ
//!   вебхуки в [`verify_webhook`] / [`construct_event`].
//!
//! Это разные значения; перепутав их, вы отвергнете все вебхуки.

pub mod client;
pub mod error;
pub mod http;
pub mod logging;
pub mod models;
pub(crate) mod random;
pub mod resources;
pub mod signing;
pub mod webhooks;

// Публичные реэкспорты для удобства.
#[cfg(feature = "reqwest-client")]
pub use client::ReqwestTransport;
pub use client::{is_test_key, Client, Config, RetryConfig, DEFAULT_TIMEOUT};
pub use error::{Error, Result};
pub use http::{HttpResponse, HttpTransport};
pub use logging::{LogLevel, Logger, ENV_LOG};
pub use models::{PaymentStatus, PayoutLinkStatus, PayoutStatus, ResolveAction};
pub use signing::compute_webhook_signature;
pub use webhooks::{construct_event, verify_webhook, VerifyOptions, WebhookHeaders};
