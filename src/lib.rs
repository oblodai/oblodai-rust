//! # Oblodai SDK
//!
//! Rust-клиент для платёжного шлюза Oblodai: приём платежей, выплаты, статические кошельки, вебхуки.
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
//!     Config::new("oblodai_...", "oblodai_live_...")
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

pub mod client;
pub mod error;
pub mod http;
pub mod logging;
pub mod models;
pub mod resources;
pub mod signing;
pub mod webhooks;

// Публичные реэкспорты для удобства.
pub use client::{Client, Config, RetryConfig};
#[cfg(feature = "reqwest-client")]
pub use client::ReqwestTransport;
pub use error::{Error, Result};
pub use http::{HttpResponse, HttpTransport};
pub use logging::{LogLevel, Logger, ENV_LOG};
pub use signing::compute_webhook_signature;
pub use webhooks::{construct_event, verify_webhook, VerifyOptions, WebhookHeaders};
