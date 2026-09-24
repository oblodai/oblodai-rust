//! Every namespace against a REAL gateway.
//!
//! Ignored by default:
//!
//! ```sh
//! OBLODAI_LIVE_URL=http://127.0.0.1:8095 cargo test --test live_sweep -- --ignored --nocapture
//! ```
//!
//! The point is not the business outcome but that the bodies the SDK sends are accepted (no 400
//! from our own shapes) and the bodies that come back decode into the generated models (no contract
//! error). Business refusals (a subsystem the stand lacks, a 403/404/409) are tolerated and logged.

#![cfg(feature = "reqwest-client")]

use std::time::{SystemTime, UNIX_EPOCH};

use oblodai::generated::resources::{
    GetBalanceDocumentQuery, GetStatementDocumentQuery, SandboxListWebhooksQuery,
};
use oblodai::models::{
    APILogRequest, CreateWalletRequest, ExchangeRatesRequest, FaucetRequest, HistoryRequest,
    LookupRequest, PageRequest, PaymentLinkCreateRequest, PaymentRequest, PayoutCalculateRequest,
    PayoutLinkItem, QrRequest, RegisterWebhookRequest, SummaryRequest,
};
use oblodai::{Client, ErrorKind, Result};
use serde_json::{json, Value};

fn base_url() -> String {
    std::env::var("OBLODAI_LIVE_URL").expect("set OBLODAI_LIVE_URL to run the live tests")
}

fn hook_url() -> String {
    std::env::var("OBLODAI_LIVE_HOOK_URL").unwrap_or_else(|_| "http://127.0.0.1:8096/hook".into())
}

fn stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

/// Fails the test on SDK-side shape problems; tolerates business refusals.
fn accept<T>(what: &str, result: Result<T>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(err) if matches!(err.kind(), ErrorKind::Contract | ErrorKind::Validation) => {
            panic!("{what}: the SDK's own shape was rejected — {err}")
        }
        Err(err) => {
            eprintln!("  {what}: refused by the gateway ({err}) — tolerated");
            None
        }
    }
}

fn builder() -> oblodai::ClientBuilder {
    Client::builder()
        .base_url(base_url())
        .allow_insecure_base_url(true)
        .env(Vec::<(String, String)>::new())
}

/// A merchant (`POST /v1/merchants`, open on a dev stand) and its sandbox key.
async fn onboard() -> Client {
    let body = json!({"email": format!("sweep-{}@example.com", stamp()), "name": "Sweep"});
    let answer = reqwest::Client::new()
        .post(format!("{}/v1/merchants", base_url()))
        .header("content-type", "application/json")
        .body(body.to_string())
        .send()
        .await
        .expect("onboarding is open on a dev stand")
        .text()
        .await
        .unwrap();
    let merchant: Value = serde_json::from_str(&answer).unwrap();
    let merchant_id = merchant["result"]["merchant_id"]
        .as_str()
        .unwrap()
        .to_string();
    let store = builder()
        .build()
        .unwrap()
        .sandbox()
        .onboard_store(merchant_id)
        .await
        .unwrap();
    builder()
        .public_id(store.api_key.public_id)
        .secret(store.api_key.secret)
        .build()
        .unwrap()
}

