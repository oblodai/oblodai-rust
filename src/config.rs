//! Client configuration: explicit options first, the environment second, validated before the
//! first request leaves the process.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use url::Url;

use crate::core::clock::{Clock, SkewCorrectingClock, SystemClock};
use crate::core::engine::Core;
use crate::core::hooks::Hooks;
use crate::core::http::HttpBackend;
use crate::core::logger::{LogLevel, Logger, NoopLogger, StderrLogger};
use crate::core::request::Credentials;
use crate::core::retry::RetryOptions;
use crate::error::{Error, Result};

/// The production API.
pub const DEFAULT_BASE_URL: &str = "https://api.oblodai.com";
/// This SDK's version, as sent in `User-Agent`.
pub const SDK_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Builds a [`Client`](crate::Client). Every field falls back to an `OBLODAI_*` environment
/// variable, so a deployment can be configured without touching the code.
#[derive(Default)]
pub struct ClientBuilder {
    public_id: Option<String>,
    secret: Option<String>,
    base_url: Option<String>,
    timeout: Option<Duration>,
    deadline: Option<Duration>,
    retry: Option<RetryOptions>,
    max_retries: Option<u32>,
    hooks: Hooks,
    logger: Option<Arc<dyn Logger>>,
    headers: Vec<(String, String)>,
    admin_token: Option<String>,
    allow_insecure_base_url: Option<bool>,
    clock: Option<Arc<dyn Clock>>,
    env: Option<HashMap<String, String>>,
    pub(crate) backend: Option<Arc<dyn HttpBackend>>,
    #[cfg(feature = "blocking")]
    pub(crate) blocking_backend: Option<Arc<dyn crate::core::http::BlockingHttpBackend>>,
}

impl std::fmt::Debug for ClientBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientBuilder")
            .field("public_id", &self.public_id)
            .field("base_url", &self.base_url)
            .finish_non_exhaustive()
    }
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Public id of the merchant's one API key (`X-Public-Id`). Falls back to
    /// `OBLODAI_PUBLIC_ID`.
    pub fn public_id(mut self, value: impl Into<String>) -> Self {
        self.public_id = Some(value.into());
        self
    }

    /// Secret of the merchant's one API key. Falls back to `OBLODAI_SECRET`.
    pub fn secret(mut self, value: impl Into<String>) -> Self {
        self.secret = Some(value.into());
        self
    }

    /// API origin. Falls back to `OBLODAI_BASE_URL`, then <https://api.oblodai.com>. A path prefix
    /// (`https://gw.corp/oblodai`) is kept and every route is appended to it.
    pub fn base_url(mut self, value: impl Into<String>) -> Self {
        self.base_url = Some(value.into());
        self
    }

    /// Per-attempt timeout. Default 30 s.
    pub fn timeout(mut self, value: Duration) -> Self {
        self.timeout = Some(value);
        self
    }

    /// Overall budget per call, retries included. Default 90 s.
    pub fn deadline(mut self, value: Duration) -> Self {
        self.deadline = Some(value);
        self
    }

    /// Retry policy. `RetryOptions { max_retries: 0, .. }` disables retries.
    pub fn retry(mut self, value: RetryOptions) -> Self {
        self.retry = Some(value);
        self
    }

    /// Retries after the first attempt (default 2); `0` disables retries. Overrides
    /// `RetryOptions::max_retries`; a call can override it again with `.max_retries(..)`.
    pub fn max_retries(mut self, value: u32) -> Self {
        self.max_retries = Some(value);
        self
    }

    /// Request and response hooks, called once per attempt (metrics, tracing, logs).
    pub fn hooks(mut self, value: Hooks) -> Self {
        self.hooks = value;
        self
    }

    /// Structured logger. `OBLODAI_LOG=debug` installs a stderr logger when none is given.
    pub fn logger(mut self, value: Arc<dyn Logger>) -> Self {
        self.logger = Some(value);
        self
    }

    /// An extra header on every request. Headers the SDK signs are never overridden.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Admin token of a self-hosted gateway; only the merchant-provisioning routes use it.
    /// Falls back to `OBLODAI_ADMIN_TOKEN`.
    pub fn admin_token(mut self, value: impl Into<String>) -> Self {
        self.admin_token = Some(value.into());
        self
    }

    /// Permit plain `http://` base URLs beyond loopback (a local core, CI). Falls back to
    /// `OBLODAI_ALLOW_INSECURE=1`.
    pub fn allow_insecure_base_url(mut self, value: bool) -> Self {
        self.allow_insecure_base_url = Some(value);
        self
    }

    /// Replace the async HTTP layer (a proxy-aware client, a recording stub in tests).
    pub fn http_backend(mut self, backend: Arc<dyn HttpBackend>) -> Self {
        self.backend = Some(backend);
        self
    }

    /// Replace the synchronous HTTP layer used by [`blocking::Client`](crate::blocking::Client).
    #[cfg(feature = "blocking")]
    pub fn blocking_http_backend(
        mut self,
        backend: Arc<dyn crate::core::http::BlockingHttpBackend>,
    ) -> Self {
        self.blocking_backend = Some(backend);
        self
    }

    /// Source the signing clock from somewhere other than the system clock (tests).
    pub fn clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = Some(clock);
        self
    }

    /// Read the fallbacks from this map instead of the process environment.
    pub fn env<I, K, V>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.env = Some(
            vars.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        );
        self
    }

    fn var(&self, name: &str) -> Option<String> {
        match &self.env {
            Some(map) => map.get(name).cloned(),
            None => std::env::var(name).ok(),
        }
        .filter(|v| !v.is_empty())
    }

    /// Resolve options against the environment and validate what can be validated up front.
    pub(crate) fn resolve(&self) -> Result<Core> {
        let base_url = self
            .base_url
            .clone()
            .or_else(|| self.var("OBLODAI_BASE_URL"))
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
            .trim_end_matches('/')
            .to_string();
        let allow_insecure = self
            .allow_insecure_base_url
            .unwrap_or_else(|| self.var("OBLODAI_ALLOW_INSECURE").as_deref() == Some("1"));
        assert_base_url(&base_url, allow_insecure)?;

        let public_id = self
            .public_id
            .clone()
            .or_else(|| self.var("OBLODAI_PUBLIC_ID"));
        let secret = self.secret.clone().or_else(|| self.var("OBLODAI_SECRET"));
        let credentials = pair(public_id, secret)?;

        let logger: Arc<dyn Logger> = match &self.logger {
            Some(l) => l.clone(),
            None => match self.var("OBLODAI_LOG").as_deref().and_then(LogLevel::parse) {
                Some(level) => Arc::new(StderrLogger::new(level)),
                None => Arc::new(NoopLogger),
            },
        };

        let user_agent = format!("oblodai-rust/{SDK_VERSION}");
        let mut core = Core::new(base_url, user_agent);
        core.credentials = credentials;
        core.logger = logger;
        core.headers = self.headers.clone();
        core.admin_token = self
            .admin_token
            .clone()
            .or_else(|| self.var("OBLODAI_ADMIN_TOKEN"));
        if let Some(t) = self.timeout {
            core.timeout = t;
        }
        if let Some(d) = self.deadline {
            core.deadline = d;
        }
        if let Some(r) = self.retry {
            core.retry = r;
        }
        if let Some(n) = self.max_retries {
            core.retry.max_retries = n;
        }
        core.hooks = self.hooks.clone();
        core.clock = Arc::new(SkewCorrectingClock::new(
            self.clock
                .clone()
                .unwrap_or_else(|| Arc::new(SystemClock) as Arc<dyn Clock>),
        ));
        Ok(core)
    }
}

