//! Everything between a method call and a socket: signing, envelopes, retries, hooks.
//!
//! The decision-making half is pure — no clock beyond an injectable one, no I/O — so the async
//! client and the `blocking` client share it byte for byte, and its rules are unit-testable
//! without a network.

pub mod clock;
pub mod debug;
pub mod engine;
pub mod envelope;
pub mod hooks;
pub mod http;
pub mod idempotency;
pub mod logger;
pub mod money;
pub mod request;
pub mod retry;
pub mod route;
pub mod signing;
pub mod transport;
pub mod util;