#[tokio::test]
#[ignore = "needs a running gateway at OBLODAI_LIVE_URL"]
async fn live_sweep() {
    let client = onboard().await;
    let public = builder().build().unwrap();
    let hook = hook_url();
    let page = || PageRequest {
        limit: Some(5),
        ..Default::default()
    };

    // --- money in the sandbox, an endpoint, an invoice ---
    client
        .sandbox()
        .faucet(FaucetRequest::new("1000", "USDT"))
        .await
        .unwrap();
    accept(
        "webhooks.register",
        client
            .webhooks()
            .register(RegisterWebhookRequest::new(hook.clone()))
            .await,
    );
    let invoice = client
        .payments()
        .create(PaymentRequest {
            network: Some("tron".into()),
            order_id: Some(format!("sw-{}", stamp())),
            ..PaymentRequest::new("25", "USDT")
        })
        .await
        .unwrap();
    let by_uuid = || LookupRequest {
        uuid: Some(invoice.uuid.clone()),
        ..Default::default()
    };

    // --- payments ---
    accept(
        "payments.get_info",
        client.payments().get_info(by_uuid()).await,
    );
    accept("payments.get_qr", client.payments().get_qr(by_uuid()).await);
    accept(
        "payments.list_history",
        client
            .payments()
            .list_history(HistoryRequest::default())
            .limit(5)
            .await,
    );
    accept(
        "payments.list_services",
        client.payments().list_services(page()).await,
    );
    accept(
        "payments.get_checkout_config",
        client.payments().get_checkout_config().await,
    );

    // --- checkout (no credentials) ---
    accept(
        "checkout.list_currencies",
        public.checkout().list_currencies().await,
    );
    accept(
        "checkout.get",
        public.checkout().get(invoice.uuid.clone()).await,
    );
    accept(
        "checkout.get_qr",
        public.checkout().get_qr(invoice.uuid.clone()).await,
    );

    // --- links ---
    accept(
        "payment_links.create",
        client
            .payment_links()
            .create(PaymentLinkCreateRequest::new("open", "USDT"))
            .await,
    );
    accept(
        "payment_links.list",
        client.payment_links().list(page()).await,
    );
    accept(
        "payout_links.create",
        client
            .payout_links()
            .create(PayoutLinkItem::new("1", "USDT", "tron"))
            .await,
    );
    accept(
        "payout_links.list",
        client.payout_links().list(page()).await,
    );

    // --- payouts ---
    accept(
        "payouts.calculate",
        client
            .payouts()
            .calculate(PayoutCalculateRequest {
                network: Some("tron".into()),
                ..PayoutCalculateRequest::new("10", "USDT")
            })
            .await,
    );
    accept(
        "payouts.list_history",
        client
            .payouts()
            .list_history(HistoryRequest::default())
            .limit(5)
            .await,
    );
    accept(
        "payouts.list_services",
        client.payouts().list_services(page()).await,
    );

    // --- account, settings, splits, wallets, referrals ---
    accept("account.get_balance", client.account().get_balance().await);
    accept(
        "account.get_summary",
        client
            .account()
            .get_summary(SummaryRequest::new("2026-01-01", "2026-12-31"))
            .await,
    );
    accept(
        "account.list_exchange_rates",
        public
            .account()
            .list_exchange_rates(ExchangeRatesRequest::default())
            .limit(5)
            .await,
    );
    accept(
        "settings.get_accuracy",
        client.settings().get_accuracy().await,
    );
    accept(
        "settings.get_auto_refund",
        client.settings().get_auto_refund().await,
    );
    accept(
        "settings.get_auto_convert",
        client.settings().get_auto_convert().await,
    );
    accept(
        "settings.list_discounts",
        client.settings().list_discounts(page()).await,
    );
    accept(
        "settings.list_api_log",
        client
            .settings()
            .list_api_log(APILogRequest::default())
            .await,
    );
    accept(
        "settings.list_accepted_currencies",
        client.settings().list_accepted_currencies(page()).await,
    );
    accept(
        "settings.get_payment_fee_config",
        client.settings().get_payment_fee_config().await,
    );
    accept(
        "settings.get_payout_fee_config",
        client.settings().get_payout_fee_config().await,
    );
    accept(
        "settings.get_refund_fee_config",
        client.settings().get_refund_fee_config().await,
    );
    accept(
        "settings.list_auto_withdraw_rules",
        client.settings().list_auto_withdraw_rules().await,
    );
    accept("api_allowlist.list", client.api_allowlist().list().await);
    accept("splits.get_config", client.splits().get_config().await);
    accept(
        "splits.list_rules",
        client.splits().list_rules(page()).await,
    );
    accept(
        "splits.get_recipient_opt_in",
        client.splits().get_recipient_opt_in().await,
    );
    accept("referrals.get_info", client.referrals().get_info().await);
    if let Some(wallet) = accept(
        "wallets.create",
        client
            .wallets()
            .create(CreateWalletRequest::new("USDT", "tron"))
            .await,
    ) {
        accept(
            "wallets.get_qr",
            client
                .wallets()
                .get_qr(QrRequest::new(wallet.address))
                .await,
        );
    }

    // --- webhooks, sandbox ---
    accept(
        "webhooks.list_deliveries",
        client.webhooks().list_deliveries(page()).await,
    );
    accept(
        "sandbox.list_webhooks",
        client
            .sandbox()
            .list_webhooks(SandboxListWebhooksQuery::default())
            .await,
    );

    // --- documents (tolerated where the stand has them off) ---
    accept(
        "documents.get_balance",
        client
            .documents()
            .get_balance(GetBalanceDocumentQuery::default())
            .await,
    );
    accept(
        "documents.get_statement",
        client
            .documents()
            .get_statement(GetStatementDocumentQuery::default())
            .await,
    );
}
