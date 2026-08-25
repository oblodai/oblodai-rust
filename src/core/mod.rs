//! Everything between a method call and a socket: signing, envelopes, retries, pagination.
//!
//! The decision-making half is pure — no clock beyond an injectable one, no I/O — so the async
//! client and the `blocking` client share it byte for byte, and its rules are unit-testable
//! without a network.

pub mod clock;
pub mod engine;
pub mod envelope;
pub mod http;
pub mod idempotency;
pub mod logger;
pub mod pagination;
pub mod request;
pub mod retry;
pub mod signing;
pub mod transport;
pub mod util;
