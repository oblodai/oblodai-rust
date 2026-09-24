//! Test scaffolding: a recording HTTP backend and recorded webhook deliveries.

#![allow(dead_code)]
// The SDK's error carries the whole API envelope by value; boxing it here would only make the
// scaffolding disagree with the crate it exercises.
#![allow(clippy::result_large_err)]

use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use oblodai::core::engine::RawResponse;
use oblodai::{BackendFuture, Error, HttpBackend, HttpRequest, Result};
use serde_json::{json, Value};

/// One request the SDK made.
#[derive(Clone, Debug)]
pub struct Recorded {
    pub url: String,
    pub method: String,
    /// Header names are lower-cased.
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub timeout: Duration,
}

impl Recorded {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    pub fn json_body(&self) -> Value {
        serde_json::from_str(self.body.as_deref().unwrap_or("null")).unwrap_or(Value::Null)
    }

    pub fn path(&self) -> String {
        url::Url::parse(&self.url).unwrap().path().to_string()
    }

    pub fn query(&self, name: &str) -> Option<String> {
        url::Url::parse(&self.url)
            .unwrap()
            .query_pairs()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.into_owned())
    }
}

/// One scripted answer.
#[derive(Clone, Debug, Default)]
pub struct Scripted {
    pub status: u16,
    pub body: String,
    pub headers: Vec<(String, String)>,
    /// Fail instead of answering (a network error).
    pub error: Option<(String, String)>,
    /// Answer only after this long — used to trip the per-attempt timeout.
    pub delay: Option<Duration>,
}

/// `{state:0,result}` with this payload.
pub fn ok(result: Value) -> Scripted {
    Scripted {
        status: 200,
        body: json!({ "state": 0, "result": result }).to_string(),
        headers: vec![("content-type".into(), "application/json".into())],
        ..Default::default()
    }
}

/// `{error:{…}}` with this status.
pub fn api_error(status: u16, error: Value) -> Scripted {
    Scripted {
        status,
        body: json!({ "error": error }).to_string(),
        headers: vec![("content-type".into(), "application/json".into())],
        ..Default::default()
    }
}

/// An answer with no Oblodai envelope at all — what a proxy or load balancer sends.
pub fn no_envelope(status: u16) -> Scripted {
    Scripted {
        status,
        body: "<html>upstream error</html>".into(),
        headers: vec![("content-type".into(), "text/html".into())],
        ..Default::default()
    }
}

/// A network failure before any answer.
pub fn network_error() -> Scripted {
    Scripted {
        error: Some(("transport.network".into(), "connection reset".into())),
        ..Default::default()
    }
}

impl Scripted {
    pub fn header(mut self, name: &str, value: impl Into<String>) -> Self {
        self.headers.push((name.to_string(), value.into()));
        self
    }

    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }
}

/// Replays scripted answers in order and records every request it saw.
#[derive(Debug)]
pub struct MockBackend {
    script: Mutex<VecDeque<Scripted>>,
    calls: Mutex<Vec<Recorded>>,
}

impl MockBackend {
    pub fn new(script: Vec<Scripted>) -> Arc<Self> {
        Arc::new(Self {
            script: Mutex::new(script.into()),
            calls: Mutex::new(Vec::new()),
        })
    }

    pub fn calls(&self) -> Vec<Recorded> {
        self.calls.lock().unwrap().clone()
    }

    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }

    pub fn last(&self) -> Recorded {
        self.calls
            .lock()
            .unwrap()
            .last()
            .cloned()
            .expect("no request was made")
    }

    pub fn first(&self) -> Recorded {
        self.calls
            .lock()
            .unwrap()
            .first()
            .cloned()
            .expect("no request was made")
    }

    fn record(&self, request: &HttpRequest) -> std::result::Result<Scripted, Error> {
        self.calls.lock().unwrap().push(Recorded {
            url: request.url.clone(),
            method: request.method.to_string(),
            headers: request
                .headers
                .iter()
                .map(|(k, v)| (k.to_ascii_lowercase(), v.clone()))
                .collect(),
            body: request.body.clone(),
            timeout: request.timeout,
        });
        self.script
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| Error::transport("transport.network", "mock: no scripted answer left"))
    }
}

fn answer(next: Scripted) -> Result<RawResponse> {
    if let Some((code, message)) = next.error {
        return Err(Error::transport(&code, message));
    }
    Ok(RawResponse {
        status: if next.status == 0 { 200 } else { next.status },
        headers: next.headers,
        body: next.body.into_bytes(),
    })
}

impl HttpBackend for MockBackend {
    fn send(&self, request: HttpRequest) -> BackendFuture<'_> {
        let timeout = request.timeout;
        let next = self.record(&request);
        Box::pin(async move {
            let next = next?;
            if let Some(delay) = next.delay {
                // The backend owns the per-attempt timeout, exactly as the real one does.
                if tokio::time::timeout(timeout, tokio::time::sleep(delay))
                    .await
                    .is_err()
                {
                    return Err(Error::transport(
                        "transport.timeout",
                        format!("request timed out after {} ms", timeout.as_millis()),
                    ));
                }
            }
            answer(next)
        })
    }
}

#[cfg(feature = "blocking")]
impl oblodai::BlockingHttpBackend for MockBackend {
    fn send(&self, request: HttpRequest) -> Result<RawResponse> {
        let timeout = request.timeout;
        let next = self.record(&request)?;
        if let Some(delay) = next.delay {
            if delay >= timeout {
                std::thread::sleep(timeout);
                return Err(Error::transport(
                    "transport.timeout",
                    format!("request timed out after {} ms", timeout.as_millis()),
                ));
            }
            std::thread::sleep(delay);
        }
        answer(next)
    }
}

// --- recorded webhook deliveries -----------------------------------------------------------

/// A real signed delivery: headers, the parsed body and the exact bytes that were signed.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct WebhookSample {
    pub headers: std::collections::BTreeMap<String, String>,
    pub body: Value,
    #[serde(default)]
    pub raw: Option<String>,
}

impl WebhookSample {
    pub fn raw_bytes(&self) -> Vec<u8> {
        match &self.raw {
            Some(raw) => raw.clone().into_bytes(),
            None => self.body.to_string().into_bytes(),
        }
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

pub fn load_webhook_samples() -> Vec<WebhookSample> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/webhook-samples.json");
    let text = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// Walk into a JSON value along a dotted path (`items.0.result`).
pub fn pick<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for step in path.split('.').filter(|s| !s.is_empty()) {
        current = match step.parse::<usize>() {
            Ok(i) => current.get(i)?,
            Err(_) => current.get(step)?,
        };
    }
    Some(current)
}

/// A real recorded answer (`tests/fixtures/<name>.json`): `payment` is an invoice as
/// `payments().list_history` returns it, `payout` a payout as `payouts().list_history` does.
pub fn sample(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/{name}.json"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
