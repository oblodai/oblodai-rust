//! The money path against a REAL core.
//!
//! Ignored by default. Point `OBLODAI_LIVE_URL` at a running gateway and run it explicitly:
//!
//! ```sh
//! OBLODAI_LIVE_URL=http://127.0.0.1:8095 cargo test --test live_sandbox -- --ignored --nocapture
//! ```
//!
//! The test onboards its own merchant, takes a sandbox key and walks the journey, so it needs no
//! fixtures and leaves no shared state behind.

// A `Client` only exists with an HTTP backend feature on.
#![cfg(feature = "reqwest-client")]

mod support;

use std::time::{SystemTime, UNIX_EPOCH};

use oblodai::contract::requests::{
    PaymentRequest, PayoutCalculateRequest, PayoutRequest, PayoutValidateRequest,
    SandboxDepositRequest, SandboxFaucetRequest,
};
use oblodai::helpers::is_payment_paid;
use oblodai::resources::PageParams;
use oblodai::{Client, ErrorKind, Lookup};

const ADDRESS: &str = "TQrY8bkbpXKPt2LZbU8jqfnpFbUSF15sbx";

fn base_url() -> String {
    std::env::var("OBLODAI_LIVE_URL").expect("set OBLODAI_LIVE_URL to run the live tests")
}

fn stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

fn anonymous() -> Client {
    Client::builder()
        .base_url(base_url())
        .allow_insecure_base_url(true)
        .env(Vec::<(String, String)>::new())
        .build()
        .unwrap()
}

/// Onboard a merchant and take its sandbox key — the same two unsigned calls a platform makes.
async fn onboard_sandbox() -> Client {
    let public = anonymous();
    let merchant = public
        .merchants()
        .create(oblodai::contract::requests::MerchantsRequest {
            email: format!("sdk-live-{}@example.com", stamp()),
            name: Some("SDK live".into()),
        })
        .await
        .expect("onboarding is open on a local core");
    let sandbox = public
        .merchants()
        .create_sandbox(&merchant.merchant_id)
        .await
        .unwrap();
    Client::builder()
        .public_id(sandbox.api_key.public_id)
        .secret(sandbox.api_key.secret)
        .base_url(base_url())
        .allow_insecure_base_url(true)
        .env(Vec::<(String, String)>::new())
        .build()
        .unwrap()
}

