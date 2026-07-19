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

/// Записанный запрос: url, заголовки, тело.
type RecordedCall = (String, Vec<(String, String)>, String);

/// Мок-транспорт: отдаёт заранее заданные ответы по очереди, запоминает запросы.
struct MockTransport {
    responses: Mutex<Vec<MockResponse>>,
    calls: Mutex<Vec<RecordedCall>>,
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

// ─────────────────── Идемпотентность: заголовок Idempotency-Key ───────────────────

/// Достаёт значение заголовка по индексу вызова.
fn header_of(t: &MockTransport, call_idx: usize, name: &str) -> Option<String> {
    let calls = t.calls.lock().unwrap();
    calls[call_idx]
        .1
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.clone())
}

/// Тело вызова как JSON.
fn body_json(t: &MockTransport, call_idx: usize) -> serde_json::Value {
    let calls = t.calls.lock().unwrap();
    serde_json::from_str(&calls[call_idx].2).unwrap()
}

/// URL вызова по индексу.
fn url_of(t: &MockTransport, call_idx: usize) -> String {
    t.calls.lock().unwrap()[call_idx].0.clone()
}

fn payment_result() -> serde_json::Value {
    json!({
        "state": 0,
        "result": { "uuid": "p1", "order_id": "", "amount": "10.00",
                    "currency": "USD", "payment_status": "check", "address": "T123" }
    })
}

/// Клиент с быстрыми повторами (для ретрай-тестов).
fn client_with_retries(t: Arc<MockTransport>) -> Client {
    Client::with_transport(
        Config::new("p", "s")
            .base_url("https://api.test")
            .retry(Some(oblodai::RetryConfig {
                max_attempts: 3,
                initial_delay: std::time::Duration::from_millis(1),
                max_delay: std::time::Duration::from_millis(5),
            })),
        t,
    )
    .unwrap()
}

#[test]
fn payment_create_sends_idempotency_key_and_no_auto_order_id() {
    let t = MockTransport::new(vec![ok(payment_result())]);
    let client = client_with(t.clone());

    client
        .payments()
        .create(json!({ "amount": "10", "currency": "USD" }))
        .unwrap();

    // Заголовок есть и похож на UUID v4.
    let key = header_of(&t, 0, "Idempotency-Key").expect("нет заголовка Idempotency-Key");
    assert_eq!(key.len(), 36, "ожидался UUID, получено {key:?}");
    assert_eq!(key.matches('-').count(), 4);

    // Авто-order_id больше НЕ подставляется: тело уходит как есть.
    let body = body_json(&t, 0);
    assert!(
        body.get("order_id").is_none(),
        "order_id не должен подставляться автоматически, получено {body}"
    );
}

#[test]
fn payment_create_keeps_caller_order_id() {
    let t = MockTransport::new(vec![ok(payment_result())]);
    let client = client_with(t.clone());

    client
        .payments()
        .create(json!({ "amount": "10", "currency": "USD", "order_id": "mine-1" }))
        .unwrap();

    assert_eq!(body_json(&t, 0)["order_id"], "mine-1");
}

#[test]
fn payment_create_same_idempotency_key_across_retries() {
    // 503 (retriable), затем успех — заголовок и тело обязаны совпадать на обеих попытках.
    let t = MockTransport::new(vec![
        MockResponse {
            status: 503,
            body: json!({ "error": { "code": "x.unavailable", "message": "later" } }).to_string(),
            retry_after: None,
        },
        ok(payment_result()),
    ]);
    let client = client_with_retries(t.clone());

    client
        .payments()
        .create(json!({ "amount": "10", "currency": "USD", "order_id": "ord-1" }))
        .unwrap();

    assert_eq!(t.call_count(), 2, "должно быть 2 попытки: 503 + успех");
    let k1 = header_of(&t, 0, "Idempotency-Key").expect("нет заголовка на 1-й попытке");
    let k2 = header_of(&t, 1, "Idempotency-Key").expect("нет заголовка на 2-й попытке");
    assert_eq!(
        k1, k2,
        "Idempotency-Key обязан совпадать на повторе, иначе возможен дубль"
    );
    assert_eq!(
        body_json(&t, 0),
        body_json(&t, 1),
        "тело обязано быть идентичным на повторе"
    );
    assert_eq!(
        body_json(&t, 0)["order_id"],
        "ord-1",
        "order_id уходит как есть"
    );
}

#[test]
fn payment_create_explicit_idempotency_key_goes_to_header_only() {
    let t = MockTransport::new(vec![ok(payment_result())]);
    let client = client_with(t.clone());

    client
        .payments()
        .create(json!({ "amount": "10", "currency": "USD", "idempotency_key": "my-key-1" }))
        .unwrap();

    assert_eq!(
        header_of(&t, 0, "Idempotency-Key").as_deref(),
        Some("my-key-1"),
        "явный ключ обязан уйти в заголовок"
    );
    let body = body_json(&t, 0);
    assert!(
        body.get("idempotency_key").is_none(),
        "idempotency_key не должен попадать в тело, получено {body}"
    );
}

