//! Every Rust snippet in README.md, README.ru.md, AGENTS.md and MIGRATION-1.3.md, compiled.
//!
//! Documentation that does not compile is documentation that is wrong, and these files are the
//! first thing both a human and an LLM read. Each function below holds one snippet between
//! `// @snippet <name>` and `// @end` markers, copied verbatim apart from the wrapper it needs (a
//! signature, and stubs for the caller's own helpers).
//!
//! Two tests keep the files honest without any copying by hand:
//!
//! * [`readme_snippets_match_this_file`] — the fenced `rust` blocks of README.md are, in order,
//!   byte-identical to the marked snippets here once the wrapper's indent is removed.
//! * [`russian_readme_carries_the_same_code`] — README.ru.md carries exactly the same code blocks,
//!   in the same order, byte for byte. Translations drift; code must not.

// A `Client` only exists with an HTTP backend feature on.
#![cfg(feature = "blocking")]
#![allow(dead_code, unused_variables, clippy::result_large_err)]

// The SDK's error carries the gateway's whole envelope by value, exactly as callers match on it.

use std::time::Duration;

use oblodai::contract::requests::{PaymentRequest, PayoutHistoryRequest, PayoutRequest};
use oblodai::{Client, Payment, PayoutKind};

fn mark_order_paid(_order_id: Option<&str>) {}
fn schedule_retry(_seconds: u64) {}

// --- README: installation / blocking client -----------------------------------------------------

fn readme_blocking(params: PaymentRequest) -> oblodai::Result<()> {
    // @snippet readme_blocking
    let client = oblodai::blocking::Client::from_env()?;
    let invoice = client.payments().create(params).send()?;
    for payout in client.payouts().history(Default::default()).iter() {
        println!("{}", payout?.uuid);
    }
    // @end
    let _ = invoice;
    Ok(())
}

// --- README: where to get keys ------------------------------------------------------------------

fn readme_one_key(public_id: &str, secret: &str) -> oblodai::Result<()> {
    // @snippet readme_one_key
    let client = oblodai::Client::builder()
        .public_id(public_id)
        .secret(secret)
        .build()?;
    // @end
    let _ = client;
    Ok(())
}

// --- README: quick start ------------------------------------------------------------------------

async fn readme_quickstart() -> oblodai::Result<()> {
    // @snippet readme_quickstart
    use oblodai::contract::requests::PaymentRequest;
    use oblodai::Client;

    let client = Client::from_env()?;
    let invoice = client
        .payments()
        .create(PaymentRequest {
            amount: "25".into(),                 // amounts are decimal strings, never floats
            currency: "USDT".into(),             // what you price in: a fiat or an asset
            network: Some("tron".into()),        // omit to let the payer choose on the pay page
            order_id: Some("order-1001".into()), // your reference; idempotent per order_id
            url_callback: Some("https://shop.example/oblodai/webhook".into()),
            ..Default::default()
        })
        .await?;
    println!(
        "pay at {} — {} {}",
        invoice.url, invoice.address, invoice.status
    );
    // @end
    Ok(())
}

async fn readme_payout(client: &Client) -> oblodai::Result<()> {
    // @snippet readme_payout
    use oblodai::contract::requests::PayoutRequest;

    let payout = client
        .payouts()
        .create(PayoutRequest {
            amount: "10".into(),
            currency: "USDT".into(),
            network: Some("tron".into()),
            address: "TQrY8bkbpXKPt2LZbU8jqfnpFbUSF15sbx".into(),
            order_id: "payout-1001".into(),
            ..Default::default()
        })
        .idempotency_key("payout-1001") // makes the retry safe across restarts too
        .await?;
    println!("{} {}", payout.uuid, payout.status);
    // @end
    Ok(())
}

// --- README: sandbox / testing ------------------------------------------------------------------

async fn readme_sandbox(client: &Client, invoice: Payment) -> oblodai::Result<()> {
    // @snippet readme_sandbox
    use oblodai::contract::requests::{
        SandboxDepositRequest, SandboxFaucetRequest, TestWebhookPaymentRequest,
    };
    use oblodai::WebhookKind;

    // test money to pay out from (`test_` keys only)
    client
        .sandbox()
        .faucet(SandboxFaucetRequest {
            amount: "1000".into(),
            asset: "USDT".into(),
            ..Default::default()
        })
        .await?;

    // "pay" an invoice; repeat the same txid with more confirmations to walk the pending→paid path
    client
        .sandbox()
        .deposit(SandboxDepositRequest {
            invoice_id: invoice.uuid.clone(),
            ..Default::default()
        })
        .await?;

    // a rehearsal delivery: signed exactly like a live one, and marked `test: true`
    client
        .webhooks()
        .test(
            WebhookKind::Payment,
            TestWebhookPaymentRequest {
                url_callback: "https://shop.example/oblodai/webhook".into(),
                ..Default::default()
            },
        )
        .await?;

    // what was delivered, with payloads — then a clean slate
    let deliveries = client
        .sandbox()
        .webhooks(Default::default())
        .all(None)
        .await?;
    client.sandbox().reset().await?;
    // @end
    Ok(())
}

// --- README: lists ------------------------------------------------------------------------------

