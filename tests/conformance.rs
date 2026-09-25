//! The shared conformance suite every Oblodai SDK runs (backend `tools/sdkgen/conformance`).
//!
//! Scenarios are read from `$SDKGEN_CONFORMANCE`, else from `tools/sdkgen/conformance` of the
//! backend checkout the drift check uses (`$OBLODAI_BACKEND`, else `../oblodai-backend`). Signing
//! vectors are not in the scenario files: each suite names the backend `openapi.json` and a pointer
//! into its `x-oblodai-signing`, and the vectors are read from there.
//!
//! Every call scenario runs on the async client (tokio's clock paused, so a retry pause is measured
//! exactly and costs no time) and on the blocking client (real pauses, measured with a tolerance),
//! both over a scripted HTTP backend.

#![cfg(feature = "blocking")]
#![allow(clippy::result_large_err)]

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use oblodai::core::engine::{CallOptions, RawResponse};
use oblodai::models::{LookupRequest, PaymentRequest};
use oblodai::webhooks::{
    is_known_event, parse_webhook, verify_webhook, verify_webhook_delivery, Headers, VerifyOptions,
    WEBHOOK_EVENTS,
};
use oblodai::WebhookEvent;
use oblodai::{
    canonical_string, from_json, sign_request, sign_webhook, BackendFuture, BlockingHttpBackend,
    Clock, Error, HttpBackend, HttpRequest, Method, RetryOptions, SignInput,
};
use serde_json::Value;

// --- where the suite lives --------------------------------------------------------------------

fn backend_root() -> PathBuf {
    match std::env::var_os("OBLODAI_BACKEND") {
        Some(dir) => PathBuf::from(dir),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("../oblodai-backend"),
    }
}

/// The suite directory, or `None` (skip, loudly) when no backend checkout carries one. With
/// `SDKGEN_CONFORMANCE` or `OBLODAI_BACKEND` set, a missing suite is a failure, not a skip.
fn conformance_dir() -> Option<PathBuf> {
    let explicit = std::env::var_os("SDKGEN_CONFORMANCE").map(PathBuf::from);
    let configured = explicit.is_some() || std::env::var_os("OBLODAI_BACKEND").is_some();
    let dir = explicit.unwrap_or_else(|| backend_root().join("tools/sdkgen/conformance"));
    if dir.is_dir() {
        return Some(dir);
    }
    if configured {
        panic!("conformance suite not found at {}", dir.display());
    }
    eprintln!(
        "conformance: skipped — no suite at {}; set OBLODAI_BACKEND or SDKGEN_CONFORMANCE",
        dir.display()
    );
    None
}

fn read_json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn suite(dir: &Path, name: &str) -> Value {
    read_json(&dir.join(format!("{name}.json")))
}

/// The spec's `x-oblodai-signing` and the vectors the suite points at.
fn source(dir: &Path, suite: &Value) -> (Value, Vec<Value>) {
    let src = &suite["source"];
    let spec = read_json(&dir.join(src["spec"].as_str().unwrap()));
    let vectors = spec
        .pointer(src["pointer"].as_str().unwrap())
        .and_then(Value::as_array)
        .cloned()
        .expect("vectors at the suite's pointer");
    assert!(!vectors.is_empty(), "no vectors at {}", src["pointer"]);
    (spec["x-oblodai-signing"].clone(), vectors)
}

/// Role → header name, read from the spec where the suite's `header_names` points: the suite never
/// names a header, so an SDK still sending a renamed one fails here.
fn header_names(dir: &Path, suite: &Value) -> HashMap<String, String> {
    let spec = read_json(&dir.join(suite["source"]["spec"].as_str().unwrap()));
    let at = &suite["header_names"];
    let names = spec
        .pointer(at["pointer"].as_str().unwrap())
        .and_then(Value::as_array)
        .expect("header names at the suite's pointer");
    let roles = at["roles"].as_array().unwrap();
    assert_eq!(names.len(), roles.len(), "header_names: one name per role");
    roles
        .iter()
        .zip(names)
        .map(|(r, n)| (r.as_str().unwrap().into(), n.as_str().unwrap().into()))
        .collect()
}

