//! Injectable clock for signing.
//!
//! The core rejects timestamps more than
//! ±[`SIGNATURE_SKEW_SECONDS`](super::signing::SIGNATURE_SKEW_SECONDS) from its own time; a host
//! with a drifting clock would get `merchant.bad_signature` on every call. The transport learns
//! the server's time from the `Date` header of a signature-failure response, re-signs once, and
//! adopts the offset only if that re-signed attempt succeeded (2xx). Offsets beyond
//! ±[`MAX_CLOCK_CORRECTION_SECONDS`] are never adopted: a hostile or broken responder could
//! otherwise push every later signature hours into the future (delayed replay).

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

/// The largest correction the SDK ever applies, in seconds (15 minutes): an offset beyond it, or a
/// move of the offset beyond it, is ignored (a broken proxy `Date`, or a hostile responder).
pub const MAX_CLOCK_CORRECTION_SECONDS: i64 = 900;

/// Kept for compatibility; equal to [`MAX_CLOCK_CORRECTION_SECONDS`].
#[deprecated(note = "use MAX_CLOCK_CORRECTION_SECONDS")]
pub const MAX_PLAUSIBLE_OFFSET_SECONDS: i64 = MAX_CLOCK_CORRECTION_SECONDS;

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
        self.base.now() + self.offset.load(Ordering::SeqCst)
    }

    /// The underlying clock, without the correction applied. Signing reads this once together
    /// with [`offset`](Self::offset) so the timestamp and the recorded offset always agree.
    pub fn raw_now(&self) -> i64 {
        self.base.now()
    }

    /// Server-minus-local offset currently applied, seconds.
    pub fn offset(&self) -> i64 {
        self.offset.load(Ordering::SeqCst)
    }

    /// Install an offset for every call from now on. Offsets beyond
    /// ±[`MAX_CLOCK_CORRECTION_SECONDS`] are ignored.
    pub fn correct(&self, offset: i64) {
        if offset.abs() > MAX_CLOCK_CORRECTION_SECONDS {
            return;
        }
        self.offset.store(offset, Ordering::SeqCst);
    }

    /// Undo a correction, but only if the offset currently applied is still the one this call
    /// installed. Concurrent calls share one clock: a second call that measured a different skew
    /// (or a later successful correction) must not be rolled back by this call's failure.
    ///
    /// Returns whether the revert happened.
    pub fn revert(&self, installed: i64, previous: i64) -> bool {
        self.offset
            .compare_exchange(installed, previous, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Measure the offset from a response `Date` header; `None` when absent, unparsable or
    /// implausible.
    pub fn observe_server_date(&self, date_header: Option<&str>) -> Option<i64> {
        let server = parse_http_date(date_header?)?;
        let offset = server - self.base.now();
        if offset.abs() > MAX_CLOCK_CORRECTION_SECONDS {
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