async fn readme_lists(client: &Client) -> oblodai::Result<()> {
    // @snippet readme_lists
    use futures_util::StreamExt;

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
    // @end
    let _ = refunds;
    Ok(())
}

// --- README / AGENTS / MIGRATION: webhooks ------------------------------------------------------

fn readme_webhooks(
    request_headers: Vec<(String, String)>,
    raw_body: &[u8],
    secret: &str,
) -> oblodai::Result<()> {
    // @snippet readme_webhooks
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
    // @end
    Ok(())
}

/// AGENTS.md and MIGRATION-1.3.md carry the same call with their own line breaks.
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

// --- README: errors -----------------------------------------------------------------------------

async fn readme_errors(client: &Client, params: PayoutRequest) -> oblodai::Result<oblodai::Payout> {
    // @snippet readme_errors
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
    // @end
}

// --- README / MIGRATION: per-call options -------------------------------------------------------

/// MIGRATION-1.3.md shows the same call with its own line breaks.
async fn readme_per_call_options(client: &Client, params: PayoutRequest) -> oblodai::Result<()> {
    // @snippet readme_per_call_options
    client
        .payouts()
        .create(params)
        .idempotency_key("payout-42")
        .timeout(Duration::from_secs(10)) // one attempt
        .deadline(Duration::from_secs(45)) // the whole call, retries and pauses included
        .header("X-Request-Trace", "abc123") // this call only
        .await?;
    // @end
    Ok(())
}

/// The snippets are compiled, not run — a run would need a gateway. This keeps the file a test
/// target rather than a dead module.
#[test]
fn every_documented_snippet_compiles() {
    let _ = readme_blocking;
    let _ = readme_one_key;
    let _ = readme_quickstart;
    let _ = readme_payout;
    let _ = readme_sandbox;
    let _ = readme_lists;
    let _ = readme_webhooks;
    let _ = agents_webhooks;
    let _ = readme_errors;
    let _ = readme_per_call_options;
}

// --- the two files and this one, kept in step ---------------------------------------------------

/// A fenced block of a markdown file: its info string and its body, verbatim.
#[derive(Debug, PartialEq, Eq)]
struct CodeBlock {
    lang: String,
    body: String,
}

fn code_blocks(markdown: &str) -> Vec<CodeBlock> {
    let mut blocks = Vec::new();
    let mut open: Option<CodeBlock> = None;
    for line in markdown.lines() {
        match (&mut open, line.strip_prefix("```")) {
            // A fence inside an open block closes it; the info string of a closing fence is empty.
            (Some(_), Some(_)) => blocks.push(open.take().expect("open block")),
            (Some(block), None) => {
                block.body.push_str(line);
                block.body.push('\n');
            }
            (None, Some(info)) => {
                open = Some(CodeBlock {
                    lang: info.trim().to_string(),
                    body: String::new(),
                })
            }
            (None, None) => {}
        }
    }
    assert!(open.is_none(), "unclosed code fence");
    blocks
}

/// The marked snippets of this file, in source order, with the wrapper's four-space indent removed.
fn marked_snippets(source: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut current: Option<(String, String)> = None;
    for line in source.lines() {
        let trimmed = line.trim_start();
        if let Some(name) = trimmed.strip_prefix("// @snippet ") {
            assert!(current.is_none(), "snippet {name} opened inside another");
            current = Some((name.trim().to_string(), String::new()));
        } else if trimmed == "// @end" {
            out.push(current.take().expect("// @end without // @snippet"));
        } else if let Some((name, body)) = &mut current {
            let dedented = match line.is_empty() {
                true => line,
                false => line
                    .strip_prefix("    ")
                    .unwrap_or_else(|| panic!("snippet {name}: line is not indented: {line:?}")),
            };
            body.push_str(dedented);
            body.push('\n');
        }
    }
    assert!(current.is_none(), "unclosed snippet marker");
    out
}

fn read(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Every `rust` block of README.md is one of the snippets compiled above, in the same order — so a
/// snippet cannot be edited in the README without the compiler seeing the change.
#[test]
fn readme_snippets_match_this_file() {
    let snippets = marked_snippets(include_str!("doc_snippets.rs"));
    let readme: Vec<CodeBlock> = code_blocks(&read("README.md"))
        .into_iter()
        .filter(|b| b.lang == "rust")
        .collect();

    assert_eq!(
        readme.len(),
        snippets.len(),
        "README.md has {} rust blocks, this file marks {} snippets",
        readme.len(),
        snippets.len()
    );
    for (block, (name, body)) in readme.iter().zip(&snippets) {
        assert_eq!(
            &block.body, body,
            "README.md block does not match the compiled snippet `{name}`"
        );
    }
}

/// The Russian README is a translation of the prose only: its code blocks are the English ones,
/// byte for byte, in the same order.
#[test]
fn russian_readme_carries_the_same_code() {
    let english = code_blocks(&read("README.md"));
    let russian = code_blocks(&read("README.ru.md"));

    assert_eq!(
        english.len(),
        russian.len(),
        "README.md has {} code blocks, README.ru.md has {}",
        english.len(),
        russian.len()
    );
    for (i, (en, ru)) in english.iter().zip(&russian).enumerate() {
        assert_eq!(
            en,
            ru,
            "code block #{} differs between the two READMEs",
            i + 1
        );
    }
}