#[tokio::test]
#[ignore = "needs a running gateway at OBLODAI_LIVE_URL"]
async fn live_sandbox_journey() {
    let client = onboard_sandbox().await;

    // --- public catalogue, no credentials ---
    let currencies = anonymous().catalog().currencies().await.unwrap();
    assert!(
        !currencies.currencies.is_empty(),
        "the catalogue is never empty"
    );

    // --- create an invoice, read it back both ways ---
    let order_id = format!("sdk-live-{}", stamp());
    let invoice = client
        .payments()
        .create(PaymentRequest {
            amount: "25".into(),
            currency: "USDT".into(),
            network: Some("tron".into()),
            order_id: Some(order_id.clone()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(invoice.status, oblodai::PaymentStatus::Created);
    assert_eq!(invoice.order_id, order_id);

    let by_order = client
        .payments()
        .info(Lookup::order_id(&order_id))
        .await
        .unwrap();
    assert_eq!(by_order.uuid, invoice.uuid);
    let by_uuid = client.payments().get(&invoice.uuid).await.unwrap();
    assert_eq!(by_uuid.order_id, order_id);

    let page = client
        .payments()
        .history(Default::default())
        .limit(5)
        .await
        .unwrap();
    assert!(
        page.items.iter().any(|p| p.uuid == invoice.uuid),
        "the invoice is in the history"
    );

    // A signed GET with a query string — the signature covers path + query.
    let hooks = client
        .sandbox()
        .webhooks(PageParams::default())
        .limit(5)
        .offset(0)
        .await
        .unwrap();
    assert!(hooks.paginate.per_page > 0);

    // --- idempotency: a replay returns the first answer, a different body is a conflict ---
    let key = format!("sdk-idem-{}", stamp());
    let first = client
        .payments()
        .create(PaymentRequest {
            amount: "1".into(),
            currency: "USDT".into(),
            network: Some("tron".into()),
            order_id: Some(format!("{key}-o")),
            ..Default::default()
        })
        .idempotency_key(&key)
        .await
        .unwrap();
    let replay = client
        .payments()
        .create(PaymentRequest {
            amount: "1".into(),
            currency: "USDT".into(),
            network: Some("tron".into()),
            order_id: Some(format!("{key}-o")),
            ..Default::default()
        })
        .idempotency_key(&key)
        .await
        .unwrap();
    assert_eq!(
        replay.uuid, first.uuid,
        "the same key replays the same invoice"
    );

    let conflict = client
        .payments()
        .create(PaymentRequest {
            amount: "2".into(),
            currency: "USDT".into(),
            network: Some("tron".into()),
            order_id: Some(format!("{key}-o2")),
            ..Default::default()
        })
        .idempotency_key(&key)
        .await
        .unwrap_err();
    assert_eq!(conflict.code(), "idempotency.key_reused");
    assert_eq!(conflict.http_status(), 409);
    assert_eq!(conflict.kind(), ErrorKind::IdempotencyConflict);

    // --- simulate a deposit, watch the invoice settle ---
    client
        .sandbox()
        .deposit(SandboxDepositRequest {
            invoice_id: invoice.uuid.clone(),
            amount: Some("25".into()),
            confirmations: Some(20),
            txid: Some(format!("sdk-tx-{}", stamp())),
        })
        .await
        .unwrap();
    let paid = client.payments().info(&invoice.uuid).await.unwrap();
    assert!(
        is_payment_paid(&paid.status),
        "status after the deposit: {}",
        paid.status
    );

    // --- fund the balance and move money out ---
    client
        .sandbox()
        .faucet(SandboxFaucetRequest {
            asset: "USDT".into(),
            amount: "100".into(),
            ..Default::default()
        })
        .await
        .unwrap();
    let balance = client.account().balance().await.unwrap();
    assert!(
        balance
            .balance
            .merchant
            .iter()
            .any(|b| b.currency == "USDT"),
        "the faucet credited USDT"
    );

    let calculation = client
        .payouts()
        .calculate(PayoutCalculateRequest {
            amount: "10".into(),
            currency: "USDT".into(),
            network: Some("tron".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(calculation.currency, "USDT");

    let validation = client
        .payouts()
        .validate(PayoutValidateRequest {
            amount: "10".into(),
            currency: "USDT".into(),
            network: Some("tron".into()),
            address: ADDRESS.into(),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(validation.valid, "the dry run passes before the real thing");

    let payout_order = format!("sdk-po-{}", stamp());
    let payout = client
        .payouts()
        .create(PayoutRequest {
            amount: "10".into(),
            currency: "USDT".into(),
            network: Some("tron".into()),
            address: ADDRESS.into(),
            order_id: payout_order.clone(),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(!payout.uuid.is_empty());
    let fetched = client.payouts().info(&payout.uuid).await.unwrap();
    assert_eq!(fetched.order_id.as_deref(), Some(payout_order.as_str()));

    // --- a domain refusal keeps the core's own classification ---
    let refusal = client
        .payouts()
        .create(PayoutRequest {
            amount: "999999".into(),
            currency: "USDT".into(),
            network: Some("tron".into()),
            address: ADDRESS.into(),
            order_id: format!("sdk-big-{}", stamp()),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert_eq!(refusal.code(), "payout.insufficient_funds");
    assert_eq!(refusal.http_status(), 409);
    assert!(
        refusal.request_id().is_some(),
        "a refusal always carries a request id"
    );
}

/// A one-shot HTTP receiver: it accepts a single delivery and hands back its headers and bytes.
async fn catch_one_delivery(listener: tokio::net::TcpListener) -> (Vec<(String, String)>, Vec<u8>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (mut socket, _) = listener.accept().await.expect("the gateway connected");
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let read = socket.read(&mut chunk).await.unwrap();
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        let Some(head_end) = buffer.windows(4).position(|w| w == b"\r\n\r\n") else {
            continue;
        };
        let head = String::from_utf8_lossy(&buffer[..head_end]).to_string();
        let length: usize = head
            .lines()
            .find(|l| l.to_ascii_lowercase().starts_with("content-length:"))
            .and_then(|l| l.split(':').nth(1))
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0);
        if buffer.len() >= head_end + 4 + length {
            let headers = head
                .lines()
                .skip(1)
                .filter_map(|line| line.split_once(':'))
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
                .collect();
            let body = buffer[head_end + 4..head_end + 4 + length].to_vec();
            let _ = socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                .await;
            let _ = socket.flush().await;
            return (headers, body);
        }
    }
    panic!("the delivery was cut short");
}

/// End to end: the core signs a real delivery and this SDK's verifier accepts it — over the exact
/// bytes that arrived, with nothing re-serialized in between.
#[tokio::test]
#[ignore = "needs a running gateway at OBLODAI_LIVE_URL"]
async fn live_webhook_delivery_verifies_against_the_endpoint_secret() {
    use oblodai::webhooks::{verify_webhook_delivery, Headers, VerifyOptions};

    let client = onboard_sandbox().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let receiver = format!(
        "http://127.0.0.1:{}/hook",
        listener.local_addr().unwrap().port()
    );

    let endpoint = client.webhooks().register(&receiver).await.unwrap();
    let secret = endpoint
        .secret
        .expect("the secret is shown once, at registration");

    let waiting = tokio::spawn(catch_one_delivery(listener));
    let result = client
        .webhooks()
        .test(
            oblodai::WebhookKind::Payment,
            oblodai::contract::requests::TestWebhookPaymentRequest {
                url_callback: receiver,
                currency: Some("USDT".into()),
                network: Some("tron".into()),
                status: Some(oblodai::PaymentStatus::Paid),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(result.signed, "the core signs even a rehearsal delivery");

    let (headers, body) = tokio::time::timeout(std::time::Duration::from_secs(10), waiting)
        .await
        .expect("the delivery arrived")
        .unwrap();
    let headers = Headers::from_pairs(headers);
    let ts: i64 = headers.get("X-Webhook-Timestamp").unwrap().parse().unwrap();
    let delivery =
        verify_webhook_delivery(&body, &headers, &VerifyOptions::new(&secret).now(ts)).unwrap();
    assert_eq!(delivery.event.event_kind(), "payment");
    assert_eq!(delivery.event.status(), "paid");
    assert!(delivery.id.is_some(), "X-Webhook-Id is the dedup key");

    // The same bytes with the wrong secret must not verify.
    let err =
        verify_webhook_delivery(&body, &headers, &VerifyOptions::new("wrong").now(ts)).unwrap_err();
    assert_eq!(err.code(), "webhook.bad_signature");

    // And a single flipped byte in the body must not verify either.
    let mut tampered = body.clone();
    let last = tampered.len() - 2;
    tampered[last] ^= 0x01;
    assert!(
        verify_webhook_delivery(&tampered, &headers, &VerifyOptions::new(&secret).now(ts)).is_err()
    );
}

#[tokio::test]
#[ignore = "needs a running gateway at OBLODAI_LIVE_URL"]
async fn live_webhook_is_signed_the_way_the_verifier_expects() {
    let Ok(receiver) = std::env::var("OBLODAI_LIVE_HOOK_URL") else {
        eprintln!("skipped: set OBLODAI_LIVE_HOOK_URL to a reachable receiver");
        return;
    };
    let client = onboard_sandbox().await;
    let endpoint = client.webhooks().register(&receiver).await.unwrap();
    assert!(
        endpoint.secret.is_some(),
        "the secret is shown once, at registration"
    );
    let result = client
        .webhooks()
        .test(
            oblodai::WebhookKind::Payment,
            oblodai::contract::requests::TestWebhookPaymentRequest {
                url_callback: receiver,
                currency: Some("USDT".into()),
                network: Some("tron".into()),
                status: Some(oblodai::PaymentStatus::Paid),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(result.signed, "the core signs even a rehearsal delivery");
}
