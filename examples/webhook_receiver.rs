//! A webhook receiver, in plain std — no web framework, so the verification is the only thing on
//! show. Put your own router in front of it; what matters is the three rules below.
//!
//! ```sh
//! OBLODAI_WEBHOOK_SECRET=whsec_… cargo run --example webhook_receiver
//! # then point the gateway at http://<host>:8099/oblodai/webhook
//! ```
//!
//! 1. Verify over the **raw** request bytes. A re-serialized parse will not match the signature.
//! 2. Deduplicate on `X-Webhook-Id`: retries of one delivery carry the same id.
//! 3. Drop out-of-order events with `is_stale_event` — a retried `paid` can arrive after a refund.
//! 4. Never act on a rehearsal (`delivery.is_test`) as if money moved: it is signed like a live one.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

use oblodai::webhooks::{is_stale_event, verify_webhook_delivery, Headers, VerifyOptions};
use oblodai::WebhookEvent;

fn main() -> std::io::Result<()> {
    let secret = std::env::var("OBLODAI_WEBHOOK_SECRET")
        .expect("set OBLODAI_WEBHOOK_SECRET to the endpoint secret from webhooks().register()");
    // Rotating? Keep the outgoing secret here for ~26 h: deliveries queued before the rotation stay
    // signed with it for their whole retry life.
    let previous = std::env::var("OBLODAI_WEBHOOK_SECRET_PREV").ok();

    let listener = TcpListener::bind("0.0.0.0:8099")?;
    println!("listening on http://0.0.0.0:8099/oblodai/webhook");

    let mut seen: HashSet<String> = HashSet::new();
    let mut last_sequence: HashMap<String, i64> = HashMap::new();

    for stream in listener.incoming() {
        let mut stream = stream?;
        let Some((headers, body)) = read_request(&mut stream) else {
            respond(&mut stream, 400, "bad request");
            continue;
        };

        let mut options = VerifyOptions::new(&secret);
        if let Some(prev) = &previous {
            options = options.previous_secret(prev);
        }

        let delivery = match verify_webhook_delivery(&body, &headers, &options) {
            Ok(delivery) => delivery,
            Err(err) => {
                // Never act on an unverified body. 400 tells the gateway not to keep retrying a
                // delivery this endpoint will never accept.
                eprintln!("rejected: {} ({})", err.message(), err.code());
                respond(&mut stream, 400, "bad signature");
                continue;
            }
        };

        // Answer fast: the gateway retries on a timeout, and a duplicate is cheaper than a stall.
        respond(&mut stream, 200, "ok");

        // A rehearsal from `webhooks().test()` or the sandbox: the signature is genuine, the money
        // is not. Log it, prove the endpoint works, and stop before anything is credited.
        if delivery.is_test {
            println!(
                "rehearsal {} {} — endpoint verified, nothing credited",
                delivery.event.event_kind(),
                delivery.event.uuid()
            );
            continue;
        }

        if let Some(id) = &delivery.id {
            if !seen.insert(id.clone()) {
                println!("duplicate delivery {id} — already handled");
                continue;
            }
        }

        let event = &delivery.event;
        let key = format!("{}:{}", event.event_kind(), event.uuid());
        if is_stale_event(event, last_sequence.get(&key).copied()) {
            println!("stale {key} (sequence {:?}) — dropped", event.sequence());
            continue;
        }
        if let Some(sequence) = event.sequence() {
            last_sequence.insert(key, sequence);
        }

        match event {
            WebhookEvent::Payment(payment) => {
                println!(
                    "invoice {} is {} ({} {} received)",
                    payment.uuid, payment.status, payment.payment_amount, payment.payer_currency
                );
                // mark_order_paid(payment.order_id.as_deref()) …
            }
            WebhookEvent::Payout(payout) => {
                println!(
                    "payout {} is {} (txid {})",
                    payout.uuid, payout.status, payout.txid
                );
            }
            WebhookEvent::Wallet(wallet) => {
                println!(
                    "wallet {} received {} {}",
                    wallet.address, wallet.payment_amount, wallet.payer_currency
                );
            }
            // `WebhookEvent` is `#[non_exhaustive]`: an event type newer than this SDK arrives as
            // `Other` with its body intact instead of failing verification. Acknowledge it.
            other => {
                println!(
                    "unknown event type {:?} for {} — acknowledged, not acted on",
                    other.event_kind(),
                    other.uuid()
                );
            }
        }
    }
    Ok(())
}

/// Read one HTTP request and hand back its headers and its body bytes, unmodified.
fn read_request(stream: &mut TcpStream) -> Option<(Headers, Vec<u8>)> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?; // the request line

    let mut pairs = Vec::new();
    let mut length = 0usize;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).ok()? == 0 {
            return None;
        }
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        let (name, value) = header.split_once(':')?;
        let (name, value) = (name.trim(), value.trim());
        if name.eq_ignore_ascii_case("content-length") {
            length = value.parse().ok()?;
        }
        pairs.push((name.to_string(), value.to_string()));
    }

    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).ok()?;
    Some((Headers::from_pairs(pairs), body))
}

fn respond(stream: &mut TcpStream, status: u16, body: &str) {
    let reason = if status == 200 { "OK" } else { "Bad Request" };
    let _ = write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.flush();
}