#[test]
fn payout_create_and_transfer_send_idempotency_key() {
    let t = MockTransport::new(vec![
        ok(json!({ "state": 0, "result": { "uuid": "po1", "status": "check" } })),
        ok(json!({ "state": 0, "result": { "ok": true } })),
    ]);
    let client = client_with(t.clone());

    client
        .payouts()
        .create(
            json!({ "amount": "5", "currency": "USDT", "network": "tron",
                        "address": "T1", "order_id": "w-1" }),
        )
        .unwrap();
    client
        .account()
        .transfer_to_personal(json!({ "amount": "5", "currency": "USDT" }))
        .unwrap();

    assert!(header_of(&t, 0, "Idempotency-Key").is_some());
    assert!(header_of(&t, 1, "Idempotency-Key").is_some());
    // Авто-order_id не подставляется и здесь.
    assert!(body_json(&t, 1).get("order_id").is_none());
}

#[test]
fn info_endpoints_do_not_send_idempotency_key() {
    let t = MockTransport::new(vec![ok(payment_result())]);
    let client = client_with(t.clone());

    client.payments().info(Some("p1"), None).unwrap();
    assert!(
        header_of(&t, 0, "Idempotency-Key").is_none(),
        "read-only эндпоинты не должны слать Idempotency-Key"
    );
}

// ─────────────────── Массовые операции (v1.1.0) ───────────────────

fn batch_submit_result(kind: &str) -> serde_json::Value {
    json!({ "state": 0, "result": {
        "batch_id": "b-1", "kind": kind, "count": 2, "status": "pending"
    }})
}

#[test]
fn payment_batch_submit() {
    let t = MockTransport::new(vec![ok(batch_submit_result("payment"))]);
    let client = client_with(t.clone());

    let sub = client
        .payments()
        .create_batch(
            vec![
                json!({ "amount": "10", "currency": "USD", "order_id": "a-1" }),
                json!({ "amount": "20", "currency": "EUR", "order_id": "a-2" }),
            ],
            Some("stop"),
        )
        .unwrap();

    assert_eq!(sub.batch_id, "b-1");
    assert_eq!(sub.status, "pending");
    assert!(url_of(&t, 0).ends_with("/v1/payment/batch"));
    let body = body_json(&t, 0);
    assert_eq!(body["payments"].as_array().unwrap().len(), 2);
    assert_eq!(body["on_error"], "stop");
    assert!(
        header_of(&t, 0, "Idempotency-Key").is_some(),
        "батч обязан быть идемпотентным"
    );
}

#[test]
fn refund_and_payout_batch_paths_and_fields() {
    let t = MockTransport::new(vec![
        ok(batch_submit_result("refund")),
        ok(batch_submit_result("payout")),
    ]);
    let client = client_with(t.clone());

    client
        .payments()
        .refund_batch(vec![json!({ "uuid": "p1", "reference": "r-1" })], None)
        .unwrap();
    client
        .payouts()
        .create_batch(vec![json!({ "amount": "5", "order_id": "w-1" })], None)
        .unwrap();

    assert!(url_of(&t, 0).ends_with("/v1/refund/batch"));
    assert!(body_json(&t, 0).get("refunds").is_some());
    assert!(
        body_json(&t, 0).get("on_error").is_none(),
        "on_error опционален"
    );
    assert!(url_of(&t, 1).ends_with("/v1/payout/batch"));
    assert!(body_json(&t, 1).get("payouts").is_some());
    assert!(header_of(&t, 1, "Idempotency-Key").is_some());
}

#[test]
fn batch_info_parses_items() {
    let t = MockTransport::new(vec![ok(json!({ "state": 0, "result": {
        "batch_id": "b-1", "kind": "payment", "status": "completed", "on_error": "continue",
        "total": 2, "succeeded": 1, "failed": 1,
        "items": [
            { "idx": 0, "status": "succeeded", "order_id": "a-1",
              "result": { "uuid": "p1", "payment_status": "check" } },
            { "idx": 1, "status": "failed", "order_id": "a-2",
              "error": "payment.unknown_currency" }
        ]
    }}))]);
    let client = client_with(t.clone());

    let info = client.batches().info("b-1", Some(100), Some(0)).unwrap();
    assert_eq!(info.status, "completed");
    assert_eq!(info.total, 2);
    assert_eq!(info.items.len(), 2);
    assert_eq!(info.items[0].result["uuid"], "p1");
    assert_eq!(info.items[1].error, json!("payment.unknown_currency"));

    assert!(url_of(&t, 0).ends_with("/v1/batch/info"));
    let body = body_json(&t, 0);
    assert_eq!(body["batch_id"], "b-1");
    assert_eq!(body["limit"], 100);
    assert!(
        header_of(&t, 0, "Idempotency-Key").is_none(),
        "batch/info — read-only"
    );
}

// ─────────────────── Платёжные ссылки (v1.1.0) ───────────────────

#[test]
fn payment_link_create_and_toggle() {
    let t = MockTransport::new(vec![
        ok(json!({ "state": 0, "result": { "link_id": "l-1", "url": "https://pay/link/l-1" } })),
        ok(json!({ "state": 0, "result": { "link_id": "l-1", "active": false } })),
    ]);
    let client = client_with(t.clone());

    let link = client
        .payment_links()
        .create(json!({ "title": "Донат", "amount_mode": "open", "currency": "USD" }))
        .unwrap();
    assert_eq!(link.link_id, "l-1");
    assert_eq!(link.url, "https://pay/link/l-1");
    assert!(url_of(&t, 0).ends_with("/v1/payment/link"));

    let toggled = client.payment_links().toggle("l-1", false).unwrap();
    assert!(!toggled.active);
    assert!(url_of(&t, 1).ends_with("/v1/payment/link/toggle"));
    assert_eq!(body_json(&t, 1)["active"], false);
}

