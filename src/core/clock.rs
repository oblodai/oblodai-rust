//! Injectable clock for signing.
//!
//! The core rejects timestamps more than ±300 s from its own time; a host with a drifting clock
//! would get `merchant.bad_signature` on every call. The transport learns the server's time from
//! the `Date` header of a signature-failure response, re-signs once, and keeps the offset only if
//! that re-signed attempt got past authentication.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use super::util::{parse_http_date, unix_now};

/// Current unix time in seconds. Injectable so tests never depend on the wall clock.
pub trait Clock: Send + Sync {
    fn now(&self) -> i64;
}

/// The system clock.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> i64 {
        unix_now()
    }
}

/// Offsets beyond this are implausible clock drift and are ignored (a broken proxy `Date`).
pub const MAX_PLAUSIBLE_OFFSET_SECONDS: i64 = 24 * 3600;

/// A clock that can be nudged onto the server's time.
pub struct SkewCorrectingClock {
    base: Arc<dyn Clock>,
    offset: AtomicI64,
}

impl SkewCorrectingClock {
    pub fn new(base: Arc<dyn Clock>) -> Self {
        Self {
            base,
            offset: AtomicI64::new(0),
        }
    }

    pub fn now(&self) -> i64 {
        self.base.now() + self.offset.load(Ordering::Relaxed)
    }

    /// Server-minus-local offset currently applied, seconds.
    pub fn offset(&self) -> i64 {
        self.offset.load(Ordering::Relaxed)
    }

    pub fn correct(&self, offset: i64) {
        self.offset.store(offset, Ordering::Relaxed);
    }

    /// Measure the offset from a response `Date` header; `None` when absent, unparsable or
    /// implausible.
    pub fn observe_server_date(&self, date_header: Option<&str>) -> Option<i64> {
        let server = parse_http_date(date_header?)?;
        let offset = server - self.base.now();
        if offset.abs() > MAX_PLAUSIBLE_OFFSET_SECONDS {
            return None;
        }
        Some(offset)
    }
}

impl Default for SkewCorrectingClock {
    fn default() -> Self {
        Self::new(Arc::new(SystemClock))
    }
}

impl std::fmt::Debug for SkewCorrectingClock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkewCorrectingClock")
            .field("offset", &self.offset())
            .finish()
    }
}