/// Settings a copy of a client can change: [`Client::with_options`](crate::Client::with_options).
///
/// Unset fields keep the original client's value.
#[derive(Clone, Debug, Default)]
pub struct ClientOptions {
    timeout: Option<Duration>,
    deadline: Option<Duration>,
    max_retries: Option<u32>,
    headers: Vec<(String, String)>,
    hooks: Option<Hooks>,
}

impl ClientOptions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Per-attempt timeout.
    pub fn timeout(mut self, value: Duration) -> Self {
        self.timeout = Some(value);
        self
    }

    /// Overall budget per call, retries included.
    pub fn deadline(mut self, value: Duration) -> Self {
        self.deadline = Some(value);
        self
    }

    /// Retries after the first attempt; `0` disables retries.
    pub fn max_retries(mut self, value: u32) -> Self {
        self.max_retries = Some(value);
        self
    }

    /// An extra header on every request of the copy, on top of the original's.
    pub fn extra_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Replace the hooks.
    pub fn hooks(mut self, value: Hooks) -> Self {
        self.hooks = Some(value);
        self
    }

    pub(crate) fn apply(&self, core: &mut Core) {
        if let Some(t) = self.timeout {
            core.timeout = t;
        }
        if let Some(d) = self.deadline {
            core.deadline = d;
        }
        if let Some(n) = self.max_retries {
            core.retry.max_retries = n;
        }
        core.headers.extend(self.headers.iter().cloned());
        if let Some(h) = &self.hooks {
            core.hooks = h.clone();
        }
    }
}

fn pair(public_id: Option<String>, secret: Option<String>) -> Result<Option<Credentials>> {
    match (public_id, secret) {
        (Some(public_id), Some(secret)) => Ok(Some(Credentials { public_id, secret })),
        (None, None) => Ok(None),
        _ => Err(Error::config(
            "sdk.bad_config",
            "public_id and secret must be provided together",
            Some("public_id"),
        )),
    }
}

fn assert_base_url(base_url: &str, allow_insecure: bool) -> Result<()> {
    let parsed = Url::parse(base_url).map_err(|_| {
        Error::config(
            "sdk.bad_config",
            format!("base_url is not a valid URL: {base_url}"),
            Some("base_url"),
        )
    })?;
    if parsed.scheme() == "https" {
        return Ok(());
    }
    let host = parsed.host_str().unwrap_or_default();
    let local = host == "localhost" || host == "127.0.0.1" || host == "[::1]" || host == "::1";
    if parsed.scheme() == "http" && (allow_insecure || local) {
        return Ok(());
    }
    Err(Error::config(
        "sdk.bad_config",
        format!(
            "base_url must use https (got {}://{host}); call allow_insecure_base_url(true) for a \
             local core",
            parsed.scheme()
        ),
        Some("base_url"),
    ))
}