/// The rehearsal header name the spec gives (`header_names.test_pointer`): again the spec's name,
/// not the SDK's constant.
fn test_header(dir: &Path, suite: &Value) -> String {
    let spec = read_json(&dir.join(suite["source"]["spec"].as_str().unwrap()));
    let name = spec
        .pointer(
            suite["header_names"]["test_pointer"]
                .as_str()
                .expect("header_names.test_pointer"),
        )
        .and_then(Value::as_str)
        .expect("rehearsal header name at header_names.test_pointer");
    assert!(!name.is_empty(), "empty rehearsal header name");
    name.to_string()
}

// --- signing ----------------------------------------------------------------------------------

#[test]
fn request_signing_matches_the_core() {
    let Some(dir) = conformance_dir() else { return };
    let suite = suite(&dir, "signing");
    let (_, vectors) = source(&dir, &suite);
    let names = header_names(&dir, &suite);
    let mut ran = 0;
    for check in suite["checks"].as_array().unwrap() {
        for (i, v) in vectors.iter().enumerate() {
            let key = v["idempotency_key"].as_str().unwrap_or("");
            let input = SignInput {
                ts: v["ts"].as_i64().unwrap(),
                method: v["method"].as_str().unwrap(),
                request_uri: v["request_uri"].as_str().unwrap(),
                idempotency_key: (!key.is_empty()).then_some(key),
                body: v["body"].as_str().unwrap_or("").as_bytes(),
            };
            let name = format!("{}#{i}", check["name"].as_str().unwrap());
            match check["kind"].as_str().unwrap() {
                "request_canonical" => {
                    assert_eq!(canonical_string(&input), v["canonical"], "{name}")
                }
                "request_signature" => assert_eq!(
                    sign_request(v["secret"].as_str().unwrap(), &input),
                    v["signature"],
                    "{name}"
                ),
                "request_headers" => {
                    request_carries_spec_headers(&name, check, v, &names);
                }
                other => panic!("unknown signing check kind {other:?}"),
            }
            ran += 1;
        }
    }
    assert!(ran > 0);
}

/// A fixed clock: the vector's `ts`.
struct At(i64);

impl Clock for At {
    fn now(&self) -> i64 {
        self.0
    }
}

/// Records the one request it is sent and answers it with an empty success.
#[derive(Default)]
struct Capture(Mutex<Vec<HttpRequest>>);

impl BlockingHttpBackend for Capture {
    fn send(&self, request: HttpRequest) -> Result<RawResponse, Error> {
        self.0.lock().unwrap().push(request);
        Ok(RawResponse {
            status: 200,
            headers: vec![("content-type".into(), "application/json".into())],
            body: br#"{"state":0,"result":{}}"#.to_vec(),
        })
    }
}