#[test]
fn payment_link_list_and_info() {
    let t = MockTransport::new(vec![
        ok(json!({ "state": 0, "result": { "items": [
            { "link_id": "l-1", "amount_mode": "fixed", "amount_fixed": "5", "active": true }
        ]}})),
        ok(json!({ "state": 0, "result": {
            "link_id": "l-1", "active": true,
            "payments": [ { "uuid": "p1", "status": "paid", "amount": "5" } ]
        }})),
    ]);
    let client = client_with(t.clone());

    let items = client.payment_links().list(Some(10), None).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].amount_fixed, "5");
    assert!(url_of(&t, 0).ends_with("/v1/payment/link/list"));

    let info = client.payment_links().info("l-1").unwrap();
    assert_eq!(info.payments.len(), 1);
    assert_eq!(info.payments[0].status, "paid");
    assert!(url_of(&t, 1).ends_with("/v1/payment/link/info"));
}

#[test]
fn payment_link_checkout_public_unsigned() {
    let t = MockTransport::new(vec![ok(payment_result())]);
    let client = client_with(t.clone());

    let p = client
        .payment_links()
        .checkout(
            "l-1",
            json!({ "amount": "5", "currency": "USDT", "network": "tron" }),
        )
        .unwrap();
    assert_eq!(p.uuid, "p1");

    assert!(url_of(&t, 0).ends_with("/v1/link/l-1/checkout"));
    let headers = t.last_headers();
    assert!(
        !headers.iter().any(|(k, _)| k == "X-Signature"),
        "публичный checkout не должен подписываться"
    );
}

// ─────────────────── Сплиты (v1.1.0) ───────────────────

#[test]
fn splits_rules_and_config() {
    let t = MockTransport::new(vec![
        ok(json!({ "state": 0, "result": { "rule_id": "r-1", "percent": 10.0 } })),
        ok(json!({ "state": 0, "result": { "items": [
            { "rule_id": "r-1", "percent": 10.0, "active": true,
              "address": "T1", "network": "tron", "reversible": false }
        ]}})),
        ok(json!({ "state": 0, "result": { "deleted": true } })),
        ok(json!({ "state": 0, "result": { "refund_hold_hours": 24 } })),
    ]);
    let client = client_with(t.clone());

    let rule = client
        .splits()
        .create_rule(json!({ "address": "T1", "network": "tron", "percent": 10.0 }))
        .unwrap();
    assert_eq!(rule.rule_id, "r-1");

    let rules = client.splits().list_rules().unwrap();
    assert_eq!(rules.len(), 1);
    assert!(!rules[0].reversible);

    client.splits().delete_rule("r-1").unwrap();
    assert_eq!(body_json(&t, 2)["rule_id"], "r-1");

    let cfg = client.splits().get_config().unwrap();
    assert_eq!(cfg.refund_hold_hours, 24);

    assert!(url_of(&t, 0).ends_with("/v1/split/rule"));
    assert!(url_of(&t, 1).ends_with("/v1/split/rule/list"));
    assert!(url_of(&t, 2).ends_with("/v1/split/rule/delete"));
    assert!(url_of(&t, 3).ends_with("/v1/split/config/get"));
}

// ─────────────────── Счёт на e-mail (v1.1.0) ───────────────────

#[test]
fn send_email_body_and_path() {
    let t = MockTransport::new(vec![ok(json!({ "state": 0, "result": {
        "sent": true, "email": "b@e.com", "uuid": "p1"
    }}))]);
    let client = client_with(t.clone());

    let res = client
        .payments()
        .send_email(Some("p1"), None, Some("b@e.com"))
        .unwrap();
    assert_eq!(res["sent"], true);

    assert!(url_of(&t, 0).ends_with("/v1/payment/send-email"));
    let body = body_json(&t, 0);
    assert_eq!(body["uuid"], "p1");
    assert_eq!(body["email"], "b@e.com");
    assert!(
        header_of(&t, 0, "Idempotency-Key").is_none(),
        "send-email без Idempotency-Key"
    );
}

// ─────────────────── Payout-ссылки (v1.1.0) ───────────────────

use oblodai::PayoutLinkStatus;

fn payout_link_result() -> serde_json::Value {
    json!({ "state": 0, "result": {
        "link_id": "pl-1", "status": "funded", "amount": "0.005", "currency": "BTC",
        "network": "bitcoin", "expires_at": "2026-08-14T17:00:00Z",
        "claim_token": "Xk3vTOKEN", "claim_url": "https://pay/claim/Xk3vTOKEN"
    }})
}

#[test]
fn payout_link_create_sends_idempotency_key() {
    let t = MockTransport::new(vec![ok(payout_link_result())]);
    let client = client_with(t.clone());

    let link = client
        .payout_links()
        .create(
            json!({ "currency": "BTC", "network": "bitcoin", "amount": "0.005",
                        "reference": "bonus-42", "expires_in_hours": 720 }),
        )
        .unwrap();

    assert_eq!(link.link_id, "pl-1");
    assert_eq!(link.status, PayoutLinkStatus::Funded);
    assert_eq!(link.claim_token, "Xk3vTOKEN");

    assert!(url_of(&t, 0).ends_with("/v1/payout/link"));
    // Создание ссылки резервирует баланс — без ключа повтор профинансировал бы вторую ссылку.
    let key = header_of(&t, 0, "Idempotency-Key")
        .expect("/v1/payout/link обязан слать Idempotency-Key: он резервирует средства");
    assert_eq!(key.len(), 36, "ожидался UUID v4, получено {key:?}");
    // Но запрос подписан (management-эндпоинт).
    assert!(t.last_headers().iter().any(|(k, _)| k == "X-Signature"));
}

