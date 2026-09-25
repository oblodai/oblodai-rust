//! Every Rust block of README.md, compiled AND run.
//!
//! Each function below holds one README snippet between `// @snippet <name>` and `// @end`,
//! copied verbatim apart from the wrapper it needs (a signature, and stubs for the caller's own
//! helpers). [`readme_blocks_are_the_snippets`] asserts that the fenced `rust` blocks of README.md
//! are, in order, byte-identical to the marked snippets once the wrapper's indent is removed, and
//! that README.ru.md carries the same code byte for byte. [`the_snippets_run`] then runs every
//! snippet against a fake gateway on a real socket, `Client::from_env()` included.

#![cfg(feature = "blocking")]
#![allow(dead_code, unused_variables, clippy::result_large_err)]

mod support;

use std::time::Duration;

use oblodai::core::signing::HEADER_SIGNATURE;
use oblodai::models::{
    BalanceResult, BatchInfoResponse, BatchSubmitResponse, DocumentJobAccepted, DocumentJobView,
    FaucetResult, PaymentWebhook, PayoutItem, ResetResult, SimulateDepositResult,
    TestWebhookKindResult,
};
use oblodai::webhooks::{HEADER_WEBHOOK_SIGNATURE, HEADER_WEBHOOK_TIMESTAMP};
use oblodai::{Client, WebhookEvent};
use serde_json::{json, Value};
use support::{envelope, model, FakeGateway, Received};

fn mark_order_paid(_order_id: &str) {}
fn schedule_retry(_seconds: u64) {}

// --- the snippets -------------------------------------------------------------------------------

fn readme_blocking() -> oblodai::Result<()> {
    // @snippet readme_blocking
    use oblodai::models::{HistoryRequest, PaymentRequest};

    let client = oblodai::blocking::Client::from_env()?;
    let invoice = client
        .payments()
        .create(PaymentRequest::new("25", "USDT"))
        .send()?;
    for payout in client
        .payouts()
        .list_history(HistoryRequest::default())
        .iter()
    {
        println!("{}", payout?.uuid);
    }
    // @end
    Ok(())
}

fn readme_one_key(public_id: &str, secret: &str) -> oblodai::Result<Client> {
    // @snippet readme_one_key
    let client = oblodai::Client::builder()
        .public_id(public_id)
        .secret(secret)
        .build()?;
    // @end
    Ok(client)
}

