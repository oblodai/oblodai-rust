//! Retry policy. Two questions decide every retry:
//!
//! 1. **Can it succeed?** — the core's `retryable` flag (authoritative when the core wrote the
//!    envelope), or a transient status for answers that carry no envelope.
//! 2. **Is repeating safe?** — only for read-only routes and for writes the core deduplicates by
//!    [`HEADER_IDEMPOTENCY_KEY`](super::signing::HEADER_IDEMPOTENCY_KEY). A write the core does
//!    not deduplicate is never re-sent once it MAY have reached the core: a transport error or a
//!    proxy 503 after the request left the socket could mean the payout already happened.
//!
//! An envelope error on an unsafe write is still retried when `retryable` — the core answered, so
//! it did not perform the operation (429/503/frozen/maturing all fail before any effect).
//! `Retry-After` always wins over the computed backoff; otherwise exponential backoff with jitter.

use crate::error::{Error, ErrorKind};

/// Knobs of the retry policy. `max_retries: 0` disables retries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetryOptions {
    /// Maximum number of retries after the first attempt. Default 2.
    pub max_retries: u32,
    /// Base delay for the first retry, ms. Default 250.
    pub base_delay_ms: u64,
    /// Upper bound for a computed (non-`Retry-After`) delay, ms. Default 4000.
    pub max_delay_ms: u64,
    /// Upper bound honoured for a server-provided `Retry-After`, ms. Default 30000.
    pub max_retry_after_ms: u64,
}

impl Default for RetryOptions {
    fn default() -> Self {
        Self {
            max_retries: 2,
            base_delay_ms: 250,
            max_delay_ms: 4_000,
            max_retry_after_ms: 30_000,
        }
    }
}

/// Where a retry decision is being taken.
#[derive(Clone, Copy, Debug)]
pub struct RetryContext {
    /// 0 for the first retry decision (i.e. after attempt #1 failed).
    pub attempt: u32,
    /// True when re-sending cannot duplicate a side effect.
    pub safe_to_repeat: bool,
}

/// Whether this failure may be retried at all.
pub fn should_retry(err: &Error, ctx: RetryContext, opts: &RetryOptions) -> bool {
    if ctx.attempt >= opts.max_retries {
        return false;
    }
    if !err.retryable() {
        return false;
    }
    if err.kind() == ErrorKind::Transport {
        return ctx.safe_to_repeat;
    }
    // No core envelope: something in front of the core answered; the core may have done the work.
    if err.synthetic() {
        return ctx.safe_to_repeat;
    }
    true
}

/// Delay before the next attempt, in ms. `random` is a value in `[0, 1)`, injectable so tests are
/// deterministic.
pub fn retry_delay_ms(err: &Error, ctx: RetryContext, opts: &RetryOptions, random: f64) -> u64 {
    if let Some(after) = err.retry_after() {
        if after > 0 {
            return (after.saturating_mul(1000)).min(opts.max_retry_after_ms);
        }
    }
    let exp = opts.max_delay_ms.min(
        opts.base_delay_ms
            .saturating_mul(1u64 << ctx.attempt.min(16)),
    );
    // Full jitter with a floor so a burst of retries never lands in the same instant.
    let jittered = (random.clamp(0.0, 1.0) * exp as f64) as u64;
    jittered.max(exp / 4)
}

/// A tiny non-cryptographic PRNG for backoff jitter (the keys that matter come from `uuid`).
pub fn jitter() -> f64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static STATE: AtomicU64 = AtomicU64::new(0);
    let seed = STATE.fetch_add(1, Ordering::Relaxed)
        ^ std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64)
            .unwrap_or(0);
    // splitmix64
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}