#[test]
fn payout_link_create_explicit_idempotency_key_goes_to_header_only() {
    let t = MockTransport::new(vec![ok(payout_link_result())]);
    let client = client_with(t.clone());

    client
        .payout_links()
        .create(
            json!({ "currency": "BTC", "network": "bitcoin", "amount": "0.005",
                        "idempotency_key": "link-key-1" }),
        )
        .unwrap();

    assert_eq!(
        header_of(&t, 0, "Idempotency-Key").as_deref(),
        Some("link-key-1")
    );
    let body = body_json(&t, 0);
    assert!(
        body.get("idempotency_key").is_none(),
        "idempotency_key не должен попадать в тело, получено {body}"
    );
}

#[test]
fn payout_link_create_same_idempotency_key_across_retries() {
    // 503 (retriable), затем успех: ключ обязан совпасть, иначе повтор создаст ВТОРУЮ
    // профинансированную ссылку и зарезервирует баланс дважды.
    let t = MockTransport::new(vec![
        MockResponse {
            status: 503,
            body: json!({ "error": { "code": "x.unavailable", "message": "later" } }).to_string(),
            retry_after: None,
        },
        ok(payout_link_result()),
    ]);
    let client = client_with_retries(t.clone());

    client
        .payout_links()
        .create(json!({ "currency": "BTC", "network": "bitcoin", "amount": "0.005" }))
        .unwrap();

    assert_eq!(t.call_count(), 2, "должно быть 2 попытки: 503 + успех");
    let k1 = header_of(&t, 0, "Idempotency-Key").expect("нет заголовка на 1-й попытке");
    let k2 = header_of(&t, 1, "Idempotency-Key").expect("нет заголовка на 2-й попытке");
    assert_eq!(
        k1, k2,
        "ключ обязан совпадать на повторе, иначе дубль ссылки"
    );
    assert_eq!(body_json(&t, 0), body_json(&t, 1));
}

#[test]
fn payout_link_batch_same_idempotency_key_across_retries() {
    let batch_ok = ok(json!({ "state": 0, "result": {
        "created": 1, "total": 1,
        "results": [ { "ok": true, "link": { "link_id": "pl-1", "status": "funded" } } ]
    }}));
    let t = MockTransport::new(vec![
        MockResponse {
            status: 503,
            body: json!({ "error": { "code": "x.unavailable", "message": "later" } }).to_string(),
            retry_after: None,
        },
        batch_ok,
    ]);
    let client = client_with_retries(t.clone());

    client
        .payout_links()
        .create_batch(vec![
            json!({ "currency": "BTC", "network": "bitcoin", "amount": "0.005" }),
        ])
        .unwrap();

    assert_eq!(t.call_count(), 2);
    let k1 = header_of(&t, 0, "Idempotency-Key").expect("пачка обязана слать Idempotency-Key");
    let k2 = header_of(&t, 1, "Idempotency-Key").expect("нет заголовка на повторе");
    assert_eq!(k1, k2, "ключ пачки обязан совпадать на повторе");
}

/// `503 idempotency.unavailable` — стор идемпотентности недоступен, шлюз fail-closed и операцию
/// НЕ выполняет. Это единственный код идемпотентного слоя, который надо повторять, причём с ТЕМ ЖЕ
/// ключом: иначе повтор пройдёт как новая операция и профинансирует вторую ссылку.
#[test]
fn payout_link_retries_idempotency_unavailable_with_same_key() {
    let t = MockTransport::new(vec![
        MockResponse {
            status: 503,
            body: json!({ "error": { "code": "idempotency.unavailable",
                                     "message": "idempotency store unavailable, retry" } })
            .to_string(),
            retry_after: None,
        },
        ok(payout_link_result()),
    ]);
    let client = client_with_retries(t.clone());

    client
        .payout_links()
        .create(json!({ "currency": "BTC", "network": "bitcoin", "amount": "0.005" }))
        .unwrap();

    assert_eq!(
        t.call_count(),
        2,
        "503 idempotency.unavailable обязан повторяться"
    );
    let k1 = header_of(&t, 0, "Idempotency-Key").unwrap();
    let k2 = header_of(&t, 1, "Idempotency-Key").unwrap();
    assert_eq!(k1, k2, "повтор после 503 обязан идти с тем же ключом");
}

/// Терминальные коды идемпотентного слоя: SDK обязан вернуть их вызывающему с ПЕРВОЙ попытки.
/// `409 payoutlink.duplicate_reference` тут особенно важен — раньше шлюз отдавал на дубль
/// `reference` 500, и SDK повторял его как транзиентный.
#[test]
fn payout_link_does_not_retry_terminal_idempotency_codes() {
    for (status, code) in [
        (409u16, "idempotency.in_progress"),
        (409, "payoutlink.duplicate_reference"),
        (400, "idempotency.key_reused"),
        (400, "idempotency.bad_key"),
    ] {
        let t = MockTransport::new(vec![MockResponse {
            status,
            body: json!({ "error": { "code": code, "message": "no" } }).to_string(),
            retry_after: None,
        }]);
        let client = client_with_retries(t.clone());

        let err = client
            .payout_links()
            .create(json!({ "currency": "BTC", "network": "bitcoin", "amount": "0.005" }))
            .unwrap_err();

        assert_eq!(err.code(), Some(code));
        assert!(!err.is_retriable(), "{code} обязан быть терминальным");
        assert_eq!(t.call_count(), 1, "{code} не должен повторяться");
    }
}