/// request_headers: the vector's request goes through the client's own signing transport (key of
/// `public_id` + the vector's secret, clock at the vector's `ts`); the request that reaches the
/// wire carries the spec's header names with the vector's values.
fn request_carries_spec_headers(
    name: &str,
    check: &Value,
    v: &Value,
    names: &HashMap<String, String>,
) {
    let public_id = check["public_id"].as_str().expect("public_id");
    let ts = v["ts"].as_i64().unwrap();
    let key = v["idempotency_key"].as_str().unwrap_or("");
    let body = v["body"].as_str().unwrap_or("");
    let (path, query) = v["request_uri"]
        .as_str()
        .unwrap()
        .split_once('?')
        .unwrap_or((v["request_uri"].as_str().unwrap(), ""));
    // A route of the vector's shape: a copy of a generated one with the vector's method and path.
    let mut route = *oblodai::routes::ROUTES[0];
    route.method = match v["method"].as_str().unwrap() {
        "GET" => Method::Get,
        "POST" => Method::Post,
        other => panic!("{name}: method {other} is not wired"),
    };
    route.path = Box::leak(path.to_string().into_boxed_str());
    route.auth = oblodai::RouteAuth::Key;
    route.idempotent = !key.is_empty();
    route.safe = false;
    route.bare = false;
    route.list_kind = None;
    let route: &'static oblodai::RouteSpec = Box::leak(Box::new(route));

    let capture = Arc::new(Capture::default());
    let client = oblodai::Client::builder()
        .public_id(public_id)
        .secret(v["secret"].as_str().unwrap())
        .base_url("https://api.test")
        .clock(Arc::new(At(ts)))
        .env(Vec::<(String, String)>::new())
        .blocking_http_backend(capture.clone() as Arc<dyn BlockingHttpBackend>)
        .build_blocking()
        .unwrap();
    let opts = CallOptions {
        body: (!body.is_empty()).then(|| serde_json::from_str(body).unwrap()),
        query: query
            .split('&')
            .filter(|p| !p.is_empty())
            .map(|p| {
                let (k, v) = p.split_once('=').unwrap_or((p, ""));
                (k.to_string(), v.to_string())
            })
            .collect(),
        idempotency_key: (!key.is_empty()).then(|| key.to_string()),
        max_retries: Some(0),
        ..CallOptions::default()
    };
    client
        .transport()
        .call_value(route, opts)
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    let sent = capture.0.lock().unwrap();
    assert_eq!(sent.len(), 1, "{name}");
    let req = &sent[0];
    // The request is the vector's, byte for byte — else the headers below prove nothing.
    assert_eq!(
        req.url,
        format!("https://api.test{}", v["request_uri"].as_str().unwrap()),
        "{name}"
    );
    assert_eq!(req.body.as_deref().unwrap_or(""), body, "{name}: body");
    let want = [
        ("public_id", Some(public_id.to_string())),
        (
            "signature",
            Some(v["signature"].as_str().unwrap().to_string()),
        ),
        ("timestamp", Some(ts.to_string())),
        (
            "idempotency_key",
            (!key.is_empty()).then(|| key.to_string()),
        ),
    ];
    for (role, value) in want {
        let header_name = &names[role];
        assert_eq!(
            header(req, header_name).map(str::to_string),
            value,
            "{name}: {header_name} (role {role})"
        );
    }
}

#[test]
fn webhooks_verify_like_the_core() {
    let Some(dir) = conformance_dir() else { return };
    let suite = suite(&dir, "webhook");
    let (signing, vectors) = source(&dir, &suite);
    let names = header_names(&dir, &suite);
    let skew = signing["skew_seconds"].as_i64().unwrap();
    for check in suite["checks"].as_array().unwrap() {
        for (i, v) in vectors.iter().enumerate() {
            let name = format!("{}#{i}", check["name"].as_str().unwrap());
            let secret = v["secret"].as_str().unwrap();
            let ts = v["ts"].as_i64().unwrap();
            let mut payload = v["payload"].as_str().unwrap().to_string();
            let mut signature = v["signature"].as_str().unwrap().to_string();
            match check["kind"].as_str().unwrap() {
                "webhook_signature" => {
                    assert_eq!(
                        sign_webhook(secret, ts, payload.as_bytes()),
                        signature,
                        "{name}"
                    );
                    continue;
                }
                "webhook_verify" => {}
                other => panic!("unknown webhook check kind {other:?}"),
            }
            let offset = match &check["now_from_ts"] {
                Value::String(s) if s == "skew" => skew,
                Value::String(s) if s == "skew+1" => skew + 1,
                n => n.as_i64().expect("now_from_ts"),
            };
            match check["mutate"].as_str().unwrap() {
                "payload" => payload.push(' '),
                "signature" => {
                    let first = if signature.starts_with('0') { "1" } else { "0" };
                    signature.replace_range(0..1, first);
                }
                "none" => {}
                other => panic!("unknown mutation {other:?}"),
            }
            let headers = Headers::from_pairs([
                (names["timestamp"].clone(), ts.to_string()),
                (names["signature"].clone(), signature),
            ]);
            let options = VerifyOptions::new(secret)
                .tolerance_seconds(skew)
                .now(ts + offset);
            let got = verify_webhook(payload.as_bytes(), &headers, &options);
            match check["expect"].as_str().unwrap() {
                // The vectors sign bare payloads, not whole events: verification gets past the MAC
                // and the freshness window and only then may refuse to parse — still a pass.
                "ok" => {
                    if let Err(err) = got {
                        assert_eq!(err.code(), "webhook.bad_payload", "{name}: {err}");
                    }
                }
                want => {
                    let err = got.expect_err(&name);
                    assert_eq!(err.code(), format!("webhook.{want}"), "{name}");
                }
            }
        }
    }
}