async fn readme_quickstart() -> oblodai::Result<()> {
    // @snippet readme_quickstart
    use oblodai::models::PaymentRequest;
    use oblodai::Client;

    let client = Client::from_env()?;
    let invoice = client
        .payments()
        .create(PaymentRequest {
            network: Some("tron".into()), // omit to let the payer choose on the pay page
            order_id: Some("order-1001".into()), // your reference; idempotent per order_id
            url_callback: Some("https://shop.example/oblodai/webhook".into()),
            ..PaymentRequest::new("25", "USDT") // amount (a decimal string) and currency
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
    use oblodai::models::PayoutRequest;

    let payout = client
        .payouts()
        .create(PayoutRequest {
            network: Some("tron".into()),
            ..PayoutRequest::new(
                "TQrY8bkbpXKPt2LZbU8jqfnpFbUSF15sbx",
                "10",
                "USDT",
                "payout-1001",
            )
        })
        .idempotency_key("payout-1001") // makes the retry safe across restarts too
        .await?;
    println!("{} {}", payout.uuid, payout.status);
    // @end
    Ok(())
}

async fn readme_call_options(client: &Client) -> oblodai::Result<()> {
    // @snippet readme_call_options
    use std::time::Duration;

    let balance = client
        .account()
        .get_balance()
        .timeout(Duration::from_secs(10)) // one attempt
        .deadline(Duration::from_secs(45)) // the whole call, retries and pauses included
        .max_retries(5)
        .extra_header("X-Tenant", "eu") // this call only
        .request_id("order-1001-balance") // X-Request-ID; a fresh UUID when not set
        .await?;
    // @end
    Ok(())
}

async fn readme_lists(client: &Client) -> oblodai::Result<()> {
    // @snippet readme_lists
    use futures_util::StreamExt;
    use oblodai::enums::PayoutKind;
    use oblodai::models::HistoryRequest;

    // one page
    let page = client
        .payments()
        .list_history(HistoryRequest::default())
        .limit(50)
        .await?;
    println!("{} of {}", page.items.len(), page.paginate.total);

    // every item, one page fetched at a time
    let mut payouts = client
        .payouts()
        .list_history(HistoryRequest::default())
        .stream();
    while let Some(payout) = payouts.next().await {
        println!("{}", payout?.uuid);
    }

    // page by page
    let mut pages = client
        .payments()
        .list_history(HistoryRequest::default())
        .by_page();
    while let Some(page) = pages.next().await {
        println!("a page of {}", page?.items.len());
    }

    // or collect, with a cap
    let refunds = client
        .payouts()
        .list_history(HistoryRequest {
            kind: Some(PayoutKind::Refund),
            ..Default::default()
        })
        .all(Some(1000))
        .await?;
    // @end
    Ok(())
}

async fn readme_jobs(client: &Client) -> oblodai::Result<()> {
    // @snippet readme_jobs
    use oblodai::models::{DocumentJobRequest, PayoutBatchRequest};
    use oblodai::JobStatus;

    // a batch: `job()` sends the create call, `wait()` polls batches().get_info() until it ends
    let job = client
        .batches()
        .create_payout(PayoutBatchRequest::default())
        .job()
        .await?;
    let batch = job.wait().await?; // status `completed` or `stopped`
    println!(
        "{}: {} ok, {} failed",
        job.id(),
        batch.succeeded,
        batch.failed
    );

    // a document export: wait, then download the file
    let job = client
        .documents()
        .create_job(DocumentJobRequest::new("statement"))
        .job()
        .await?;
    if job.wait().await?.status() == "done" {
        let file = job.download().await?;
        println!("{} bytes of {}", file.bytes.len(), file.content_type);
    }
    // @end
    Ok(())
}

async fn readme_raw_and_hooks(client: &Client) -> oblodai::Result<()> {
    // @snippet readme_raw_and_hooks
    use oblodai::{ClientOptions, Hooks};
    use std::time::Duration;

    // status, headers and request id of a successful call; `parse()` gives the usual value
    let raw = client.account().get_balance().with_raw_response().await?;
    println!("{} {}", raw.status(), raw.request_id());
    let balance = raw.parse()?;

    // a copy of the client with other settings; the original is untouched
    let patient = client.with_options(ClientOptions::new().timeout(Duration::from_secs(60)));

    // hooks see every attempt (the signature is redacted)
    let traced = client.with_options(
        ClientOptions::new().hooks(
            Hooks::new()
                .on_request(|r| println!("-> {} {} #{}", r.method, r.url, r.attempt))
                .on_response(|r| println!("<- {} in {:?}", r.status, r.elapsed)),
        ),
    );
    traced.account().get_balance().await?;
    // @end
    Ok(())
}

async fn readme_sandbox(client: &Client, invoice_uuid: String) -> oblodai::Result<()> {
    // @snippet readme_sandbox
    use oblodai::generated::resources::SandboxListWebhooksQuery;
    use oblodai::models::{FaucetRequest, SimulateDepositRequest, TestWebhookKindRequest};

    // test money to pay out from (`test_` keys only)
    client
        .sandbox()
        .faucet(FaucetRequest::new("1000", "USDT"))
        .await?;

    // "pay" an invoice; repeat the same txid with more confirmations to walk pending → paid
    client
        .sandbox()
        .simulate_deposit(SimulateDepositRequest::new(invoice_uuid))
        .await?;

    // a rehearsal delivery: signed exactly like a live one, and marked `test: true`
    client
        .webhooks()
        .send_test_payment(TestWebhookKindRequest::new(
            "https://shop.example/oblodai/webhook",
        ))
        .await?;

    // what was delivered, with payloads — then a clean slate
    let deliveries = client
        .sandbox()
        .list_webhooks(SandboxListWebhooksQuery::default())
        .all(None)
        .await?;
    client.sandbox().reset().await?;
    // @end
    Ok(())
}

fn readme_webhooks(
    raw_body: &[u8],
    request_headers: Vec<(String, String)>,
    secret: &str,
) -> oblodai::Result<()> {
    // @snippet readme_webhooks
    use oblodai::enums::PaymentStatus;
    use oblodai::webhooks::{verify_webhook_delivery, Headers, VerifyOptions};
    use oblodai::WebhookEvent;

    let headers = Headers::from_pairs(request_headers); // any (name, value) pairs
    let delivery = verify_webhook_delivery(raw_body, &headers, &VerifyOptions::new(secret))?;

    if delivery.is_test {
        return Ok(()); // a rehearsal: signed like a live one, but nothing moved
    }

    match &delivery.event {
        WebhookEvent::Payment(p) if p.status == PaymentStatus::Paid => mark_order_paid(&p.order_id),
        _ => {}
    }
    // @end
    Ok(())
}

async fn readme_errors(
    client: &Client,
    params: oblodai::models::PayoutRequest,
) -> oblodai::Result<oblodai::models::PayoutItem> {
    // @snippet readme_errors
    match client.payouts().create(params).await {
        Ok(payout) => Ok(payout),
        Err(err) => {
            // `[payout.insufficient_funds] … (request_id=…)`
            eprintln!("{err}");
            match err.code() {
                // retryable — the balance may still arrive
                "payout.insufficient_funds" | "payout.funds_maturing" => {
                    schedule_retry(err.retry_after().unwrap_or(60));
                    Err(err)
                }
                _ => Err(err), // the SDK already retried what was safe to retry
            }
        }
    }
    // @end
}

// --- running them -------------------------------------------------------------------------------

fn page(items: Vec<Value>) -> Value {
    json!({"items": items, "paginate": {"total": items.len(), "per_page": 50, "offset": 0, "has_pages": false}})
}

/// What the fake gateway answers, by route: the smallest valid model of each.
fn answer(req: &Received) -> (u16, Vec<(String, String)>, Vec<u8>) {
    let payment = support::sample("payment");
    let payout = model::<PayoutItem>(json!({"uuid": "po-1", "status": "pending"}));
    let result = match req.path() {
        "/v1/payment" => payment,
        "/v1/payment/history" => page(vec![payment]),
        "/v1/payout" if req.body.windows(9).any(|w| w == b"999999999") => {
            let body = json!({"error": {"code": "payout.insufficient_funds",
                "message": "not enough USDT", "retryable": false, "request_id": "gw-1"}});
            return (409, vec![], body.to_string().into_bytes());
        }
        "/v1/payout" => payout,
        "/v1/payout/history" => page(vec![support::sample("payout")]),
        "/v1/balance" => model::<BalanceResult>(json!({})),
        "/v1/sandbox/faucet" => model::<FaucetResult>(json!({})),
        "/v1/sandbox/deposit" => model::<SimulateDepositResult>(json!({})),
        "/v1/test-webhook/payment" => model::<TestWebhookKindResult>(json!({"signed": true})),
        "/v1/sandbox/webhooks" => page(vec![]),
        "/v1/sandbox/reset" => model::<ResetResult>(json!({})),
        "/v1/payout/batch" => model::<BatchSubmitResponse>(json!({"batch_id": "b-1"})),
        "/v1/batch/info" => {
            model::<BatchInfoResponse>(json!({"batch_id": "b-1", "status": "completed"}))
        }
        "/v1/documents/jobs" => model::<DocumentJobAccepted>(json!({"job_id": "j-1"})),
        "/v1/documents/jobs/info" => {
            model::<DocumentJobView>(json!({"job_id": "j-1", "status": "done"}))
        }
        "/v1/documents/jobs/file" => {
            return (
                200,
                vec![("Content-Type".into(), "application/pdf".into())],
                b"%PDF".to_vec(),
            )
        }
        other => panic!("the README called {other}, which the fake gateway does not serve"),
    };
    envelope(result)
}

fn gateway_env(gateway: &FakeGateway) {
    // One test owns the process environment: `from_env()` is what the README shows.
    std::env::set_var("OBLODAI_BASE_URL", &gateway.base_url);
    std::env::set_var("OBLODAI_PUBLIC_ID", "test_oblodai_readme");
    std::env::set_var("OBLODAI_SECRET", "oblodai_test_readme");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_snippets_run() {
    let gateway = FakeGateway::start(answer);
    gateway_env(&gateway);

    readme_quickstart().await.unwrap();
    let client = readme_one_key("test_oblodai_readme", "oblodai_test_readme").unwrap();
    // The builder reads OBLODAI_BASE_URL too, so this client talks to the fake gateway.
    readme_payout(&client).await.unwrap();
    readme_call_options(&client).await.unwrap();
    readme_sandbox(&client, "x".into()).await.unwrap();
    readme_lists(&client).await.unwrap();
    readme_raw_and_hooks(&client).await.unwrap();
    readme_jobs(&client).await.unwrap();

    let refused = readme_errors(
        &client,
        oblodai::models::PayoutRequest::new("T", "999999999", "USDT", "o-big"),
    )
    .await
    .unwrap_err();
    assert_eq!(
        refused.to_string(),
        "[payout.insufficient_funds] not enough USDT (request_id=gw-1)"
    );

    tokio::task::spawn_blocking(readme_blocking)
        .await
        .unwrap()
        .unwrap();

    let paths: Vec<String> = gateway
        .received()
        .iter()
        .map(|r| r.path().to_string())
        .collect();
    for want in [
        "/v1/payment",
        "/v1/payout",
        "/v1/batch/info",
        "/v1/documents/jobs/file",
    ] {
        assert!(
            paths.iter().any(|p| p == want),
            "the README never reached {want}: {paths:?}"
        );
    }
    let signed = gateway
        .received()
        .iter()
        .all(|r| r.header(HEADER_SIGNATURE).is_some());
    assert!(
        signed,
        "every README call is signed with the key from the environment"
    );
}

#[test]
fn the_webhook_snippet_accepts_a_signed_delivery_and_refuses_a_forged_one() {
    let secret = "whsec_readme";
    let body = serde_json::to_vec(&model::<PaymentWebhook>(
        json!({"type": "payment", "status": "paid", "order_id": "order-1001"}),
    ))
    .unwrap();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let headers = |sig: String| {
        vec![
            (HEADER_WEBHOOK_TIMESTAMP.to_string(), ts.to_string()),
            (HEADER_WEBHOOK_SIGNATURE.to_string(), sig),
        ]
    };
    readme_webhooks(
        &body,
        headers(oblodai::sign_webhook(secret, ts, &body)),
        secret,
    )
    .unwrap();
    let err = readme_webhooks(
        &body,
        headers(oblodai::sign_webhook("x", ts, &body)),
        secret,
    )
    .unwrap_err();
    assert_eq!(err.code(), "webhook.bad_signature");
    let _ = WebhookEvent::Other(Value::Null);
    let _ = Duration::ZERO;
}

// --- README blocks are the snippets ---------------------------------------------------------------

fn read(name: &str) -> String {
    std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(name))
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn rust_blocks(markdown: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current: Option<String> = None;
    for line in markdown.lines() {
        match &mut current {
            None if line.trim_end() == "```rust" => current = Some(String::new()),
            Some(block) if line.trim_end() == "```" => {
                out.push(std::mem::take(block));
                current = None;
            }
            Some(block) => {
                block.push_str(line);
                block.push('\n');
            }
            None => {}
        }
    }
    out
}

fn marked_snippets(source: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut current: Option<(String, String)> = None;
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix("// @snippet ") {
            current = Some((name.to_string(), String::new()));
        } else if trimmed == "// @end" {
            out.push(current.take().expect("@end without @snippet"));
        } else if let Some((_, body)) = &mut current {
            body.push_str(line.strip_prefix("    ").unwrap_or(line));
            body.push('\n');
        }
    }
    out
}

#[test]
fn readme_blocks_are_the_snippets() {
    let snippets = marked_snippets(&read("tests/readme.rs"));
    let blocks = rust_blocks(&read("README.md"));
    assert_eq!(
        blocks.len(),
        snippets.len(),
        "README.md has {} rust blocks, tests/readme.rs {} snippets",
        blocks.len(),
        snippets.len()
    );
    for ((name, snippet), block) in snippets.iter().zip(&blocks) {
        assert_eq!(block, snippet, "README block for snippet {name} differs");
    }
    assert_eq!(
        rust_blocks(&read("README.ru.md")),
        blocks,
        "README.ru.md must carry the same code blocks, byte for byte"
    );
}
