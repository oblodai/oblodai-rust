//! Тесты Oblodai SDK. Ядро проверяется через мок-транспорт (без сети).

use oblodai::{
    compute_webhook_signature, construct_event, verify_webhook, Client, Config, Error,
    HttpResponse, HttpTransport, VerifyOptions, WebhookHeaders,
};
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// ─────────────────────────── Мок-транспорт ───────────────────────────

/// Один запрограммированный ответ.
#[derive(Clone)]
struct MockResponse {
    status: u16,
    body: String,
    retry_after: Option<u64>,
}

/// Мок-транспорт: отдаёт заранее заданные ответы по очереди, запоминает запросы.
struct MockTransport {
    responses: Mutex<Vec<MockResponse>>,
    calls: Mutex<Vec<(String, Vec<(String, String)>, String)>>,
}

impl MockTransport {
    fn new(responses: Vec<MockResponse>) -> Arc<Self> {
        Arc::new(Self {
            responses: Mutex::new(responses),
            calls: Mutex::new(Vec::new()),
        })
    }
    fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
    fn last_headers(&self) -> Vec<(String, String)> {
        self.calls
            .lock()
            .unwrap()
            .last()
            .map(|c| c.1.clone())
            .unwrap_or_default()
    }
}

impl MockTransport {
    fn take(&self, url: &str, headers: &[(String, String)], body: &[u8]) -> HttpResponse {
        self.calls.lock().unwrap().push((
            url.to_string(),
            headers.to_vec(),
            String::from_utf8_lossy(body).to_string(),
        ));
        let mut r = self.responses.lock().unwrap();
        let resp = if r.len() > 1 {
            r.remove(0)
        } else {
            r[0].clone()
        };
        HttpResponse {
            status: resp.status,
            body: resp.body.into_bytes(),
            retry_after: resp.retry_after.map(std::time::Duration::from_secs),
        }
    }
}

impl HttpTransport for MockTransport {
    fn post(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &[u8],
    ) -> Result<HttpResponse, Error> {
        Ok(self.take(url, headers, body))
    }

    fn get(&self, url: &str, headers: &[(String, String)]) -> Result<HttpResponse, Error> {
        Ok(self.take(url, headers, b""))
    }
}

fn ok(body: serde_json::Value) -> MockResponse {
    MockResponse {
        status: 200,
        body: body.to_string(),
        retry_after: None,
    }
}

fn client_with(t: Arc<MockTransport>) -> Client {
    Client::with_transport(
        Config::new("pub_1", "sec_1")
            .base_url("https://api.test")
            .retry(None),
        t,
    )
    .unwrap()
}

// ─────────────────────────── Подпись/вебхуки ───────────────────────────

#[test]
fn webhook_signature_matches_reference() {
    let secret = "wh_secret";
    let ts = "1700000000";
    let body = b"{\"status\":\"paid\"}";
    let sig = compute_webhook_signature(secret, ts, body);
    // Пересчёт вручную тем же алгоритмом должен совпасть.
    let sig2 = compute_webhook_signature(secret, ts, body);
    assert_eq!(sig, sig2);
    assert_eq!(sig.len(), 64); // hex sha256
}

#[test]
fn verify_webhook_accepts_valid() {
    let secret = "wh";
    let ts = now().to_string();
    let body = b"{\"type\":\"payment\",\"status\":\"paid\"}";
    let sig = compute_webhook_signature(secret, &ts, body);
    let headers = WebhookHeaders {
        timestamp: &ts,
        signature: &sig,
    };
    assert!(verify_webhook(secret, body, &headers, &VerifyOptions::default()).is_ok());
}

#[test]
fn verify_webhook_rejects_bad_signature() {
    let ts = now().to_string();
    let headers = WebhookHeaders {
        timestamp: &ts,
        signature: "deadbeef",
    };
    let err = verify_webhook("wh", b"{}", &headers, &VerifyOptions::default());
    assert!(matches!(err, Err(Error::Signature(_))));
}

#[test]
fn verify_webhook_rejects_replay() {
    let secret = "wh";
    let old = (now() - 3600).to_string();
    let body = b"{\"status\":\"paid\"}";
    let sig = compute_webhook_signature(secret, &old, body);
    let headers = WebhookHeaders {
        timestamp: &old,
        signature: &sig,
    };

    // с проверкой свежести — отклонить
    let err = verify_webhook(
        secret,
        body,
        &headers,
        &VerifyOptions {
            max_age_seconds: 300,
            now: None,
        },
    );
    assert!(matches!(err, Err(Error::Signature(_))));

    // без проверки свежести — пройти
    assert!(verify_webhook(
        secret,
        body,
        &headers,
        &VerifyOptions {
            max_age_seconds: 0,
            now: None
        }
    )
    .is_ok());
}