/// forward_compat webhooks: the body parses, keeps its raw type, and is known exactly as said.
#[test]
fn webhook_bodies_parse_with_their_raw_kind() {
    let Some(dir) = conformance_dir() else { return };
    let bodies = suite(&dir, "forward_compat")["webhooks"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        !bodies.is_empty(),
        "forward_compat.json has no webhook bodies"
    );
    for case in bodies {
        let name = case["name"].as_str().unwrap();
        let raw = serde_json::to_vec(&case["body"]).unwrap();
        let event = parse_webhook(&raw).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            event.event_kind(),
            case["expect"]["type"].as_str().unwrap(),
            "{name}"
        );
        assert_eq!(
            is_known_event(&event),
            case["expect"]["known"].as_bool().unwrap(),
            "{name}"
        );
    }
}

#[test]
fn webhook_deliveries_parse_and_expose_every_header() {
    let Some(dir) = conformance_dir() else { return };
    let suite = suite(&dir, "webhook_delivery");
    let (_, deliveries) = source(&dir, &suite);
    let names = header_names(&dir, &suite);
    let test_header = test_header(&dir, &suite);
    let mut events: Vec<&str> = deliveries
        .iter()
        .map(|d| d["event"].as_str().unwrap())
        .collect();
    events.sort_unstable();
    let mut known: Vec<&str> = WEBHOOK_EVENTS.iter().map(|(e, _)| *e).collect();
    known.sort_unstable();
    assert_eq!(
        events, known,
        "a delivery of every event this release knows"
    );

    let mut ran = 0;
    for check in suite["checks"].as_array().unwrap() {
        assert_eq!(check["kind"], "webhook_delivery", "unknown check kind");
        let key = match check["key"].as_str().unwrap() {
            "current" => "secret",
            "previous" => "previous_secret",
            other => panic!("unknown key {other:?}"),
        };
        let rehearsal = check["test"].as_bool().unwrap_or(false);
        for d in &deliveries {
            let name = format!("{} — {}", check["name"].as_str().unwrap(), d["event"]);
            let headers = Headers::from_pairs(
                d["headers"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
                    .chain(rehearsal.then(|| (test_header.clone(), "true".to_string()))),
            );
            let options =
                VerifyOptions::new(d[key].as_str().unwrap()).now(d["ts"].as_i64().unwrap());
            let delivery = verify_webhook_delivery(
                d["payload"].as_str().unwrap().as_bytes(),
                &headers,
                &options,
            )
            .unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(
                !matches!(delivery.event, WebhookEvent::Other(_)),
                "{name}: parsed as an unknown kind"
            );
            assert_eq!(
                delivery.event.event_kind(),
                d["kind"].as_str().unwrap(),
                "{name}"
            );
            assert!(
                !delivery.event.object_id().is_empty(),
                "{name}: no object id"
            );
            assert_eq!(
                delivery.is_test, rehearsal,
                "{name}: is_test (rehearsal header {test_header})"
            );
            let fields = suite["fields"].as_object().expect("fields by role");
            assert_eq!(fields.len(), names.len(), "a field for every header role");
            for (role, field) in fields {
                let header = &names[role];
                let want = d["headers"][header].as_str().unwrap();
                let got = match field.as_str().unwrap() {
                    "" => continue,
                    "id" => delivery.id.clone(),
                    "event_id" => delivery.event_id.clone(),
                    "event_type" => delivery.event_type.clone(),
                    "event_time" => delivery.event_time.map(|v| v.to_string()),
                    "sent_at" => Some(delivery.sent_at.to_string()),
                    other => {
                        panic!("{name}: the delivery info has no field {other:?} for {header}")
                    }
                };
                assert_eq!(got.as_deref(), Some(want), "{name}: {field} ≠ {header}");
            }
            ran += 1;
        }
    }
    assert!(ran > 0);
}

// --- calls ------------------------------------------------------------------------------------

/// Replays the scenario's responses and records what the SDK sent and when.
struct Script {
    queue: Mutex<VecDeque<Value>>,
    requests: Mutex<Vec<HttpRequest>>,
    /// When each request arrived: tokio's (paused) clock on the async client, the wall clock on
    /// the blocking one.
    at: Mutex<Vec<Duration>>,
    origin_tokio: tokio::time::Instant,
    origin_std: std::time::Instant,
}

impl Script {
    fn new(responses: &Value) -> Arc<Self> {
        Arc::new(Self {
            queue: Mutex::new(responses.as_array().unwrap().iter().cloned().collect()),
            requests: Mutex::new(Vec::new()),
            at: Mutex::new(Vec::new()),
            origin_tokio: tokio::time::Instant::now(),
            origin_std: std::time::Instant::now(),
        })
    }

    fn answer(&self, request: HttpRequest, now: Duration) -> Result<RawResponse, Error> {
        let url = request.url.clone();
        self.requests.lock().unwrap().push(request);
        self.at.lock().unwrap().push(now);
        let next = self
            .queue
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| panic!("unscripted request {url}"));
        if next["transport_error"] == "timeout" {
            return Err(Error::transport("transport.timeout", "scripted timeout"));
        }
        let mut headers: Vec<(String, String)> = next["headers"]
            .as_object()
            .map(|h| {
                h.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let status = next["status"].as_u64().unwrap() as u16;
        let body = match next.get("json") {
            Some(json) => {
                headers.push(("content-type".into(), "application/json".into()));
                json.to_string().into_bytes()
            }
            None => {
                headers.push(("content-type".into(), "text/html".into()));
                b"<html>proxy</html>".to_vec()
            }
        };
        Ok(RawResponse {
            status,
            headers,
            body,
        })
    }

    fn delays_ms(&self) -> Vec<f64> {
        let at = self.at.lock().unwrap();
        at.windows(2)
            .map(|w| (w[1] - w[0]).as_secs_f64() * 1000.0)
            .collect()
    }
}

impl HttpBackend for Script {
    fn send(&self, request: HttpRequest) -> BackendFuture<'_> {
        let now = tokio::time::Instant::now() - self.origin_tokio;
        let answer = self.answer(request, now);
        Box::pin(async move { answer })
    }
}

impl BlockingHttpBackend for Script {
    fn send(&self, request: HttpRequest) -> Result<RawResponse, Error> {
        let now = self.origin_std.elapsed();
        self.answer(request, now)
    }
}

fn builder(script: &Arc<Script>) -> oblodai::ClientBuilder {
    oblodai::Client::builder()
        .public_id("oblodai_conformance")
        .secret("oblodai_test_conformance")
        .base_url("https://api.test")
        // The default policy, as a merchant gets it.
        .retry(RetryOptions::default())
        .env(Vec::<(String, String)>::new())
        .http_backend(script.clone() as Arc<dyn HttpBackend>)
        .blocking_http_backend(script.clone() as Arc<dyn BlockingHttpBackend>)
}

/// Checks that the call went to the operation the scenario named.
fn on(route: &oblodai::RouteSpec, op: &str) {
    assert_eq!(
        route.operation_id, op,
        "dispatch table points {op} elsewhere"
    );
}

fn to_json<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).unwrap()
}

