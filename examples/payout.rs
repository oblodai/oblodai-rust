//! Send money out: check the price, dry-run the payout, then create it.
//!
//! ```sh
//! OBLODAI_PUBLIC_ID=… OBLODAI_SECRET=… cargo run --example payout
//! ```
//!
//! Payouts are signed with the same API key as everything else; a `test_` key runs this in the
//! sandbox.

use std::time::{SystemTime, UNIX_EPOCH};

use oblodai::contract::requests::{PayoutCalculateRequest, PayoutRequest, PayoutValidateRequest};
use oblodai::{Client, ErrorKind};

const ADDRESS: &str = "TQrY8bkbpXKPt2LZbU8jqfnpFbUSF15sbx";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;

    let balance = client.account().balance().await?;
    for entry in &balance.balance.merchant {
        println!("balance   {} {}", entry.balance, entry.currency);
    }

    // What it will cost, without creating anything.
    let quote = client
        .payouts()
        .calculate(PayoutCalculateRequest {
            amount: "10".into(),
            currency: "USDT".into(),
            network: Some("tron".into()),
            ..Default::default()
        })
        .await?;
    println!(
        "quote     {} + {} fee, paid by {}",
        quote.amount.clone().unwrap_or_default(),
        quote.commission.clone().unwrap_or_default(),
        quote.fee_bearer
    );

    // Every check the real call makes — address format, memo, funds, maturity — and nothing moves.
    let dry_run = client
        .payouts()
        .validate(PayoutValidateRequest {
            amount: "10".into(),
            currency: "USDT".into(),
            network: Some("tron".into()),
            address: ADDRESS.into(),
            ..Default::default()
        })
        .await?;
    if !dry_run.valid {
        println!("the dry run refused it: {}", dry_run.maturity_note);
        return Ok(());
    }

    let order_id = format!(
        "payout-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );

    // The SDK attaches an Idempotency-Key automatically and reuses it on every retry, so a timeout
    // can never turn into a second payout. Pass your own to make that hold across restarts too.
    let result = client
        .payouts()
        .create(PayoutRequest {
            amount: "10".into(),
            currency: "USDT".into(),
            network: Some("tron".into()),
            address: ADDRESS.into(),
            order_id: order_id.clone(),
            ..Default::default()
        })
        .idempotency_key(&order_id)
        .await;

    match result {
        Ok(payout) => {
            println!("payout    {} → {}", payout.uuid, payout.status);
            println!("debited   {} {}", payout.payer_amount, payout.currency);
        }
        Err(err) if err.retryable() => {
            // The gateway's own flag: the balance may still arrive. Retry later, not now — the SDK
            // already retried what was safe to retry inside this call.
            println!(
                "not now ({}): try again in {} s",
                err.code(),
                err.retry_after().unwrap_or(60)
            );
        }
        Err(err) if err.kind() == ErrorKind::Validation => {
            println!("rejected: {} (field {:?})", err.message(), err.field());
        }
        Err(err) => return Err(err.into()),
    }

    Ok(())
}
