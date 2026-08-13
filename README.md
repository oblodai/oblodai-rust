# Oblodai Rust SDK

> [Читать по-русски →](README.ru.md)

The official Rust SDK for the **Oblodai** payment gateway: accepting payments, payouts, bulk
operations (batches), payment and payout links, split payments, e-mail invoices, static wallets,
webhooks, and a developer sandbox.
Automatic request signing, parsing of responses into typed structures, error handling, and
automatic retries with duplicate protection (`Idempotency-Key`).

> **The client is BLOCKING.** The transport is `reqwest::blocking`; pauses between retries use
> `std::thread::sleep`. Every call blocks the thread it is made from. **Async applications
> (tokio, async-std) must not call the SDK directly from a task** — a blocked executor thread
> stalls every other task on it. Move the call onto a blocking pool: see
> [Async applications](#async-applications-tokio).

> **The base URL is HTTPS-only.** The default is `https://api.oblodai.com`; override it via
> `Config::base_url(...)`. `http://` on an external host is rejected with an error (the SDK sends
> `X-Public-Id` and the `X-Signature` signature — any middleman would see them in plaintext). The
> exception is loopback (`localhost`, `127.0.0.1`, `::1`) for local setups.

## Installation

```toml
[dependencies]
oblodai = "1"
serde_json = "1"
```

Requires Rust 1.75+.

## Where to get keys

Keys are issued in the Oblodai dashboard — [oblodai.com](https://oblodai.com), API keys section. A
pair consists of two values:

| Value | What it is | Environment variable |
| --- | --- | --- |
| `public_id` | Non-secret key identifier, sent in the `X-Public-Id` header | `OBLODAI_PUBLIC_ID` |
| Secret | Signs outgoing requests (`X-Signature`), never sent in the request itself | `OBLODAI_SECRET` |

Important:

- **The secret is shown only once — at key creation.** Save it right away; if you lose it, the key
  cannot be "looked up" — you issue a new one instead.
- **The sandbox gets its own separate test key:** a `public_id` of the form `test_…`, a secret of
  the form `oblodai_test_…`. Live keys have no such prefixes. To check which key you are holding,
  use the helper `oblodai::is_test_key(public_id)`.
- The API key secret is **not** the webhook secret. The latter is separate and is returned when
  you register an endpoint; see [Verifying webhooks](#verifying-webhooks).
- A secret does not belong in a repository: keep it in environment variables or a secrets manager.

## Credentials

Keep the keys in environment variables (see `.env.example`):

```bash
export OBLODAI_PUBLIC_ID=test_...
export OBLODAI_SECRET=oblodai_test_...
# optional: export OBLODAI_BASE_URL=https://api.oblodai.com
```

```rust
// reads OBLODAI_PUBLIC_ID / OBLODAI_SECRET / OBLODAI_BASE_URL
let client = oblodai::Client::from_env()?;
```

## Quick start

```rust
use oblodai::{Client, Config};
use serde_json::json;

fn main() -> oblodai::Result<()> {
    // or explicitly (equivalent to Client::from_env above):
    let client = Client::new(
        Config::new("test_...", "oblodai_test_...")
            .base_url("https://api.oblodai.com"),
    )?;

    let payment = client.payments().create(json!({
        "amount": "10",
        "currency": "USD",
        "order_id": "order-1",
        "to_currency": "USDT",
        "network": "tron",
    }))?;

    println!("{}", payment.address); // address to pay to
    println!("{}", payment.url);     // hosted payment page (empty if the gateway has no
                                     // public base URL configured — see "Notes")
    Ok(())
}
```

**The same code works with a live key — only the key changes.** A test key is used here
(`test_…` / `oblodai_test_…`) to start in the sandbox; the business methods, paths, and models are
exactly the same with a live key.

### Async applications (tokio)

The client is blocking. In an async runtime, move calls onto a blocking pool, otherwise the
request (up to 30 seconds by default, plus retry delays) will stall an executor thread dead:

```rust
use oblodai::{Client, Config};
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The client is Send + Sync — create it once and reuse it.
    let client = Arc::new(Client::from_env()?);

    let c = Arc::clone(&client);
    let payment = tokio::task::spawn_blocking(move || {
        c.payments().create(json!({
            "amount": "10", "currency": "USD", "order_id": "order-1",
            "to_currency": "USDT", "network": "tron",
        }))
    })
    .await??; // first `?` — task panic/cancellation, second — SDK error

    println!("{}", payment.address);
    Ok(())
}
```

There is no asynchronous (`async fn`) variant of the client in the SDK — that is a deliberate
boundary of the package.

### Timeout

The timeout of a SINGLE HTTP attempt is configured via `Config::timeout` (30 seconds by default):

```rust
use std::time::Duration;

let client = Client::new(
    Config::new("test_...", "oblodai_test_...")
        .timeout(Duration::from_secs(10)),
)?;
```

This is the per-attempt timeout, not the whole call: with retries enabled, the upper bound on
waiting is `timeout × max_attempts` plus backoff delays. The value is applied by the built-in
reqwest transport; a custom `HttpTransport` (see
[Custom HTTP transport](#custom-http-transport)) interprets it on its own.

## Sandbox / testing

**Integration code does not change between test and production — only the key changes.** All SDK
business methods work with a test key exactly as with a live one: a test `public_id` starts with
`test_`, a test secret with `oblodai_test_`. The helper `oblodai::is_test_key(public_id)` returns
`true` for a test key.

What is new are five helper methods on `client.sandbox()`. They have no live counterpart: they
stand in for "the customer paid on-chain". A live key gets a 403 `sandbox.live_key` on them.
**Sandbox calls are strictly TEST code**; do not weave them into your live integration.

```rust
use oblodai::{Client, Config};
use serde_json::json;

fn main() -> oblodai::Result<()> {
    // Test key — same constructor, same code.
    let client = Client::new(Config::new("test_...", "oblodai_test_..."))?;

    // 1. Create an invoice with the usual business method.
    let payment = client.payments().create(json!({
        "amount": "10", "currency": "USD", "order_id": "order-1",
        "to_currency": "USDT", "network": "tron",
    }))?;

    // 2. Simulate an on-chain payment (no fields — exactly the amount due, confirmed at once).
    client.sandbox().simulate_deposit(&payment.uuid, json!({}))?;

    // 3. Poll the invoice as in production — it becomes paid.
    let paid = client.payments().info(Some(&payment.uuid), None)?;
    println!("{}", paid.payment_status);

    // 4. Balance "out of thin air" — and a regular payout from it.
    client.sandbox().faucet("USDT", "1000", Some("seed-1"))?;
    client.payouts().create(json!({
        "amount": "25", "currency": "USDT", "network": "tron",
        "address": "T...", "order_id": "w-1",
    }))?;
    Ok(())
}
```

More involved scenarios:

```rust
// An underpayment "stuck" at 1 confirmation...
client.sandbox().simulate_deposit(&payment.uuid, json!({
    "amount": "5", "confirmations": 1, "txid": "tx-a",
}))?;
// ...repeating with the same txid and a HIGHER confirmations deepens the SAME deposit (idempotent).
client.sandbox().simulate_deposit(&payment.uuid, json!({
    "amount": "5", "confirmations": 12, "txid": "tx-a",
}))?;

client.sandbox().reset()?;                      // zero balances + cancel unpaid invoices
let deliveries = client.sandbox().list_webhooks()?; // up to 50 deliveries, newest first, with payload
client.sandbox().replay_webhook(&deliveries[0].id)?; // queue the delivery for redelivery
```

Fine points:

- **Shallow confirmations do NOT mature on their own.** A simulated deposit is not re-emitted and
  there is no chain behind it, so an invoice whose `confirmations` are below the required number
  hangs indefinitely — in `confirm_check` (deposit seen, waiting for confirmations), or, if less
  than the amount was sent, in `wrong_amount_waiting`. **Not `check`:** `check` means no payment
  has been seen at all. The only way to bring the invoice to `paid` is to repeat
  `simulate_deposit` with the **same `txid`** and a higher `confirmations` (this deepens the SAME
  deposit rather than creating a new one).
- **The ~10 minutes is about something else:** the maturity hold on a **payout**. Funds from a
  just-credited deposit cannot be withdrawn immediately — the attempt returns
  `payout.funds_maturing`. In the sandbox this hold is lifted by age (default 10 minutes,
  `GATEWAY_SANDBOX_MATURITY_MINUTES` on the gateway side) — or right away, if you repeat the same
  `txid` with a large depth. This mechanism has nothing to do with invoice confirmations.
- **`reset` is NOT a "clean slate".** Balances are zeroed, but only invoices in the `check`
  (internally `created`) and `select` statuses are cancelled — the ones where no payment has been
  seen. An invoice with a deposit already through (`confirm_check`, `wrong_amount_waiting`)
  **deliberately stays alive**: cancelling it would let the deposit mature into a cancelled
  invoice. If you need a truly clean run — create new invoices rather than counting on `reset` to
  clear out all the old ones.
- **UTXO networks (Bitcoin and the like):** no overpayment auto-refund and no payer address — the
  behavior is identical to production.
- `faucet`: capped at 1000000 per call; its `idempotency_key` is a **body** field of the request
  (not the `Idempotency-Key` header, unlike the creating business methods).
- `list_webhooks` is the only **signed GET**: the signature is computed over the same canonical
  string with an empty body (`{ts}\nGET\n/v1/sandbox/webhooks\n`).

## Verifying webhooks

A webhook signature differs from a request signature — the SDK does both. For incoming webhooks,
take the **raw body** and the `X-Webhook-Timestamp` / `X-Webhook-Signature` headers.

> ### ⚠ There are TWO secrets, and they differ
>
> | What | Where it comes from | What it is for |
> |---|---|---|
> | **API key secret** (`OBLODAI_SECRET`, `Config::secret`) | from the dashboard, together with `public_id` | signs the SDK's **outgoing** requests |
> | **Endpoint secret** (`endpoint_secret`) | the `secret` field in the `client.webhooks().register(url)` response | verifies **incoming** webhooks |
>
> Everywhere in this README, "secret" without qualification means the FIRST one — the API key. In
> `verify_webhook` / `construct_event` you need the SECOND. Plug in the API key and you will
> reject 100% of webhooks with `Error::Signature`. The endpoint secret is issued at first
> registration and survives a URL change — save it then and there.

```rust
use oblodai::{construct_event, verify_webhook, VerifyOptions, WebhookHeaders, Error};
use serde_json::Value;

// endpoint_secret is NOT the API key: it is the `secret` from client.webhooks().register(url).
fn handle(endpoint_secret: &str, raw_body: &[u8], ts: &str, sig: &str) {
    let headers = WebhookHeaders { timestamp: ts, signature: sig };

    // By default both the signature AND freshness are checked (replay protection, 5-minute window).
    match construct_event::<Value>(endpoint_secret, raw_body, &headers, &VerifyOptions::default()) {
        Ok(event) => {
            if event["type"] == "payment" && event["status"] == "paid" {
                // mark order event["order_id"] as paid (idempotent on uuid + status)
            }
        }
        Err(Error::Signature(_)) => { /* return 403 */ }
        Err(_) => { /* everything else */ }
    }
}
```

The endpoint secret is taken once at registration and stored in your config/storage:

```rust
let reg = client.webhooks().register("https://shop.example/hooks/oblodai")?;
// reg.secret — save THIS value; it is the one to verify webhooks with.
```

To disable replay protection: `VerifyOptions { max_age_seconds: 0, now: None }`.

### Registration is an upsert of the SINGLE endpoint

A project has exactly **one** webhook endpoint. A repeated `register()` with a DIFFERENT URL does
not add a second receiver — it **redirects** deliveries: the same `endpoint_id` is returned and
the old URL silently goes quiet. The secret is preserved in the process (deliveries already
queued are signed with it; reissuing would break them). If you need to fan out to several
consumers — accept the webhook with a single receiver of your own and distribute from there
yourself.

### Status in the webhook body

The `status` field of a payment event comes from the same table as `payment_status`, but
**without** `wrong_amount_waiting`: webhooks send the unrefined status, so a partial payment
arrives as `confirm_check`. Only `payments().info(...)` returns the refined value.

## Statuses

Statuses arrive as strings (`payment.payment_status`, `payout.status`), but the SDK provides enums
for them — [`PaymentStatus`], [`PayoutStatus`], [`PayoutLinkStatus`]. The model fields deliberately
remain `String` (a new value on the gateway side will not break response parsing), while typed
parsing is available through methods:

```rust
use oblodai::PaymentStatus;

let payment = client.payments().info(Some(&uuid), None)?;
match payment.status() {                       // -> PaymentStatus
    PaymentStatus::Paid | PaymentStatus::PaidOver => { /* ship the order */ }
    PaymentStatus::WrongAmount => { /* closed underpaid: resolve is possible */ }
    PaymentStatus::Cancel => { /* expired or cancelled */ }
    _ => { /* still in progress — keep polling */ }
}

// An unfamiliar value -> PaymentStatus::Unknown; response parsing does not fail on it.
assert_eq!(PaymentStatus::from_api("confirm_check"), PaymentStatus::ConfirmCheck);
assert_eq!(PaymentStatus::Paid.as_str(), "paid");
```

### Payment statuses

| Value | `PaymentStatus` | Meaning | Terminal |
|---|---|---|---|
| `check` | `Check` | Invoice created, no payment seen yet. | no |
| `confirm_check` | `ConfirmCheck` | Payment seen on the network, waiting for confirmations. | no |
| `wrong_amount_waiting` | `WrongAmountWaiting` | **Partial** payment seen, waiting for the remainder. | **no** |
| `wrong_amount` | `WrongAmount` | Invoice closed underpaid. | yes |
| `paid` | `Paid` | Paid in full. | yes |
| `paid_over` | `PaidOver` | Overpaid (the excess goes to auto-refund, if enabled and the network supports it). | yes |
| `cancel` | `Cancel` | Expired or cancelled. | yes |
| `select` | `Select` | Currency-agnostic invoice: the buyer has not chosen a currency and network yet. | no |

Terminality is duplicated by the `is_final` field in the response — on unfamiliar values rely on
it, not on `Unknown`.

> **`wrong_amount_waiting` ≠ `wrong_amount`.** The first is a LIVE invoice: the payer sent less,
> but can still top up the amount, and the invoice will become `paid`. The second is one already
> closed underpaid. `payments().resolve(...)` works **only** with the second: on
> `wrong_amount_waiting` the gateway replies `409 resolution.not_underpaid`. You can check via
> `payment.status().is_resolvable()`.

### Payout statuses

| Value | `PayoutStatus` | Meaning | Terminal |
|---|---|---|---|
| `check` | `Check` | Created, awaiting approval (`payouts().approve`). | no |
| `process` | `Process` | Approved: the transaction is being built, broadcast, or already on the network. | no |
| `paid` | `Paid` | Confirmed on the network. | yes |
| `fail` | `Fail` | Failed. | yes |
| `cancel` | `Cancel` | Cancelled. | yes |

The same vocabulary applies to the refund-payout status in the `resolve` response
(`Resolution::payout_status()`) and to mass payout items (`MassPayoutItem::status()`).

> **Exception:** `PaymentLinkPayment::status` (the list of payments on a payment link) returns the
> invoice's **internal** literals — `created`, `expired`, `cancelled` instead of `check` and
> `cancel`. For the canonical status, go to `payments().info(uuid, None)`.

## Error handling

All errors are the [`Error`] enum. API errors carry a machine-readable code.

```rust
use oblodai::Error;

match client.payouts().create(params) {
    Ok(payout) => { /* ... */ }
    Err(Error::Api { code, status, message, .. }) => {
        match code.as_str() {
            "payout.insufficient_funds" => { /* not enough funds */ }
            // Funds are still maturing. The error is TERMINAL: is_retriable() == false, the SDK
            // does not retry it — an immediate retry would not help. Wait and call again yourself.
            "payout.funds_maturing"    => { /* ... */ }
            _ => {}
        }
        eprintln!("{code} (HTTP {status}): {message}");
    }
    Err(e) => eprintln!("{e}"),
}
```

`err.is_retriable()` tells you whether the error is transient; `err.code()` returns the code for
`Error::Api`.

### Error variants

| Variant | When |
|---|---|
| `Error::Api { code, status, .. }` | The API returned an `error` envelope. |
| `Error::Connection(_)` | Network unreachable or timeout. |
| `Error::Signature(_)` | Webhook signature verification failed. |
| `Error::Serialization(_)` | (De)serialization error. |
| `Error::Config(_)` | Invalid configuration. |

## Retries

Transient errors (`5xx`, `429`, network failures) are retried automatically with exponential
backoff and jitter. Request errors (`4xx`), as well as `payout.funds_maturing` (funds are still
maturing — an immediate retry would not help), are considered terminal and are not retried. Among
the idempotency codes that means: `503 idempotency.unavailable` the SDK replays itself (with the
same key), while `409 idempotency.in_progress`, `409 payoutlink.duplicate_reference`,
`400 idempotency.key_reused`, and `400 idempotency.bad_key` are returned to you as is. The
server's `Retry-After` header is honored as is (not clamped to `max_delay`, only to the absolute
ceiling of 300 s).

```rust
use oblodai::{Config, RetryConfig};
use std::time::Duration;

let config = Config::new("...", "...").retry(Some(RetryConfig {
    max_attempts: 4,
    initial_delay: Duration::from_millis(500),
    max_delay: Duration::from_secs(30),
}));
// .retry(None) — disable retries
```

> **Important about the timeout.** A timeout does not mean the operation failed. Retrying is safe
> thanks to the `Idempotency-Key` header (see below): if the operation was already created, the
> server returns that same one — there will be no duplicate.

## Idempotency (v1.1.0, breaking change)

Creating calls (`payments().create/refund/resolve/create_batch/refund_batch`,
`payouts().create/create_mass/create_batch/refund`, `account().transfer_to_personal`,
`account().transfer_to_user/transfer_batch`) send the **`Idempotency-Key`** header — a UUID v4
generated **once, before any retries**: all internal retries of a single call go out with the same
key, so timeout + retry does not create a duplicate. The header is not part of the request
signature.

- **`order_id` is NO longer filled in automatically** (the v1.0.x behavior has been removed). It
  is sent as is — it is your business identifier; set it explicitly so you can later find the
  operation via `info`. For payouts `order_id` is always required (`payout.order_id_required`).
- **Your own idempotency key** (deduplication across calls/processes): pass an `idempotency_key`
  field in the parameters of a creating call — it goes into the header and **does not end up in
  the body**:

```rust
client.payments().create(json!({
    "amount": "10", "currency": "USD", "order_id": "ord-1",
    "idempotency_key": "3f8a2c1e-...-your-uuid",
}))?;
```

- **`payout_links().create/create_batch`** send the header too: both operations reserve balance,
  and without it a lost response + auto-retry would fund a second link. A retry with the same key
  replays the first response (same link, same `claim_token`), and the balance is debited exactly
  once; the response is marked with the `Idempotent-Replayed: true` header. **Without** the header
  (for example, if you call the API bypassing the SDK), two identical calls create TWO links.
  Additionally deduplicate creation via a per-link `reference`. A batch caveat: a partially failed
  batch is replayed as is — the failed items will not be resent under the same key; send them in a
  new call.

### Response codes of the idempotency layer

On idempotent routes (including `/v1/payout/link` and `/v1/payout/link/batch`) the gateway may
respond:

| Code | HTTP | Meaning | Retried by the SDK? |
|---|---|---|---|
| `idempotency.key_reused` | 400 | Same key with a DIFFERENT body. | no (terminal) |
| `idempotency.bad_key` | 400 | Invalid key (e.g. longer than 255 characters). | no (terminal) |
| `idempotency.in_progress` | 409 | Concurrent retry while the first request is still executing. | no — retry yourself later |
| `idempotency.unavailable` | 503 | The idempotency store is unavailable; the gateway fails closed and does NOT execute the operation. | yes, automatically |

Classification is exactly as with regular errors: `4xx` are terminal (`is_retriable() == false`),
`5xx` are retried. That is, the SDK replays a 503 itself with the SAME key, while on a 409
`idempotency.in_progress` it returns the error to you — wait and repeat the call, passing the
same `idempotency_key` explicitly.

> **Batches and the cache limit.** The gateway does not cache responses larger than 256 KB — a
> retry with the same key will then execute ANEW. This is realistically reachable on large payout
> link batches, so set a per-item `reference`: the unique index on the gateway side remains the
> second, durable layer of protection and works even without the header and even when the response
> did not fit in the cache. A duplicate `reference` returns `409 payoutlink.duplicate_reference`
> (previously it was a `500`, which the SDK retried; now it is a terminal conflict — no retry).
- **`wallets().blocked_address_refund` is an exception:** the header is not sent and is not
  needed. The refund is idempotent by state: the server derives the payout link from the wallet id
  and looks it up under a per-wallet lock, so a retry returns the SAME payout. This is stronger
  than the header — concurrent retries serialize instead of conflicting.
- **`payouts().approve` is an exception:** it is a status transition, not a creation. A repeated
  approve of an already approved payout replies `409 payout.not_pending`; read that as "already
  approved" and double-check via `payouts().info(...)`.

## Custom HTTP transport

By default the built-in `reqwest`-based client is used (the `reqwest-client` feature, enabled). To
plug in your own transport (or for tests without a network), disable the feature and implement the
[`HttpTransport`] trait:

```toml
[dependencies]
oblodai = { version = "1", default-features = false }
```

```rust
use oblodai::{Client, Config, HttpTransport, HttpResponse, Error};
use std::sync::Arc;

struct MyTransport;
impl HttpTransport for MyTransport {
    fn post(&self, url: &str, headers: &[(String, String)], body: &[u8])
        -> Result<HttpResponse, Error> {
        // your HTTP call
        # let _ = (url, headers, body);
        Ok(HttpResponse { status: 200, body: b"{}".to_vec() })
    }
}

let client = Client::with_transport(Config::new("p", "s"), Arc::new(MyTransport)).unwrap();
```

## Transfers to users (v1.2.0)

An internal **fee-free** transfer from the merchant balance to the personal wallet of a platform
user (payout key). `to_user_id` is the user's id (**UUID, not username**): a username is resolved
to an id via the dashboard's public profile. Idempotency is the same as for the other money
methods: the `Idempotency-Key` header (your own key — the `idempotency_key` field); on the backend
the ladder is "header → `order_id` → signature".

```rust
let res = client.account().transfer_to_user(json!({
    "to_user_id": "5c3f8a2c-9b1d-4e6f-8a2c-1e9b7d5f3a10",
    "amount": "25", "currency": "USDT", "order_id": "salary-7",
}))?;
println!("{}", res.recipient_balance);

// A batch (payroll, up to 5000): results — via batches().info(batch_id, ...)
let sub = client.account().transfer_batch(vec![
    json!({ "to_user_id": "…", "amount": "25", "currency": "USDT", "order_id": "s-1" }),
    json!({ "to_user_id": "…", "amount": "30", "currency": "USDT", "order_id": "s-2" }),
], Some("continue"))?;
println!("{}", sub.batch_id);
```

## Public pay for your own checkout page (v1.2.0)

`GET /v1/pay/{id}` and `POST /v1/pay/{id}/select` are public (unsigned), like `/v1/link/{id}` and
`/v1/claim/{token}`: they can be called straight from the payment page, no secret needed.
`public_get` returns the invoice's client-facing fields — address, amount, QR, status, deadline
(the backend does not expose the private `additional_data` / `payer_email`); for a
currency-agnostic invoice in the `select` status — also the list of methods to choose from.
`public_select` locks in the currency+network, the rate, and the deposit address.

```rust
let state = client.payments().public_get(&payment.uuid)?;   // status for polling
if state.payment_status == "select" {
    let finalized = client.payments().public_select(&payment.uuid, "USDT", "tron")?;
    println!("{}", finalized.address);
}
```

## Method overview

> **Resource names are uniform across all Oblodai SDKs.** The canon for payment links is
> `payment_links` (`paymentLinks` in JS/PHP, `PaymentLinks` in Go). The short name `links` is kept
> as a **documented alias**: `client.links()` is the same resource as `client.payment_links()`,
> with the same methods. Both names are supported, nothing is deprecated; the alias exists so code
> ports between languages without renames.

```rust
// Payments
client.payments().create(params)
client.payments().info(uuid, order_id)
client.payments().history(params)
client.payments().services()
client.payments().qr(uuid, order_id)
client.payments().resend(uuid, order_id)
client.payments().refund(params)
client.payments().set_accepted(methods) / list_accepted()
client.payments().set_accuracy(params) / get_accuracy()
client.payments().set_autorefund(params) / get_autorefund()
client.payments().create_batch(payments, on_error)      // payment batch (up to 5000)
client.payments().refund_batch(refunds, on_error)       // refund batch
client.payments().send_email(uuid, order_id, email)     // e-mail invoice
client.payments().resolve(ResolveAction::Accept, params) // fate of an underpayment (ONLY wrong_amount)
client.payments().public_get(uuid)                      // public, unsigned (your own checkout)
client.payments().public_select(uuid, currency, network) // public: currency+network selection

// Payouts
client.payouts().create(params)
client.payouts().create_mass(payouts, source)
client.payouts().create_batch(payouts, on_error)        // payout batch (up to 5000)
client.payouts().info(uuid, order_id)
client.payouts().history(params)
client.payouts().services()
client.payouts().calculate(params)
client.payouts().approve(uuid)
client.payouts().refund(params)
client.payouts().get_fee_config() / set_fee_config(bool)
client.payouts().get_refund_fee_config() / set_refund_fee_config(bool)

// Wallets
client.wallets().create(params)
client.wallets().block(address, force_block)
client.wallets().blocked_address_refund(uuid, address)
client.wallets().qr(address)

// Account
client.account().balance()
client.account().referral()
client.account().transfer_to_personal(params)
client.account().transfer_to_user(params)               // transfer to a user (fee-free)
client.account().transfer_batch(transfers, on_error)    // transfer batch (up to 5000)
client.account().vrcs(enabled)

// Webhooks
client.webhooks().register(url)                         // upsert of the project's SINGLE endpoint
client.webhooks().deliveries()
client.webhooks().test_payment(params)

// Settings
client.settings().list_auto_withdraw() / set_auto_withdraw(params) / delete_auto_withdraw(currency)
client.settings().list_allowlist() / add_allowlist(cidr) / remove_allowlist(cidr) / enable_allowlist(bool)

// Bulk operations: batch state
client.batches().info(batch_id, limit, offset)

// Payment links (reusable, "donation-style"); client.links() is an alias of the same resource
client.payment_links().create(params)
client.payment_links().list(limit, offset) / info(link_id) / toggle(link_id, active)
client.payment_links().public_get(link_id)              // public, unsigned
client.payment_links().checkout(link_id, params)        // public, unsigned

// Split payments
client.splits().create_rule(params)                     // {address,network} OR {merchant_id} + percent
client.splits().list_rules() / delete_rule(rule_id)
client.splits().get_config() / set_config(refund_hold_hours)

// Payout links ("crypto checks", payout key)
client.payout_links().create(params)                    // set expires_in_hours explicitly!
client.payout_links().create_batch(links)               // up to 500 links
client.payout_links().list(limit, offset) / info(link_id) / cancel(link_id)
client.payout_links().claim_info(token)                 // public, unsigned
client.payout_links().claim(token, address, memo)       // public, unsigned

// Rates (public, no key)
client.rates().list(Some("ETH"))

// Sandbox (test key ONLY; a live key gets 403 sandbox.live_key)
client.sandbox().simulate_deposit(invoice_id, params) // simulate an on-chain deposit
client.sandbox().faucet(asset, amount, idem)          // test balance (capped at 1000000)
client.sandbox().reset()                              // zero balances + cancel unpaid invoices
client.sandbox().list_webhooks()                      // up to 50 deliveries (signed GET)
client.sandbox().replay_webhook(delivery_id)          // queue the delivery for redelivery
```

### Payout links in brief

You reserve funds into a link **without knowing the recipient's wallet**; the recipient opens the
public `claim_url` page and enters an address — a regular payout is spawned.
`claim_token`/`claim_url` are returned **only in the `create` response** — save them right away.
**Set `expires_in_hours` (1–720) explicitly**: with 0/absent, the backend clamps the lifetime to
**1 hour**. An unclaimed link returns the reserve when it expires or on `cancel`.

```rust
let link = client.payout_links().create(json!({
    "currency": "USDT", "network": "tron", "amount": "25",
    "reference": "bonus-42",            // your deduplication key; duplicate → 409 payoutlink.duplicate_reference
    "expires_in_hours": 168,            // 7 days; without this field — just 1 hour!
    "email": "user@example.com",        // optional: claim e-mail to the recipient
}))?;
println!("{}", link.claim_url); // hand this to the recipient
```

> **`claim_url` and `payment.url` may arrive as an EMPTY string.** The gateway builds them from
> its public base URL (`GATEWAY_PUBLIC_BASE_URL`). In production the gateway does not start
> without it, but on a local or test setup it often goes unset — and both links arrive empty.
> This is not an SDK bug and not a gateway bug. If you test locally, build the link yourself from
> the token/identifier: `{your_base_url}/claim/{claim_token}` and `{your_base_url}/pay/{uuid}`.

## Notes

- **Amounts are strings** in currency units (`"25.00"`), not numbers. This preserves precision.
- **Duplicates are prevented by the `Idempotency-Key` header** (see the "Idempotency" section).
  `order_id` is your business identifier for looking up the operation, set it explicitly; the SDK
  does not fill it in.
- **The secret belongs on the server only.** The SDK is server-side; do not embed the key in
  client applications.
- **There are two secrets.** The API key signs outgoing requests; webhooks are verified with the
  **endpoint secret** from `webhooks().register(...).secret` — see "Verifying webhooks".
- **Statuses** are typed with the `PaymentStatus` / `PayoutStatus` / `PayoutLinkStatus` enums (the
  fields remain strings) — see "Statuses".
- **`payment.url` and `claim_url`** are built by the gateway from its public base URL and arrive
  empty on a setup without one: build the link yourself from the `uuid` / `claim_token`.
- **Request bodies** are accepted as `serde_json::Value` — build them with the `json!` macro.

## License

MIT