/// `operationId` → the generated method, typed params built from the scenario's JSON args. A new
/// operation in the suite fails here loudly: wire it with one arm.
async fn call_async(client: &oblodai::Client, op: &str, args: Value) -> Result<Value, Error> {
    match op {
        "createPayment" => {
            let req = client.payments().create(from_json::<PaymentRequest>(args)?);
            on(req.route(), op);
            req.await.map(to_json)
        }
        "cancelPayment" => {
            let req = client.payments().cancel(from_json::<LookupRequest>(args)?);
            on(req.route(), op);
            req.await.map(to_json)
        }
        "getBalance" => {
            let req = client.account().get_balance();
            on(req.route(), op);
            req.await.map(to_json)
        }
        other => panic!("tests/conformance.rs: operation {other} is not wired yet"),
    }
}

fn call_blocking(
    client: &oblodai::blocking::Client,
    op: &str,
    args: Value,
) -> Result<Value, Error> {
    match op {
        "createPayment" => {
            let req = client.payments().create(from_json::<PaymentRequest>(args)?);
            on(req.route(), op);
            req.send().map(to_json)
        }
        "cancelPayment" => {
            let req = client.payments().cancel(from_json::<LookupRequest>(args)?);
            on(req.route(), op);
            req.send().map(to_json)
        }
        "getBalance" => {
            let req = client.account().get_balance();
            on(req.route(), op);
            req.send().map(to_json)
        }
        other => panic!("tests/conformance.rs: operation {other} is not wired yet"),
    }
}

