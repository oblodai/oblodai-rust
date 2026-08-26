<div align="center">

# Oblodai Rust SDK

Official Rust client for the [Oblodai](https://oblodai.com) crypto payment gateway: invoices,
payouts, refunds, payout links, static wallets, webhooks and documents — the whole merchant API,
typed end to end and verified against the gateway's own contract snapshot.

[![crates.io](https://img.shields.io/crates/v/oblodai.svg)](https://crates.io/crates/oblodai)
[![docs.rs](https://img.shields.io/docsrs/oblodai)](https://docs.rs/oblodai)
[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

</div>

- Rust 2021, MSRV 1.86. Async on `reqwest` + `tokio` (rustls, no OpenSSL); a `blocking` client behind a feature.
  (The SDK's own code builds on 1.75; the floor comes from the dependency tree — `reqwest` → `url` → `idna`/`icu`.)
- Every route the gateway exposes has a method here; request bodies, enums and error codes are generated from the gateway.
- Retries driven by the API's own `retryable` flag, automatic idempotency keys, clock-skew correction.
- `oblodai::webhooks`: signature verification that needs no client and no API key.

```toml
[dependencies]
oblodai = "1.3"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Start in the sandbox

Get your keys in the Oblodai dashboard. A **sandbox key** (`test_…`) drives a chainless copy of the
gateway — fake balance from a faucet, simulated deposits, real webhooks — so integrate against it first.

```rust
use oblodai::{Client, contract::requests::PaymentRequest};

#[tokio::main]
async fn main() -> oblodai::Result<()> {
    let client = Client::from_env()?; // OBLODAI_PUBLIC_ID / OBLODAI_SECRET

    let invoice = client
        .payments()
        .create(PaymentRequest {
            amount: "25".into(),           // amounts are decimal strings, never floats
            currency: "USDT".into(),       // what you price in — a fiat (USD, EUR, …) or an asset
            network: Some("tron".into()),  // omit to let the payer choose on the pay page
            order_id: Some("order-1001".into()), // your reference; idempotent per order_id
            url_callback: Some("https://shop.example/oblodai/webhook".into()),
            ..Default::default()
        })
        .await?;

    println!("{} {} {}", invoice.url, invoice.address, invoice.status);
    Ok(())
}
```

Prices in fiat: `amount: "25".into(), currency: "USD".into(), to_currency: Some("USDT".into())` —
`currency` is what you charge, `to_currency` the asset the payer sends. See [`examples/`](examples).

### Two keys

The gateway issues a **payment key** (`pk_…`) and a **payout key** (`wk_…`). Sandbox keys are both at
once; live keys are separate, and money-out routes need the payout one: `payouts`, `refunds`,
`payout_links`, `transfers`, `splits`, `wallets().refund_blocked_deposit`, auto-withdraw, the IP
allow-list, `webhooks().rotate_secret`, `sandbox().faucet`/`reset`. Pass both pairs and the SDK picks
the right one per call:

```rust
let client = oblodai::Client::builder()
    .public_id(public_id).secret(secret)
    .payout_public_id(payout_public_id).payout_secret(payout_secret)
    .build()?;
// or OBLODAI_PUBLIC_ID / OBLODAI_SECRET / OBLODAI_PAYOUT_PUBLIC_ID / OBLODAI_PAYOUT_SECRET
```

A call with the wrong kind is a 403 `merchant.wrong_key_kind`.

## Resources

| Namespace                       | Methods                                                                                                                                                                                     |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `payments()`                    | create · info/get · cancel · history/list · batch · qr · services · send_email · resend · public_view · select · public_qr                                                                   |
| `refunds()`                     | create · resolve · batch                                                                                                                                                                    |
| `payouts()`                     | create · validate · calculate · info/get · cancel · approve · history/list · mass · batch · services · get/set_fee_config · get/set_refund_fee_config                                        |
| `payout_links()`                | create · info/get · list · cancel · batch · cheque · claim_preview · claim                                                                                                                   |
| `payment_links()`               | create · info/get · list · toggle · public_view · checkout                                                                                                                                   |
| `batches()` / `transfers()`     | info · to_personal · to_user · batch                                                                                                                                                        |
| `wallets()`                     | create · qr · block · refund_blocked_deposit                                                                                                                                                |
| `webhooks()`                    | register · rotate_secret · deliveries · test                                                                                                                                                |
| `documents()`                   | statement · ledger · balance_certificate · fee_schedule · split_report · batch_report · link_report · wallet_statement · referrals_report · create_job · job_info · job_file · download      |
| `splits()`                      | create_rule · list_rules · delete_rule · get/set_config · get/set_opt_in                                                                                                                    |
| `settings()`                    | set_discount · list_discounts · get/set_accuracy · get/set_auto_refund · list_accepted · set_accepted · get/set_payment_fee_config · list/set/delete_auto_withdraw · list/add/remove/enable_api_allowlist |
| `account()` / `catalog()`       | balance · referral · vrcs/set_vrcs · currencies · exchange_rates                                                                                                                            |
| `sandbox()`                     | faucet · deposit · webhooks · replay · reset                                                                                                                                                |
| `merchants()`                   | create · create_sandbox (provisioning; `admin_token` on a self-hosted gateway)                                                                                                              |

Every method returns a builder that is also a future: `.await` it, or set per-call options first —
`.idempotency_key(…)`, `.timeout(…)`, `.deadline(…)`, `.prefer_payout_key(true)`. Lookups accept a
bare uuid or a `Lookup`: `payments().info("uuid")`, `payments().info(Lookup::order_id("order-1001"))`.

### Lists

List methods return a `Pager`. Nothing is requested until you consume it.

```rust
use futures_util::StreamExt;

// one page
let page = client.payments().history(Default::default()).limit(50).await?;
println!("{} of {}", page.items.len(), page.paginate.total);

// every item, one page fetched at a time
let mut payouts = client.payouts().history(Default::default()).stream();
while let Some(payout) = payouts.next().await {
    println!("{}", payout?.uuid);
}

// or collect, with a cap
let refunds = client
    .payouts()
    .history(PayoutHistoryRequest { kind: Some(PayoutKind::Refund), ..Default::default() })
    .all(Some(1000))
    .await?;
```

### Statuses

- Payment: `select → created → confirm_check → paid | paid_over | wrong_amount | expired | cancelled`.
  `is_payment_paid(&status)` is true for `paid`/`paid_over`; `wrong_amount` (underpaid) waits for
  `refunds().resolve(…)`; `is_payment_final` covers the rest.
- Payout: `pending → approved → awaiting_cosign → broadcasting → sent → confirmed | failed | cancelled`.

Prefer webhooks for state changes; poll `info` only as a fallback. Every enum carries an
`Other(String)` variant, so a value a newer gateway introduces still decodes.

### Errors

Every failure is an `oblodai::Error` carrying the API's error envelope: `code()`
(`payout.insufficient_funds`), `http_status()`, `retryable()`, `retry_after()`, `request_id()`,
`field()`, `synthetic()`, and a `kind()` for matching: `Validation` (400), `Authentication` (401),
`Permission` (403), `NotFound` (404), `Conflict` / `IdempotencyConflict` (409), `RateLimit` (429),
`Unavailable` (503), `Internal`, `Transport` (no response), `Config` (rejected before sending),
`Contract`, `Signature`. Quote `request_id()` to support. The raw body is never printed by `Debug`
and never serialized.

```rust
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
```

### Retries and idempotency

- Create-type routes get an `Idempotency-Key` automatically (one per logical call, reused on every
  retry), so a timeout can never produce a second payout. Pass your own with `.idempotency_key(…)` to
  make retries safe across restarts; on routes the gateway does not deduplicate the SDK refuses a key
  (`sdk.idempotency_unsupported`).
- An error is retried only when the API says `retryable: true`. Answers without an API envelope (a
  proxy 502/503) and transport failures are retried only on read routes or keyed writes.
  `Retry-After` is honoured.
- `ClientBuilder::retry(RetryOptions { max_retries, base_delay_ms, max_delay_ms, max_retry_after_ms })`;
  `.timeout(…)` per attempt, `.deadline(…)` per call. Dropping the future cancels the call.

### Webhooks

```rust
use oblodai::webhooks::{verify_webhook_delivery, Headers, VerifyOptions};

let headers = Headers::from_pairs(request_headers);   // any (name, value) pairs
let delivery = verify_webhook_delivery(raw_body, &headers, &VerifyOptions::new(secret))?;

if delivery.is_test {
    return Ok(());   // a rehearsal: signed like a live one, but nothing moved
}

match &delivery.event {
    oblodai::WebhookEvent::Payment(p) if p.status == oblodai::PaymentStatus::Paid => {
        mark_order_paid(p.order_id.as_deref())
    }
    _ => {}
}
```

Verify over the **raw** bytes. Rehearsal deliveries (`webhooks().test()`, sandbox) are signed like
live ones and carry `test: true` (and `X-Webhook-Test: true`) — check `delivery.is_test`
(or `is_test_event(&delivery.event)` / `event.is_test()`) and never act on one as if money moved.
`delivery.id` (`X-Webhook-Id`) is stable across retries — use it to
deduplicate; `event.sequence()` orders events (`is_stale_event`). After `webhooks().rotate_secret()`
pass `.previous_secret(old)` for at least 26 hours. The module needs no client and no API key.

### Money helpers

`add_amounts`, `subtract_amounts`, `compare_amounts`, `amounts_equal`, `is_zero_amount` — exact
decimal arithmetic on the string amounts the API uses. Never parse a `Money` as `f64`.

### Blocking client

```toml
oblodai = { version = "1.3", features = ["blocking"] }
```

```rust
let client = oblodai::blocking::Client::from_env()?;
let invoice = client.payments().create(params).send()?;
for payout in client.payouts().history(Default::default()).iter() {
    println!("{}", payout?.uuid);
}
```

It is the same method tree over the same pure core (signing, envelopes, retry decisions); only the
I/O differs.

### Self-hosted or local gateway

`base_url("http://localhost:8095")` works out of the box; other plain-http hosts need
`.allow_insecure_base_url(true)` (or `OBLODAI_ALLOW_INSECURE=1`). A path prefix in `base_url` is kept.
`OBLODAI_LOG=debug` installs a stderr logger; secrets and signatures are redacted.

## The contract snapshot

`contract/` is exported by the gateway's own test suite: the route registry, request DTO schemas with
English field docs, enums, every error code, signing vectors, golden response bodies recorded from a
live gateway and real signed webhook deliveries. `src/contract/{routes,enums,requests,version}.rs`
are generated from it.

```sh
python3 scripts/codegen.py            # regenerate after refreshing contract/
python3 scripts/codegen.py --check    # CI gate: fail when the two disagree
```

## Development

```sh
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features             # unit + contract tests, no network
python3 scripts/codegen.py --check

# against a real gateway
OBLODAI_LIVE_URL=http://127.0.0.1:8095 cargo test --all-features -- --ignored --test-threads=1
```

License: MIT.