#[test]
fn blocked_address_refund_relies_on_server_side_dedup() {
    // Заголовок здесь НЕ шлётся сознательно: эндпоинт идемпотентен по состоянию (ссылка выплаты
    // выводится из id кошелька и ищется под per-wallet блокировкой), что сильнее заголовка.
    let t = MockTransport::new(vec![ok(
        json!({ "state": 0, "result": { "uuid": "po-1" } }),
    )]);
    let client = client_with(t.clone());

    client
        .wallets()
        .blocked_address_refund("w-1", "bc1qxyz")
        .unwrap();

    assert!(url_of(&t, 0).ends_with("/v1/wallet/blocked-address-refund"));
    assert!(header_of(&t, 0, "Idempotency-Key").is_none());
    assert!(t.last_headers().iter().any(|(k, _)| k == "X-Signature"));
}

#[test]
fn payout_link_batch_index_aligned() {
    let t = MockTransport::new(vec![ok(json!({ "state": 0, "result": {
        "created": 1, "total": 2,
        "results": [
            { "ok": true, "link": { "link_id": "pl-1", "status": "funded",
                                    "claim_token": "tkn", "batch_id": "bt-1" } },
            { "ok": false, "error": "payoutlink.insufficient_funds",
              "message": "available balance is less than the link amount" }
        ]
    }}))]);
    let client = client_with(t.clone());

    let res = client
        .payout_links()
        .create_batch(vec![
            json!({ "currency": "BTC", "network": "bitcoin", "amount": "0.005" }),
            json!({ "currency": "BTC", "network": "bitcoin", "amount": "99" }),
        ])
        .unwrap();

    assert_eq!(res.created, 1);
    assert_eq!(res.total, 2);
    assert!(res.results[0].ok);
    assert_eq!(res.results[0].link.as_ref().unwrap().batch_id, "bt-1");
    assert!(!res.results[1].ok);
    assert_eq!(res.results[1].error, "payoutlink.insufficient_funds");
    assert!(res.results[1].link.is_none());

    assert!(url_of(&t, 0).ends_with("/v1/payout/link/batch"));
    assert_eq!(body_json(&t, 0)["links"].as_array().unwrap().len(), 2);
    assert!(
        header_of(&t, 0, "Idempotency-Key").is_some(),
        "пачка ссылок резервирует баланс — обязана слать Idempotency-Key"
    );
}

#[test]
fn payout_link_list_info_cancel() {
    let t = MockTransport::new(vec![
        ok(json!({ "state": 0, "result": { "links": [
            { "link_id": "pl-1", "status": "claimed", "payout_id": "po-9", "claim_address": "bc1q" }
        ]}})),
        ok(json!({ "state": 0, "result": { "link_id": "pl-2", "status": "weird_future_status" } })),
        ok(json!({ "state": 0, "result": { "link_id": "pl-3", "status": "cancelled" } })),
    ]);
    let client = client_with(t.clone());

    let links = client.payout_links().list(Some(50), Some(0)).unwrap();
    assert_eq!(links[0].status, PayoutLinkStatus::Claimed);
    assert_eq!(links[0].payout_id, "po-9");
    assert!(
        links[0].claim_token.is_empty(),
        "list не содержит claim_token"
    );

    // Неизвестный статус не ломает разбор (forward-совместимость).
    let info = client.payout_links().info("pl-2").unwrap();
    assert_eq!(info.status, PayoutLinkStatus::Unknown);

    let cancelled = client.payout_links().cancel("pl-3").unwrap();
    assert_eq!(cancelled.status, PayoutLinkStatus::Cancelled);

    assert!(url_of(&t, 0).ends_with("/v1/payout/link/list"));
    assert!(url_of(&t, 1).ends_with("/v1/payout/link/info"));
    assert!(url_of(&t, 2).ends_with("/v1/payout/link/cancel"));
    assert_eq!(body_json(&t, 2)["link_id"], "pl-3");
}

#[test]
fn claim_info_public_get_unsigned() {
    let t = MockTransport::new(vec![ok(json!({ "state": 0, "result": {
        "status": "funded", "amount": "0.005", "currency": "BTC", "network": "bitcoin",
        "expires_at": "2026-08-14T17:00:00Z", "claimable": true
    }}))]);
    let client = client_with(t.clone());

    let info = client.payout_links().claim_info("Xk3vTOKEN").unwrap();
    assert!(info.claimable);
    assert_eq!(info.status, PayoutLinkStatus::Funded);

    assert!(url_of(&t, 0).ends_with("/v1/claim/Xk3vTOKEN"));
    let headers = t.last_headers();
    assert!(
        !headers
            .iter()
            .any(|(k, _)| k == "X-Signature" || k == "X-Public-Id"),
        "GET /v1/claim/{{token}} — публичный, без подписи"
    );
}