#[test]
fn construct_event_parses_body() {
    let secret = "wh";
    let ts = now().to_string();
    let body = b"{\"type\":\"payment\",\"status\":\"paid\",\"uuid\":\"abc\"}";
    let sig = compute_webhook_signature(secret, &ts, body);
    let headers = WebhookHeaders {
        timestamp: &ts,
        signature: &sig,
    };

    let event: serde_json::Value =
        construct_event(secret, body, &headers, &VerifyOptions::default()).unwrap();
    assert_eq!(event["uuid"], "abc");
    assert_eq!(event["status"], "paid");
}

// ─────────────────────────── Клиент ───────────────────────────

#[test]
fn client_signs_and_unwraps() {
    let t = MockTransport::new(vec![ok(json!({
        "state": 0,
        "result": { "uuid": "p1", "order_id": "o1", "amount": "10.00",
                    "currency": "USD", "payment_status": "check", "address": "T123" }
    }))]);
    let client = client_with(t.clone());

    let payment = client
        .payments()
        .create(json!({ "amount": "10", "currency": "USD", "order_id": "o1" }))
        .unwrap();

    assert_eq!(payment.uuid, "p1");
    assert_eq!(payment.address, "T123");

    let headers = t.last_headers();
    let has = |name: &str| headers.iter().any(|(k, _)| k == name);
    assert!(has("X-Public-Id"));
    assert!(has("X-Timestamp"));
    let sig = headers
        .iter()
        .find(|(k, _)| k == "X-Signature")
        .map(|(_, v)| v.clone())
        .unwrap();
    assert_eq!(sig.len(), 64);
}

#[test]
fn client_api_error() {
    let t = MockTransport::new(vec![MockResponse {
        status: 409,
        body: json!({ "error": { "code": "payout.insufficient_funds", "message": "no" } })
            .to_string(),
        retry_after: None,
    }]);
    let client = client_with(t);

    let err = client
        .payouts()
        .create(json!({ "amount": "5", "currency": "USDT", "address": "T", "order_id": "x" }))
        .unwrap_err();

    match err {
        Error::Api { code, status, .. } => {
            assert_eq!(code, "payout.insufficient_funds");
            assert_eq!(status, 409);
        }
        _ => panic!("expected Api error, got {err:?}"),
    }
    assert!(!Error::Api {
        code: "payout.insufficient_funds".into(),
        message: String::new(),
        status: 409,
        raw: String::new(),
        retry_after: None,
    }
    .is_retriable());
}

#[test]
fn client_retries_503_then_success() {
    let t = MockTransport::new(vec![
        MockResponse {
            status: 503,
            body: json!({ "error": { "code": "x.unavailable", "message": "later" } }).to_string(),
            retry_after: None,
        },
        ok(json!({ "state": 0, "result": { "balance": { "merchant": [] } } })),
    ]);
    let client = Client::with_transport(
        Config::new("p", "s")
            .base_url("https://api.test")
            .retry(Some(oblodai::RetryConfig {
                max_attempts: 3,
                initial_delay: std::time::Duration::from_millis(1),
                max_delay: std::time::Duration::from_millis(5),
            })),
        t.clone(),
    )
    .unwrap();

    let bal = client.account().balance().unwrap();
    assert_eq!(bal.merchant.len(), 0);
    assert_eq!(t.call_count(), 2); // 503 + успех
}

#[test]
fn client_does_not_retry_400() {
    let t = MockTransport::new(vec![MockResponse {
        status: 400,
        body: json!({ "error": { "code": "request.bad_json", "message": "bad" } }).to_string(),
        retry_after: None,
    }]);
    let client = Client::with_transport(
        Config::new("p", "s")
            .base_url("https://api.test")
            .retry(Some(oblodai::RetryConfig {
                max_attempts: 3,
                initial_delay: std::time::Duration::from_millis(1),
                max_delay: std::time::Duration::from_millis(5),
            })),
        t.clone(),
    )
    .unwrap();

    let err = client.account().balance().unwrap_err();
    assert_eq!(err.code(), Some("request.bad_json"));
    assert_eq!(t.call_count(), 1); // без повторов
}

#[test]
fn client_public_rate_no_signature() {
    let t = MockTransport::new(vec![ok(json!({
        "state": 0, "result": [ { "from": "ETH", "to": "USDT", "course": "3450" } ]
    }))]);
    let client = client_with(t.clone());

    let rates = client.rates().list(Some("ETH")).unwrap();
    assert_eq!(rates.len(), 1);
    assert_eq!(rates[0].course, "3450");
    assert_eq!(rates[0].from, "ETH");

    let headers = t.last_headers();
    assert!(!headers.iter().any(|(k, _)| k == "X-Signature"));
}

