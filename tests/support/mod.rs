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

// --- a fake gateway on a real socket ------------------------------------------------------------

/// One request the fake gateway received.
#[derive(Clone, Debug)]
pub struct Received {
    pub method: String,
    /// Path and query, as sent.
    pub target: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Received {
    pub fn path(&self) -> &str {
        self.target.split('?').next().unwrap_or_default()
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

type Answerer = dyn Fn(&Received) -> (u16, Vec<(String, String)>, Vec<u8>) + Send + Sync;

/// An HTTP/1.1 server on 127.0.0.1 that answers every request with `answer(request)` — what a
/// `Client::from_env()` pointed at `OBLODAI_BASE_URL` talks to in the README and example tests.
pub struct FakeGateway {
    pub base_url: String,
    received: Arc<Mutex<Vec<Received>>>,
}

impl FakeGateway {
    pub fn start(
        answer: impl Fn(&Received) -> (u16, Vec<(String, String)>, Vec<u8>) + Send + Sync + 'static,
    ) -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let received = Arc::new(Mutex::new(Vec::new()));
        let log = received.clone();
        let answer: Arc<Answerer> = Arc::new(answer);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { return };
                let (log, answer) = (log.clone(), answer.clone());
                std::thread::spawn(move || serve(stream, &log, &*answer));
            }
        });
        Self { base_url, received }
    }

    pub fn received(&self) -> Vec<Received> {
        self.received.lock().unwrap().clone()
    }
}

fn serve(mut stream: std::net::TcpStream, log: &Mutex<Vec<Received>>, answer: &Answerer) {
    use std::io::{BufRead, BufReader, Read, Write};
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let mut parts = line.split_whitespace();
        let (method, target) = (
            parts.next().unwrap_or_default().to_string(),
            parts.next().unwrap_or_default().to_string(),
        );
        let mut headers = Vec::new();
        let mut length = 0usize;
        loop {
            let mut header = String::new();
            reader.read_line(&mut header).unwrap();
            let header = header.trim_end();
            if header.is_empty() {
                break;
            }
            if let Some((k, v)) = header.split_once(':') {
                if k.eq_ignore_ascii_case("content-length") {
                    length = v.trim().parse().unwrap_or(0);
                }
                headers.push((k.trim().to_string(), v.trim().to_string()));
            }
        }
        let mut body = vec![0u8; length];
        reader.read_exact(&mut body).unwrap();
        let request = Received {
            method,
            target,
            headers,
            body,
        };
        let (status, extra, bytes) = answer(&request);
        log.lock().unwrap().push(request);
        let mut head = format!("HTTP/1.1 {status} X\r\nContent-Length: {}\r\n", bytes.len());
        for (k, v) in extra {
            head.push_str(&format!("{k}: {v}\r\n"));
        }
        head.push_str("\r\n");
        if stream.write_all(head.as_bytes()).is_err() || stream.write_all(&bytes).is_err() {
            return;
        }
    }
}

/// The smallest answer a model accepts (its `Default`), with some fields set.
pub fn model<T: Default + serde::Serialize>(fields: Value) -> Value {
    let mut value = serde_json::to_value(T::default()).unwrap();
    if let Some(map) = fields.as_object() {
        for (k, v) in map {
            value[k] = v.clone();
        }
    }
    value
}

/// A `{state:0,result}` answer for the fake gateway.
pub fn envelope(result: Value) -> (u16, Vec<(String, String)>, Vec<u8>) {
    (
        200,
        vec![("Content-Type".into(), "application/json".into())],
        json!({"state": 0, "result": result})
            .to_string()
            .into_bytes(),
    )
}