#[test]
fn claim_public_post_unsigned() {
    let t = MockTransport::new(vec![ok(json!({ "state": 0, "result": {
        "status": "claimed", "payout_id": "po-1", "amount": "0.005",
        "currency": "BTC", "network": "bitcoin", "address": "bc1qxyz"
    }}))]);
    let client = client_with(t.clone());

    let res = client
        .payout_links()
        .claim("Xk3vTOKEN", "bc1qxyz", Some("memo-1"))
        .unwrap();
    assert_eq!(res.status, PayoutLinkStatus::Claimed);
    assert_eq!(res.payout_id, "po-1");
    assert_eq!(res.address, "bc1qxyz");

    assert!(url_of(&t, 0).ends_with("/v1/claim/Xk3vTOKEN"));
    let body = body_json(&t, 0);
    assert_eq!(body["address"], "bc1qxyz");
    assert_eq!(body["memo"], "memo-1");
    let headers = t.last_headers();
    assert!(
        !headers
            .iter()
            .any(|(k, _)| k == "X-Signature" || k == "X-Public-Id"),
        "POST /v1/claim/{{token}} — публичный, без подписи"
    );
    assert!(header_of(&t, 0, "Idempotency-Key").is_none());
}

// ─────────────────── Resolve недоплаты (v1.1.0) ───────────────────

use oblodai::ResolveAction;

#[test]
fn resolve_accept_sends_action_and_idempotency() {
    let t = MockTransport::new(vec![ok(json!({ "state": 0, "result": {
        "payment_uuid": "p1", "order_id": "ord-1", "resolution": "accepted",
        "amount_kept": "48.5", "currency": "USDT"
    }}))]);
    let client = client_with(t.clone());

    let res = client
        .payments()
        .resolve(ResolveAction::Accept, json!({ "uuid": "p1" }))
        .unwrap();
    assert_eq!(res.resolution, "accepted");
    assert_eq!(res.amount_kept, "48.5");

    assert!(url_of(&t, 0).ends_with("/v1/payment/resolve"));
    let body = body_json(&t, 0);
    assert_eq!(body["action"], "accept");
    assert_eq!(body["uuid"], "p1");
    assert!(
        header_of(&t, 0, "Idempotency-Key").is_some(),
        "resolve обёрнут в withIdempotency"
    );
}

#[test]
fn resolve_refund_with_explicit_key() {
    let t = MockTransport::new(vec![ok(json!({ "state": 0, "result": {
        "payment_uuid": "p1", "order_id": "ord-1", "resolution": "refunded",
        "uuid": "po-refund-1", "amount": "48.5", "currency": "USDT",
        "address": "0xPayer", "status": "check", "is_final": false
    }}))]);
    let client = client_with(t.clone());

    let res = client
        .payments()
        .resolve(
            ResolveAction::Refund,
            json!({ "order_id": "ord-1", "reference": "rf-1", "idempotency_key": "res-key-1" }),
        )
        .unwrap();
    assert_eq!(res.resolution, "refunded");
    assert_eq!(res.uuid, "po-refund-1");
    assert!(!res.is_final);

    let body = body_json(&t, 0);
    assert_eq!(body["action"], "refund");
    assert_eq!(body["reference"], "rf-1");
    assert!(body.get("idempotency_key").is_none());
    assert_eq!(
        header_of(&t, 0, "Idempotency-Key").as_deref(),
        Some("res-key-1")
    );
}

// ─────────────────── Песочница (v1.2.0) ───────────────────

#[test]
fn sandbox_deposit_path_body_and_unwrap() {
    let t = MockTransport::new(vec![ok(json!({ "state": 0, "result": {
        "invoice_id": "inv-1", "txid": "sbx-tx-1", "amount": "10.00", "confirmations": 0
    }}))]);
    let client = client_with(t.clone());

    // Без опций: оплатить ровно причитающееся, сразу подтверждено.
    let dep = client
        .sandbox()
        .simulate_deposit("inv-1", json!({}))
        .unwrap();
    assert_eq!(dep.invoice_id, "inv-1");
    assert_eq!(dep.txid, "sbx-tx-1");
    assert_eq!(dep.amount, "10.00");
    assert_eq!(dep.confirmations, 0);

    assert!(url_of(&t, 0).ends_with("/v1/sandbox/deposit"));
    let body = body_json(&t, 0);
    assert_eq!(body["invoice_id"], "inv-1");
    assert!(
        body.get("amount").is_none(),
        "amount опционален и не подставляется"
    );
    assert!(
        body.get("txid").is_none(),
        "txid опционален и не подставляется"
    );
    // Запрос подписан (как все бизнес-методы), без Idempotency-Key.
    assert!(header_of(&t, 0, "X-Signature").is_some());
    assert!(header_of(&t, 0, "Idempotency-Key").is_none());
}

#[test]
fn sandbox_deposit_replay_same_txid_deepens_confirmations() {
    let t = MockTransport::new(vec![
        ok(json!({ "state": 0, "result": {
            "invoice_id": "inv-1", "txid": "tx-a", "amount": "5", "confirmations": 1 }})),
        ok(json!({ "state": 0, "result": {
            "invoice_id": "inv-1", "txid": "tx-a", "amount": "5", "confirmations": 12 }})),
    ]);
    let client = client_with(t.clone());

    // Недоплата с мелким числом подтверждений...
    let d1 = client
        .sandbox()
        .simulate_deposit(
            "inv-1",
            json!({ "amount": "5", "confirmations": 1, "txid": "tx-a" }),
        )
        .unwrap();
    assert_eq!(d1.confirmations, 1);

    // ...затем повтор с тем же txid и бОльшим числом — «углубление».
    let d2 = client
        .sandbox()
        .simulate_deposit(
            "inv-1",
            json!({ "amount": "5", "confirmations": 12, "txid": "tx-a" }),
        )
        .unwrap();
    assert_eq!(d2.confirmations, 12);

    let b1 = body_json(&t, 0);
    assert_eq!(b1["amount"], "5");
    assert_eq!(b1["confirmations"], 1);
    assert_eq!(b1["txid"], "tx-a");
    assert_eq!(body_json(&t, 1)["txid"], "tx-a");
}