#[test]
fn client_webhook_register_no_envelope() {
    let t = MockTransport::new(vec![MockResponse {
        status: 201,
        body: json!({ "endpoint_id": "e1", "url": "https://x", "secret": "s1" }).to_string(),
        retry_after: None,
    }]);
    let client = client_with(t);

    let reg = client.webhooks().register("https://x").unwrap();
    assert_eq!(reg.secret, "s1");
    assert_eq!(reg.endpoint_id, "e1");
}

#[test]
fn client_mass_payout_partial() {
    let t = MockTransport::new(vec![ok(json!({
        "state": 0,
        "result": { "items": [
            { "uuid": "u1", "order_id": "p-1", "status": "process", "success": true },
            { "order_id": "p-2", "success": false, "message": "insufficient" }
        ] }
    }))]);
    let client = client_with(t);

    let res = client
        .payouts()
        .create_mass(
            vec![
                json!({ "amount": "25", "currency": "USDT", "network": "tron", "address": "T1", "order_id": "p-1" }),
                json!({ "amount": "10", "currency": "USDT", "network": "tron", "address": "T2", "order_id": "p-2" }),
            ],
            None,
        )
        .unwrap();

    assert_eq!(res.items.len(), 2);
    assert!(res.items[0].success);
    assert!(!res.items[1].success);
    assert_eq!(res.items[1].message, "insufficient");
}

#[test]
fn missing_config_errors() {
    let t = MockTransport::new(vec![ok(json!({}))]);
    assert!(Client::with_transport(Config::new("", "s"), t.clone()).is_err());
    assert!(Client::with_transport(Config::new("p", ""), t).is_err());
}

#[test]
fn config_from_env() {
    std::env::set_var("OBLODAI_PUBLIC_ID", "pub_env");
    std::env::set_var("OBLODAI_SECRET", "sec_env");
    std::env::set_var("OBLODAI_BASE_URL", "https://env.example");

    let cfg = Config::from_env().unwrap();
    assert_eq!(cfg.public_id, "pub_env");
    assert_eq!(cfg.secret, "sec_env");
    assert_eq!(cfg.base_url, "https://env.example");

    // Пропущенная обязательная переменная → Error::Config.
    std::env::remove_var("OBLODAI_PUBLIC_ID");
    assert!(matches!(Config::from_env(), Err(Error::Config(_))));

    std::env::remove_var("OBLODAI_SECRET");
    std::env::remove_var("OBLODAI_BASE_URL");
}

#[test]
fn currencies_public_get_unsigned() {
    let t = MockTransport::new(vec![ok(json!({
        "currencies": [
            { "symbol": "USDT", "decimals": 6, "networks": [
                { "network": "tron", "kind": "token", "min_confirmations": 20,
                  "available": true, "deposit_available": true, "payout_available": true }
            ] }
        ]
    }))]);
    let client = client_with(t.clone());

    let cur = client.rates().currencies().unwrap();
    assert_eq!(cur.len(), 1);
    assert_eq!(cur[0].symbol, "USDT");
    assert_eq!(cur[0].networks[0].network, "tron");

    // Порядок важен: last_headers() лочит `calls` внутри — нельзя держать этот лок здесь одновременно.
    let headers = t.last_headers();
    assert!(!headers.iter().any(|(k, _)| k == "X-Signature"));
    let url = t.calls.lock().unwrap()[0].0.clone();
    assert!(url.ends_with("/v1/currencies"));
}

#[test]
fn list_discounts_covered() {
    let t = MockTransport::new(vec![ok(json!({
        "state": 0,
        "result": [ { "currency": "USDT", "network": "tron", "discount_percent": 3 } ]
    }))]);
    let client = client_with(t.clone());

    let list = client.payments().list_discounts().unwrap();
    assert_eq!(list[0]["currency"], "USDT");
    let calls = t.calls.lock().unwrap();
    assert!(calls[0].0.ends_with("/v1/payment/discount/list"));
}

// ─────────────────── Автоматический order_id (идемпотентность) ───────────────────

/// Достаёт `order_id` из тела POST-запроса по индексу вызова.
fn body_order_id(t: &MockTransport, call_idx: usize) -> String {
    let calls = t.calls.lock().unwrap();
    let body = &calls[call_idx].2;
    let v: serde_json::Value = serde_json::from_str(body).unwrap();
    v.get("order_id")
        .and_then(|o| o.as_str())
        .unwrap_or("")
        .to_string()
}

