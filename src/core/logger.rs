//! Minimal structured logging.
//!
//! Anything that can take a level, a message and a few key/value fields fits (`tracing`, `log`, a
//! test spy).
//!
//! Values whose key looks like a secret are replaced with `[redacted]` by the SDK *before* the
//! fields are handed to the [`Logger`], so a logger you supply yourself never sees a key, a
//! signature or a cheque passcode. [`StderrLogger`] redacts again on its own, which costs nothing
//! and keeps it safe if you call it directly.

/// Severity of a log line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    /// Parse `OBLODAI_LOG`; anything else disables logging.
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "debug" => Some(LogLevel::Debug),
            "info" => Some(LogLevel::Info),
            "warn" => Some(LogLevel::Warn),
            "error" => Some(LogLevel::Error),
            _ => None,
        }
    }
}

/// A sink for the SDK's structured log lines.
pub trait Logger: Send + Sync {
    fn log(&self, level: LogLevel, message: &str, fields: &[(&str, String)]);
}

/// Drops everything. The default.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopLogger;

impl Logger for NoopLogger {
    fn log(&self, _level: LogLevel, _message: &str, _fields: &[(&str, String)]) {}
}

/// Writes to stderr, gated by level. `OBLODAI_LOG=debug|info|warn|error` selects it.
#[derive(Clone, Copy, Debug)]
pub struct StderrLogger {
    pub min_level: LogLevel,
}

impl StderrLogger {
    pub fn new(min_level: LogLevel) -> Self {
        Self { min_level }
    }
}

impl Logger for StderrLogger {
    fn log(&self, level: LogLevel, message: &str, fields: &[(&str, String)]) {
        if level < self.min_level {
            return;
        }
        let rendered = fields
            .iter()
            .map(|(k, v)| format!(" {k}={}", redact(k, v)))
            .collect::<Vec<_>>()
            .concat();
        eprintln!("[oblodai] {} {message}{rendered}", level.as_str());
    }
}

const SENSITIVE: [&str; 7] = [
    "secret",
    "signature",
    "passcode",
    "token",
    "authorization",
    "password",
    "claim_url",
];

/// Whether a field or header of this name carries a secret (never logged, never printed by
/// `Debug`). A payout link's `claim_url` embeds its claim token.
pub fn is_sensitive(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    SENSITIVE.iter().any(|s| lower.contains(s))
}

/// Replace the value of a sensitive-looking key.
pub fn redact<'a>(key: &str, value: &'a str) -> &'a str {
    if is_sensitive(key) {
        "[redacted]"
    } else {
        value
    }
}