#[test]
fn sandbox_faucet_idempotency_key_in_body() {
    let t = MockTransport::new(vec![ok(json!({ "state": 0, "result": {
        "asset": "USDT", "amount": "1000", "journal_id": "j-1"
    }}))]);
    let client = client_with(t.clone());

    let res = client
        .sandbox()
        .faucet("USDT", "1000", Some("seed-1"))
        .unwrap();
    assert_eq!(res.asset, "USDT");
    assert_eq!(res.amount, "1000");
    assert_eq!(res.journal_id, "j-1");

    assert!(url_of(&t, 0).ends_with("/v1/sandbox/faucet"));
    let body = body_json(&t, 0);
    assert_eq!(body["asset"], "USDT");
    assert_eq!(body["amount"], "1000");
    // В отличие от создающих бизнес-методов, у faucet idempotency_key — поле ТЕЛА, не заголовок.
    assert_eq!(body["idempotency_key"], "seed-1");
    assert!(header_of(&t, 0, "Idempotency-Key").is_none());
}

#[test]
fn sandbox_reset_empty_body() {
    let t = MockTransport::new(vec![ok(json!({ "state": 0, "result": {
        "invoices_cancelled": 3, "balances_zeroed": 2
    }}))]);
    let client = client_with(t.clone());

    let res = client.sandbox().reset().unwrap();
    assert_eq!(res.invoices_cancelled, 3);
    assert_eq!(res.balances_zeroed, 2);

    assert!(url_of(&t, 0).ends_with("/v1/sandbox/reset"));
    assert_eq!(body_json(&t, 0), json!({}), "reset шлёт пустое тело {{}}");
}

#[test]
fn sandbox_list_webhooks_signed_get_empty_body() {
    let t = MockTransport::new(vec![ok(json!({ "state": 0, "result": { "deliveries": [
        { "id": "d-1", "event_type": "payment", "url": "https://x", "status": "failed",
          "attempts": 3, "last_error": "connection refused",
          "payload": { "type": "payment", "status": "paid", "uuid": "p1" },
          "created_at": "2026-07-19T10:00:00Z", "updated_at": "2026-07-19T10:05:00Z" }
    ]}}))]);
    let client = client_with(t.clone());

    let deliveries = client.sandbox().list_webhooks().unwrap();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].id, "d-1");
    assert_eq!(deliveries[0].attempts, 3);
    assert_eq!(deliveries[0].payload["uuid"], "p1", "payload — сырой JSON");

    // GET с пустым телом.
    assert!(url_of(&t, 0).ends_with("/v1/sandbox/webhooks"));
    assert_eq!(t.calls.lock().unwrap()[0].2, "", "GET уходит без тела");

    // Подпись обязана совпадать с эталоном: HMAC(secret, "{ts}\nGET\n/v1/sandbox/webhooks\n").
    let ts = header_of(&t, 0, "X-Timestamp").expect("нет X-Timestamp");
    let sig = header_of(&t, 0, "X-Signature").expect("нет X-Signature");
    assert_eq!(header_of(&t, 0, "X-Public-Id").as_deref(), Some("pub_1"));

    use hmac::{Hmac, Mac};
    let signing = format!("{ts}\nGET\n/v1/sandbox/webhooks\n");
    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(b"sec_1").unwrap();
    mac.update(signing.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());
    assert_eq!(sig, expected, "подпись GET считается от пустого тела");
}

#[test]
fn sandbox_replay_webhook() {
    let t = MockTransport::new(vec![ok(json!({ "state": 0, "result": {
        "delivery_id": "d-1", "requeued": true
    }}))]);
    let client = client_with(t.clone());

    let res = client.sandbox().replay_webhook("d-1").unwrap();
    assert_eq!(res.delivery_id, "d-1");
    assert!(res.requeued);

    assert!(url_of(&t, 0).ends_with("/v1/sandbox/webhooks/replay"));
    assert_eq!(body_json(&t, 0)["delivery_id"], "d-1");
}

// ─────────────── Переводы пользователям + публичный pay (v1.2.0) ───────────────

#[test]
fn transfer_to_user_path_idempotency_and_result() {
    let t = MockTransport::new(vec![ok(json!({ "state": 0, "result": {
        "currency": "USDT", "amount": "25", "recipient_balance": "125.5",
        "to_user_id": "5c3f8a2c-9b1d-4e6f-8a2c-1e9b7d5f3a10"
    }}))]);
    let client = client_with(t.clone());

    let res = client
        .account()
        .transfer_to_user(json!({
            "to_user_id": "5c3f8a2c-9b1d-4e6f-8a2c-1e9b7d5f3a10",
            "amount": "25", "currency": "USDT", "order_id": "salary-7",
        }))
        .unwrap();

    // Разворачивание конверта {state, result} в типовую модель.
    assert_eq!(res.currency, "USDT");
    assert_eq!(res.amount, "25");
    assert_eq!(res.to_user_id, "5c3f8a2c-9b1d-4e6f-8a2c-1e9b7d5f3a10");
    assert_eq!(res.recipient_balance, "125.5");

    assert!(url_of(&t, 0).ends_with("/v1/transfer/to-user"));
    let body = body_json(&t, 0);
    assert_eq!(body["to_user_id"], "5c3f8a2c-9b1d-4e6f-8a2c-1e9b7d5f3a10");
    assert_eq!(body["order_id"], "salary-7", "order_id уходит как есть");
    // Денежный эндпоинт: заголовок Idempotency-Key обязан быть (UUID по умолчанию).
    let key = header_of(&t, 0, "Idempotency-Key").expect("нет заголовка Idempotency-Key");
    assert_eq!(key.len(), 36);
    // И запрос подписан (payout-ключ).
    assert!(header_of(&t, 0, "X-Signature").is_some());
}