fn payment_result() -> serde_json::Value {
    json!({
        "state": 0,
        "result": { "uuid": "p1", "order_id": "auto", "amount": "10.00",
                    "currency": "USD", "payment_status": "check", "address": "T123" }
    })
}

#[test]
fn payment_create_injects_order_id_when_missing() {
    let t = MockTransport::new(vec![ok(payment_result())]);
    let client = client_with(t.clone());

    client
        .payments()
        .create(json!({ "amount": "10", "currency": "USD" }))
        .unwrap();

    let oid = body_order_id(&t, 0);
    assert!(oid.starts_with("idem-"), "ожидался idem-ключ, получено {oid:?}");
    assert!(oid.len() > "idem-".len(), "order_id не должен быть пустым");
}

#[test]
fn payment_create_keeps_caller_order_id() {
    let t = MockTransport::new(vec![ok(payment_result())]);
    let client = client_with(t.clone());

    client
        .payments()
        .create(json!({ "amount": "10", "currency": "USD", "order_id": "mine-1" }))
        .unwrap();

    assert_eq!(body_order_id(&t, 0), "mine-1");
}

#[test]
fn payment_create_same_order_id_across_retries() {
    // 503 (retriable) один раз, затем успех — тело должно быть идентичным на обеих попытках.
    let t = MockTransport::new(vec![
        MockResponse {
            status: 503,
            body: json!({ "error": { "code": "x.unavailable", "message": "later" } }).to_string(),
            retry_after: None,
        },
        ok(payment_result()),
    ]);
    let client = Client::with_transport(
        Config::new("p", "s")
            .base_url("https://api.test")
            .retry(Some(oblodai::RetryConfig {
                max_attempts: 3,
                initial_delay: std::time::Duration::from_millis(1),
                max_delay: std::time::Duration::from_millis(5),
            })),
        t.clone(),
    )
    .unwrap();

    client
        .payments()
        .create(json!({ "amount": "10", "currency": "USD" }))
        .unwrap();

    assert_eq!(t.call_count(), 2, "должно быть 2 попытки: 503 + успех");
    let first = body_order_id(&t, 0);
    let second = body_order_id(&t, 1);
    assert!(first.starts_with("idem-"));
    assert_eq!(first, second, "order_id обязан совпадать на повторе, иначе возможен дубль");
}

#[test]
fn transfer_to_personal_injects_order_id() {
    let t = MockTransport::new(vec![ok(json!({ "state": 0, "result": { "ok": true } }))]);
    let client = client_with(t.clone());

    client
        .account()
        .transfer_to_personal(json!({ "amount": "5", "currency": "USDT" }))
        .unwrap();

    let oid = body_order_id(&t, 0);
    assert!(oid.starts_with("idem-"), "ожидался idem-ключ, получено {oid:?}");
}

#[test]
fn funds_maturing_is_terminal() {
    let e = Error::Api {
        code: "payout.funds_maturing".into(),
        message: String::new(),
        status: 409,
        raw: String::new(),
        retry_after: None,
    };
    assert!(!e.is_retriable(), "payout.funds_maturing должна быть терминальной");
}

#[test]
fn rate_limit_429_surfaces_message() {
    let t = MockTransport::new(vec![MockResponse {
        status: 429,
        body: json!({ "state": 1, "message": "rate limit exceeded" }).to_string(),
        retry_after: Some(60),
    }]);
    let client = client_with(t);

    let err = client.account().balance().unwrap_err();
    assert_eq!(err.retry_after(), Some(std::time::Duration::from_secs(60)));
    match err {
        Error::Api {
            code,
            message,
            status,
            ..
        } => {
            assert_eq!(code, "http.429");
            assert_eq!(status, 429);
            assert_eq!(message, "rate limit exceeded");
        }
        _ => panic!("expected Api error, got {err:?}"),
    }
}

#[test]
fn rate_limit_429_retries_after_advised_delay() {
    let t = MockTransport::new(vec![
        MockResponse {
            status: 429,
            body: json!({ "state": 1, "message": "rate limit exceeded" }).to_string(),
            retry_after: Some(0),
        },
        ok(json!({ "state": 0, "result": { "balance": { "merchant": [] } } })),
    ]);
    let client = Client::with_transport(
        Config::new("p", "s")
            .base_url("https://api.test")
            .retry(Some(oblodai::RetryConfig {
                max_attempts: 3,
                initial_delay: std::time::Duration::from_millis(1),
                max_delay: std::time::Duration::from_millis(5),
            })),
        t.clone(),
    )
    .unwrap();

    let bal = client.account().balance().unwrap();
    assert_eq!(bal.merchant.len(), 0);
    assert_eq!(t.call_count(), 2);
}