fn header<'a>(request: &'a HttpRequest, name: &str) -> Option<&'a str> {
    request
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

/// The spec's name of the idempotency-key header (the signing suite's `header_names`).
fn idempotency_header(dir: &Path) -> String {
    header_names(dir, &suite(dir, "signing"))["idempotency_key"].clone()
}

fn check(
    idempotency_header: &str,
    name: &str,
    scenario: &Value,
    script: &Script,
    outcome: Result<Value, Error>,
    exact_delays: bool,
) {
    let expect = &scenario["expect"];
    let requests = script.requests.lock().unwrap().clone();
    let urls: Vec<&str> = requests.iter().map(|r| r.url.as_str()).collect();
    assert_eq!(
        requests.len() as u64,
        expect["requests"].as_u64().unwrap(),
        "{name}: requests {urls:?}"
    );
    let keys: Vec<Option<&str>> = requests
        .iter()
        .map(|r| header(r, idempotency_header))
        .collect();
    match expect["idempotency_key"].as_str() {
        Some("absent") => assert!(keys.iter().all(Option::is_none), "{name}: {keys:?}"),
        Some("present") => assert!(keys.iter().all(Option::is_some), "{name}: {keys:?}"),
        _ => {}
    }
    if expect["same_idempotency_key"] == true {
        assert!(
            keys[0].is_some() && keys.iter().all(|k| *k == keys[0]),
            "{name}: {keys:?}"
        );
    }
    if let Some(want) = expect["delays_ms"].as_array() {
        let got = script.delays_ms();
        assert_eq!(got.len(), want.len(), "{name}: delays {got:?}");
        for (g, w) in got.iter().zip(want) {
            let w = w.as_f64().unwrap();
            if exact_delays {
                assert_eq!(*g, w, "{name}: delays {got:?}");
            } else {
                assert!(*g >= w && *g < w + 500.0, "{name}: delays {got:?}");
            }
        }
    }
    if let Some(fields) = expect["request_body_field"].as_object() {
        let body: Value =
            serde_json::from_str(requests.last().unwrap().body.as_deref().unwrap()).unwrap();
        for (k, want) in fields {
            assert_eq!(&body[k], want, "{name}: request body {k}");
        }
    }
    if let Some(code) = expect["error_code"].as_str() {
        let err = outcome.expect_err(name);
        assert_eq!(err.code(), code, "{name}: {err}");
        return;
    }
    let result = outcome.unwrap_or_else(|e| panic!("{name}: {e}"));
    if let Some(fields) = expect["result_field"].as_object() {
        for (k, want) in fields {
            assert_eq!(&result[k], want, "{name}: result {k}");
        }
    }
}

fn scenarios(dir: &Path) -> Vec<(String, Value)> {
    let mut out = Vec::new();
    for name in ["retry", "money", "forward_compat"] {
        for s in suite(dir, name)["scenarios"].as_array().unwrap() {
            out.push((format!("{name}/{}", s["name"].as_str().unwrap()), s.clone()));
        }
    }
    assert!(!out.is_empty());
    out
}

#[tokio::test(start_paused = true)]
async fn call_scenarios_on_the_async_client() {
    let Some(dir) = conformance_dir() else { return };
    let idem = idempotency_header(&dir);
    for (name, scenario) in scenarios(&dir) {
        let script = Script::new(&scenario["responses"]);
        let client = builder(&script).build().unwrap();
        let op = scenario["call"]["operation"].as_str().unwrap();
        let outcome = call_async(&client, op, scenario["call"]["args"].clone()).await;
        check(&idem, &name, &scenario, &script, outcome, true);
    }
}

#[test]
fn call_scenarios_on_the_blocking_client() {
    let Some(dir) = conformance_dir() else { return };
    let idem = idempotency_header(&dir);
    for (name, scenario) in scenarios(&dir) {
        let script = Script::new(&scenario["responses"]);
        let client = builder(&script).build_blocking().unwrap();
        let op = scenario["call"]["operation"].as_str().unwrap();
        let outcome = call_blocking(&client, op, scenario["call"]["args"].clone());
        check(&idem, &name, &scenario, &script, outcome, false);
    }
}
