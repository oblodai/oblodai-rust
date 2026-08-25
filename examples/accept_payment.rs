//! Accept a payment: create an invoice, then watch it settle.
//!
//! ```sh
//! OBLODAI_PUBLIC_ID=… OBLODAI_SECRET=… cargo run --example accept_payment
//! # against a local gateway:
//! OBLODAI_BASE_URL=http://127.0.0.1:8095 OBLODAI_PUBLIC_ID=… OBLODAI_SECRET=… \
//!   cargo run --example accept_payment
//! ```

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use oblodai::contract::requests::PaymentRequest;
use oblodai::helpers::{is_payment_final, is_payment_paid};
use oblodai::{Client, PaymentStatus};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;

    // `order_id` is your own reference. The gateway is idempotent per order_id, so re-running this
    // with the same one returns the same invoice instead of creating a second.
    let order_id = format!(
        "order-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );

    let invoice = client
        .payments()
        .create(PaymentRequest {
            amount: "25".into(),          // decimal string, never a float
            currency: "USDT".into(),      // what you price in; use "USD" + to_currency for fiat
            network: Some("tron".into()), // omit to let the payer pick on the pay page
            order_id: Some(order_id.clone()),
            payer_email: Some("buyer@example.com".into()),
            url_callback: Some("https://shop.example/oblodai/webhook".into()),
            ..Default::default()
        })
        .await?;

    println!("invoice   {}", invoice.uuid);
    println!("pay page  {}", invoice.url);
    println!("address   {} ({})", invoice.address, invoice.network);
    println!(
        "due       {} {}",
        invoice.payer_amount, invoice.payer_currency
    );
    if !invoice.destination_tag.is_empty() {
        println!("tag/memo  {}", invoice.destination_tag);
    }

    // In production you would wait for the `invoice.paid` webhook instead of polling. This loop is
    // here so the example finishes on its own.
    for _ in 0..5 {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let current = client.payments().info(&invoice.uuid).await?;
        println!("status    {}", current.status);
        if is_payment_paid(&current.status) {
            println!(
                "paid: {} of {} received",
                current.amount_paid, current.amount
            );
            return Ok(());
        }
        if current.status == PaymentStatus::WrongAmount {
            println!("underpaid — settle it with refunds().resolve()");
            return Ok(());
        }
        if is_payment_final(&current.status) {
            println!("finished as {}", current.status);
            return Ok(());
        }
    }

    println!("still waiting; the webhook will tell you when it settles");
    Ok(())
}
