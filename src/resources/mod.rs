//! The method tree: one namespace per area of the API, one method per route.

pub mod account;
pub mod base;
pub mod batches;
pub mod documents;
pub mod links;
pub mod merchants;
pub mod payments;
pub mod payouts;
pub mod refunds;
pub mod sandbox;
pub mod settings;
pub mod splits;
pub mod wallets;
pub mod webhooks;

pub use account::{Account, Catalog};
pub use base::{FileBuilder, FileResult, Lookup, PaymentLookup, PayoutLookup, RequestBuilder};
pub use batches::{BatchInfoCall, Batches, Transfers};
pub use documents::{DocumentFormat, DocumentQuery, Documents, FormatQuery, PeriodQuery};
pub use links::{PaymentLinks, PayoutLinks};
pub use merchants::Merchants;
pub use payments::Payments;
pub use payouts::Payouts;
pub use refunds::Refunds;
pub use sandbox::Sandbox;
pub use settings::Settings;
pub use splits::Splits;
pub use wallets::Wallets;
pub use webhooks::Webhooks;
