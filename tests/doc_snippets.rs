//! Every Rust snippet in README.md, AGENTS.md and MIGRATION-1.3.md, compiled.
//!
//! Documentation that does not compile is documentation that is wrong, and these three files are
//! the first thing both a human and an LLM read. Each function below is one snippet, copied
//! verbatim apart from the wrapper it needs (a signature, and stubs for the caller's own helpers).
//! If a snippet changes, change it here too — the compiler is the reviewer.

// A `Client` only exists with an HTTP backend feature on.
#![cfg(feature = "blocking")]
#![allow(dead_code, unused_variables, clippy::result_large_err)]

// The SDK's error carries the gateway's whole envelope by value, exactly as callers match on it.

use std::time::Duration;

use futures_util::StreamExt;
use oblodai::contract::requests::{PaymentRequest, PayoutHistoryRequest, PayoutRequest};
use oblodai::{Client, PayoutKind};

fn mark_order_paid(_order_id: Option<&str>) {}
fn schedule_retry(_seconds: u64) {}

// --- README: start in the sandbox -------------------------------------------------------------

async fn readme_quickstart() -> oblodai::Result<()> {
    let client = Client::from_env()?;

    let invoice = client
        .payments()
        .create(PaymentRequest {
            amount: "25".into(),
            currency: "USDT".into(),
            network: Some("tron".into()),
            order_id: Some("order-1001".into()),
            url_callback: Some("https://shop.example/oblodai/webhook".into()),
            ..Default::default()
        })
        .await?;

    println!("{} {} {}", invoice.url, invoice.address, invoice.status);
    Ok(())
}

// --- README: two keys ---------------------------------------------------------------------------

fn readme_two_keys(
    public_id: &str,
    secret: &str,
    payout_public_id: &str,
    payout_secret: &str,
) -> oblodai::Result<()> {
    let client = oblodai::Client::builder()
        .public_id(public_id)
        .secret(secret)
        .payout_public_id(payout_public_id)
        .payout_secret(payout_secret)
        .build()?;
    let _ = client;
    Ok(())
}

// --- README: lists ------------------------------------------------------------------------------

async fn readme_lists(client: &Client) -> oblodai::Result<()> {
    // one page
    let page = client
        .payments()
        .history(Default::default())
        .limit(50)
        .await?;
    println!("{} of {}", page.items.len(), page.paginate.total);

    // every item, one page fetched at a time
    let mut payouts = client.payouts().history(Default::default()).stream();
    while let Some(payout) = payouts.next().await {
        println!("{}", payout?.uuid);
    }

    // or collect, with a cap
    let refunds = client
        .payouts()
        .history(PayoutHistoryRequest {
            kind: Some(PayoutKind::Refund),
            ..Default::default()
        })
        .all(Some(1000))
        .await?;
    let _ = refunds;
    Ok(())
}

// --- README: errors -----------------------------------------------------------------------------

async fn readme_errors(client: &Client, params: PayoutRequest) -> oblodai::Result<oblodai::Payout> {
    match client.payouts().create(params).await {
        Ok(payout) => Ok(payout),
        Err(err) => match err.code() {
            // retryable — the balance may still arrive
            "payout.insufficient_funds" | "payout.funds_maturing" => {
                schedule_retry(err.retry_after().unwrap_or(60));
                Err(err)
            }
            _ => Err(err), // the SDK already retried what was safe to retry
        },
    }
}

// --- README / AGENTS / MIGRATION: webhooks ------------------------------------------------------

fn readme_webhooks(
    request_headers: Vec<(String, String)>,
    raw_body: &[u8],
    secret: &str,
) -> oblodai::Result<()> {
    use oblodai::webhooks::{verify_webhook_delivery, Headers, VerifyOptions};

    let headers = Headers::from_pairs(request_headers); // any (name, value) pairs
    let delivery = verify_webhook_delivery(raw_body, &headers, &VerifyOptions::new(secret))?;

    if delivery.is_test {
        return Ok(()); // a rehearsal: signed like a live one, but nothing moved
    }

    match &delivery.event {
        oblodai::WebhookEvent::Payment(p) if p.status == oblodai::PaymentStatus::Paid => {
            mark_order_paid(p.order_id.as_deref())
        }
        _ => {}
    }
    Ok(())
}

fn agents_webhooks(
    raw_body: &[u8],
    headers: Vec<(String, String)>,
    secret: &str,
) -> oblodai::Result<()> {
    use oblodai::webhooks::{verify_webhook_delivery, Headers, VerifyOptions};
    let delivery = verify_webhook_delivery(
        raw_body,
        &Headers::from_pairs(headers),
        &VerifyOptions::new(secret),
    )?;
    let _ = delivery;
    Ok(())
}

// --- README: blocking client --------------------------------------------------------------------

fn readme_blocking(params: PaymentRequest) -> oblodai::Result<()> {
    let client = oblodai::blocking::Client::from_env()?;
    let invoice = client.payments().create(params).send()?;
    for payout in client.payouts().history(Default::default()).iter() {
        println!("{}", payout?.uuid);
    }
    let _ = invoice;
    Ok(())
}

// --- MIGRATION: per-call options ----------------------------------------------------------------

async fn migration_per_call_options(client: &Client, params: PayoutRequest) -> oblodai::Result<()> {
    client
        .payouts()
        .create(params)
        .idempotency_key("payout-42")
        .timeout(Duration::from_secs(10)) // one attempt
        .deadline(Duration::from_secs(45)) // the whole call, retries and pauses included
        .prefer_payout_key(true)
        .header("X-Request-Trace", "abc123") // this call only
        .await?;
    Ok(())
}

/// The snippets are compiled, not run — a run would need a gateway. This keeps the file a test
/// target rather than a dead module.
#[test]
fn every_documented_snippet_compiles() {
    let _ = readme_quickstart;
    let _ = readme_two_keys;
    let _ = readme_lists;
    let _ = readme_errors;
    let _ = readme_webhooks;
    let _ = agents_webhooks;
    let _ = readme_blocking;
    let _ = migration_per_call_options;
}
