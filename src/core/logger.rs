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

const SENSITIVE: [&str; 12] = [
    "secret",
    "signature",
    "passcode",
    "token",
    "authorization",
    "password",
    "claim_url",
    "claimurl",
    "device_code",
    "api_key",
    "api-key",
    "cookie",
];

/// Query parameters of a signed link (a document URL's `exp`/`sig`) and of bearer URLs.
const SENSITIVE_QUERY: [&str; 6] = ["sig", "exp", "token", "code", "passcode", "signature"];

/// Path parameters that carry a bearer secret (`/v1/claim/{token}`, `/v1/aml/{token}`).
const SENSITIVE_PATH_PARAMS: [&str; 3] = ["token", "code", "passcode"];

/// Path segments whose NEXT segment is a bearer secret, for URLs seen without their route.
const SECRET_AFTER_SEGMENT: [&str; 2] = ["claim", "aml"];

/// A copy of `url` safe to log or show: no userinfo, the bearer path segments of claim/AML links
/// (and every `{token}`/`{code}`/`{passcode}` segment of `route_path`, when given) and the
/// signed-link query parameters (`sig`, `exp`, `token`, …) replaced by `[redacted]`.
pub fn redact_url(url: &str, route_path: Option<&str>) -> String {
    let Ok(mut parsed) = url::Url::parse(url) else {
        return "[unparseable url]".to_string();
    };
    let _ = parsed.set_username("");
    let _ = parsed.set_password(None);
    let mut segments: Vec<String> = parsed.path().split('/').map(str::to_string).collect();
    let mut redact = vec![false; segments.len()];
    for i in 1..segments.len() {
        if SECRET_AFTER_SEGMENT.contains(&segments[i - 1].as_str()) {
            redact[i] = true;
        }
    }
    if let Some(template) = route_path {
        let tpl: Vec<&str> = template.split('/').collect();
        if tpl.len() <= segments.len() {
            let offset = segments.len() - tpl.len();
            for (j, part) in tpl.iter().enumerate() {
                if let Some(name) = part.strip_prefix('{').and_then(|p| p.strip_suffix('}')) {
                    if SENSITIVE_PATH_PARAMS.contains(&name) || is_sensitive(name) {
                        redact[offset + j] = true;
                    }
                }
            }
        }
    }
    for (segment, hide) in segments.iter_mut().zip(redact) {
        if hide && !segment.is_empty() {
            *segment = "[redacted]".to_string();
        }
    }
    parsed.set_path(&segments.join("/"));
    if parsed.query().is_some() {
        let pairs: Vec<(String, String)> = parsed
            .query_pairs()
            .map(|(k, v)| {
                let hide =
                    SENSITIVE_QUERY.contains(&k.to_ascii_lowercase().as_str()) || is_sensitive(&k);
                (
                    k.into_owned(),
                    if hide {
                        "[redacted]".to_string()
                    } else {
                        v.into_owned()
                    },
                )
            })
            .collect();
        parsed.query_pairs_mut().clear().extend_pairs(pairs);
    }
    parsed.to_string()
}

/// Whether a field or header of this name carries a secret (never logged, never printed by
/// `Debug`). A payout link's `claim_url` embeds its claim token. The signature headers count by the
/// contract's names for them, whatever words those names hold.
pub fn is_sensitive(key: &str) -> bool {
    use crate::generated::signing::{
        HEADER_SIGNATURE, HEADER_WEBHOOK_SIGNATURE, HEADER_WEBHOOK_SIGNATURE_PREV,
    };
    if [
        HEADER_SIGNATURE,
        HEADER_WEBHOOK_SIGNATURE,
        HEADER_WEBHOOK_SIGNATURE_PREV,
    ]
    .iter()
    .any(|h| key.eq_ignore_ascii_case(h))
    {
        return true;
    }
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