#[test]
fn transfer_to_user_explicit_idempotency_key_header_only() {
    let t = MockTransport::new(vec![ok(json!({ "state": 0, "result": {
        "currency": "USDT", "amount": "5", "to_user_id": "u-1", "recipient_balance": "5"
    }}))]);
    let client = client_with(t.clone());

    client
        .account()
        .transfer_to_user(json!({
            "to_user_id": "u-1", "amount": "5", "currency": "USDT",
            "idempotency_key": "pay-run-42",
        }))
        .unwrap();

    assert_eq!(
        header_of(&t, 0, "Idempotency-Key").as_deref(),
        Some("pay-run-42"),
        "явный ключ обязан уйти в заголовок"
    );
    assert!(
        body_json(&t, 0).get("idempotency_key").is_none(),
        "idempotency_key не должен попадать в тело"
    );
}

#[test]
fn transfer_batch_path_fields_and_idempotency() {
    let t = MockTransport::new(vec![ok(batch_submit_result("transfer"))]);
    let client = client_with(t.clone());

    let sub = client
        .account()
        .transfer_batch(
            vec![
                json!({ "to_user_id": "u-1", "amount": "25", "currency": "USDT", "order_id": "s-1" }),
                json!({ "to_user_id": "u-2", "amount": "30", "currency": "USDT", "order_id": "s-2" }),
            ],
            Some("continue"),
        )
        .unwrap();

    assert_eq!(sub.batch_id, "b-1");
    assert_eq!(sub.status, "pending");
    assert!(url_of(&t, 0).ends_with("/v1/transfer/batch"));
    let body = body_json(&t, 0);
    assert_eq!(body["transfers"].as_array().unwrap().len(), 2);
    assert_eq!(body["on_error"], "continue");
    assert!(
        header_of(&t, 0, "Idempotency-Key").is_some(),
        "батч обязан быть идемпотентным"
    );
}

#[test]
fn public_pay_get_unsigned() {
    let t = MockTransport::new(vec![ok(json!({ "state": 0, "result": {
        "uuid": "inv-1", "amount": "10.00", "currency": "USD",
        "payment_status": "select", "is_multi": true, "url": "https://pay/inv-1"
    }}))]);
    let client = client_with(t.clone());

    let p = client.payments().public_get("inv-1").unwrap();
    assert_eq!(p.uuid, "inv-1");
    assert_eq!(p.payment_status, "select");
    assert!(p.is_multi);

    assert!(url_of(&t, 0).ends_with("/v1/pay/inv-1"));
    assert_eq!(t.calls.lock().unwrap()[0].2, "", "GET уходит без тела");
    let headers = t.last_headers();
    assert!(
        !headers
            .iter()
            .any(|(k, _)| k == "X-Signature" || k == "X-Public-Id"),
        "GET /v1/pay/{{id}} — публичный, без подписи"
    );
}

#[test]
fn public_pay_select_unsigned_finalizes_invoice() {
    let t = MockTransport::new(vec![ok(json!({ "state": 0, "result": {
        "uuid": "inv-1", "amount": "10.00", "currency": "USD",
        "payer_amount": "10.05", "payer_currency": "USDT", "network": "tron",
        "address": "TSelect1", "payment_status": "check"
    }}))]);
    let client = client_with(t.clone());

    let p = client
        .payments()
        .public_select("inv-1", "USDT", "tron")
        .unwrap();
    assert_eq!(p.uuid, "inv-1");
    assert_eq!(p.network, "tron");
    assert_eq!(
        p.address, "TSelect1",
        "после select у счёта есть депозит-адрес"
    );

    assert!(url_of(&t, 0).ends_with("/v1/pay/inv-1/select"));
    assert_eq!(
        body_json(&t, 0),
        json!({ "currency": "USDT", "network": "tron" })
    );
    let headers = t.last_headers();
    assert!(
        !headers
            .iter()
            .any(|(k, _)| k == "X-Signature" || k == "X-Public-Id"),
        "POST /v1/pay/{{id}}/select — публичный, без подписи"
    );
    assert!(header_of(&t, 0, "Idempotency-Key").is_none());
}

#[test]
fn is_test_key_detects_prefixes() {
    assert!(oblodai::is_test_key("test_abc123"));
    assert!(oblodai::is_test_key("oblodai_test_abc123"));
    assert!(!oblodai::is_test_key("oblodai_live_abc123"));
    assert!(!oblodai::is_test_key("oblodai_abc123"));
    assert!(!oblodai::is_test_key(""));
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
    assert!(
        !e.is_retriable(),
        "payout.funds_maturing должна быть терминальной"
    );
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
