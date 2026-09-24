<div align="center">

<a href="https://oblodai.com">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/oblodai/.github/main/brand/logo-white.svg">
    <img src="https://raw.githubusercontent.com/oblodai/.github/main/brand/logo-black.svg" alt="oblodai" height="52">
  </picture>
</a>

<h3>Official Rust SDK for the <a href="https://oblodai.com">oblodai</a> payment gateway</h3>

Payments, payouts, payment links, splits, static wallets, webhooks — one API key.

<a href="https://crates.io/crates/oblodai"><img src="https://img.shields.io/crates/v/oblodai?style=flat-square&label=crates.io" alt="crates.io"></a>
<a href="https://github.com/oblodai/oblodai-rust/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/oblodai/oblodai-rust/ci.yml?branch=main&style=flat-square&label=CI" alt="CI"></a>
<img src="https://img.shields.io/badge/MSRV-1.86-DEA584?style=flat-square" alt="MSRV 1.86">
<a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-000000?style=flat-square" alt="License: MIT"></a>

[Documentation](https://docs.oblodai.com) · [Dashboard](https://my.oblodai.com) · [Читать по-русски →](README.ru.md)

</div>

---

The official Rust SDK for the **Oblodai** payment gateway: accepting payments, payouts, bulk
operations (batches), payment links, payout links (crypto cheques), splits, static wallets,
transfers, webhooks, documents. Every namespace, method and model is **generated from the
gateway's OpenAPI contract** — 120 operations, one method each — over a hand-written runtime that
signs requests, retries safely, keeps one idempotency key per call and names every call with an
`X-Request-ID`. Rust 2021, **MSRV 1.86**, async on `reqwest` + `tokio` with rustls (no OpenSSL); a
synchronous client sits behind a feature flag, and with `--no-default-features` the models, the
money helpers, webhook verification and the `HttpBackend` seam build with no `reqwest` at all.

> **Base URL.** Defaults to `https://api.oblodai.com`. Override `base_url` and supply your own keys
> at initialisation if needed. The scheme must be `https://`; plain `http://` is accepted only for
> loopback (`http://127.0.0.1:8095`) or with the explicit `allow_insecure_base_url` option.

## Installation

```toml
[dependencies]
oblodai = "2.0"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
futures-util = "0.3"   # only for `.stream()` / `.by_page()` — the `StreamExt` adapters live there
```

Requires Rust **1.86** or newer. The floor comes from the dependency tree, not from the SDK's own
code; raising it is a minor-version change.

| feature           | what it adds                                                                                          |
| ----------------- | ----------------------------------------------------------------------------------------------------- |
| `reqwest-client`  | *(default)* the async `Client` over `reqwest` with rustls                                             |
| `blocking`        | a synchronous `blocking::Client` over the same pure core                                              |
| `native-roots`    | also trust the OS certificate store (a TLS-inspecting proxy, a gateway with a private CA)             |

### Blocking client

```toml
oblodai = { version = "2.0", features = ["blocking"] }
```

```rust
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
```

The same generated method tree over the same pure core; builders are sent with `.send()` and lists
iterate with `.iter()` / `.by_page()`.

## Where to get keys

Keys live in the dashboard at [my.oblodai.com](https://my.oblodai.com) → **API keys**. A key is a
public id plus a secret:

| key     | public id            | secret                |
| ------- | -------------------- | --------------------- |
| live    | `oblodai_<hex>`      | `oblodai_live_<hex>`  |
| sandbox | `test_oblodai_<hex>` | `oblodai_test_<hex>`  |

A merchant has **one API key**, and it signs every route the SDK can call — there is nothing to
choose per call:

```rust
let client = oblodai::Client::builder()
    .public_id(public_id)
    .secret(secret)
    .build()?;
```

`Client::from_env()` reads `OBLODAI_PUBLIC_ID` / `OBLODAI_SECRET`. A **sandbox key** drives a
chainless copy of the gateway — fake balance from a faucet, simulated deposits, real webhooks —
from the dashboard or `sandbox().onboard_store(merchant_id)`. Integrate against it first.

A second credential, the **onboarding admin token**, exists only on a self-hosted gateway: it is
sent as `X-Admin-Token` on the store provisioning route (`sandbox().onboard_store`) and nowhere
else. Set it with `.admin_token(…)` or `OBLODAI_ADMIN_TOKEN`.

Only a merchant onboarded long before the single-key cleanup can still hold a legacy split pair
(`oblodai_pk_…` / `oblodai_wk_…`); such a pair is refused on the other half's routes with a 403
`merchant.wrong_key_kind`. Replace it with the merchant's API key.

## Quick start

```rust
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
```

Request bodies are typed models (`oblodai::models`): `Model::new(required…)` sets the required
fields, and struct update syntax fills the rest. Amounts are `Money` — a decimal string; there is no
`From<f64>`, so a float does not compile (and one arriving through `oblodai::from_json` is refused
with `sdk.float_amount` before anything is sent).

Sending money out uses the same key:

```rust
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
```

More runnable programs live in [`examples/`](examples) — each one runs in `tests/examples.rs`
against a fake gateway.

## Call options

Every method returns a builder; nothing is sent until it is awaited (`.send()` on the blocking
client). The options are set on the builder:

```rust
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
```

| option                 | what it does                                                                                     |
| ---------------------- | ------------------------------------------------------------------------------------------------ |
| `.idempotency_key(k)`  | your own key (routes the gateway deduplicates; elsewhere `sdk.idempotency_unsupported`)          |
| `.timeout(Duration)`   | one attempt (default 30 s)                                                                       |
| `.deadline(Duration)`  | the whole call, retries and pauses included (default 90 s)                                       |
| `.max_retries(n)`      | retries after the first attempt for this call (default 2; `0` sends once)                        |
| `.extra_header(n, v)`  | a header on this call only; the SDK's own headers are never overridden                           |
| `.request_id(id)`      | `X-Request-ID` of the call — the same on every attempt; a fresh UUID when not set                |

The request id links your logs with the gateway's: it is in every error's text,
`[payout.insufficient_funds] not enough USDT (request_id=…)`, and in `err.request_id()`.

## Method overview

`client.<resource>().<method>(…)` — sixteen namespaces, one method per OpenAPI operation, the name
being the `operationId` without the resource. `names.lock` pins every name; the full list with the
1.x names is in [MIGRATION-2.0.md](MIGRATION-2.0.md).

| namespace           | examples                                                                                  |
| ------------------- | ----------------------------------------------------------------------------------------- |
| `payments()`        | `create` · `get_info` · `cancel` · `list_history` · `get_qr` · `resolve` · `send_email`   |
| `payment_links()`   | `create` · `get` · `list` · `toggle`                                                      |
| `refunds()`         | `payment` · `blocked_wallet`                                                              |
| `payouts()`         | `create` · `calculate` · `validate` · `get_info` · `list_history` · `transfer_to_user`    |
| `payout_links()`    | `create` · `create_batch` · `get` · `list` · `cancel` · `claim_payout`                    |
| `batches()`         | `create_payment` · `create_payout` · `create_refund` · `get_info`                         |
| `splits()`          | `create_rule` · `list_rules` · `get_config` · `set_config`                                |
| `wallets()`         | `create` · `block` · `get_qr`                                                             |
| `account()`         | `get_balance` · `get_summary` · `list_exchange_rates`                                     |
| `webhooks()`        | `register` · `rotate_secret` · `list_deliveries` · `send_test_payment`                    |
| `settings()`        | accuracy, discounts, auto-refund, auto-convert, fees, accepted currencies, auto-withdraw  |
| `api_allowlist()`   | `list` · `add_entry` · `remove_entry` · `set_enabled`                                     |
| `referrals()`       | `get_info`                                                                                |
| `documents()`       | `get_statement` · `get_ledger` · `create_job` · `get_job` · `download_job_file`           |
| `checkout()`        | payer-facing, no credentials: `get` · `select_method` · `list_currencies`                 |
| `sandbox()`         | `faucet` · `simulate_deposit` · `list_webhooks` · `replay_webhook` · `reset`              |

A document route answers with `FileResult { bytes, content_type, filename }`. Models keep the
fields this SDK version does not know in `extra`, and every enum has an `Other(String)` variant, so
an answer from a newer gateway still decodes. `Debug` of a model never prints a secret-looking
field (`secret`, `token`, `passcode`, `claim_url`, …).

### Lists

List methods return a `Pager`. Nothing is requested until you consume it.

```rust
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
```

### Long-running operations

Batches and document exports finish in the background. `.job()` sends the create call and returns
a `Job` that knows how to follow it:

```rust
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
```

`wait()` polls every 2 s for at most 5 minutes (`wait_with(timeout, interval)` to change that) and
returns the terminal answer — a `failed` job is returned, not raised; running out of time is
`sdk.job_timeout`. Which operations are long-running is a table of this SDK (`oblodai::lro::LRO`).

### Raw responses, client copies, hooks

```rust
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
```

### Statuses and amounts

- Payment: `select → created → confirm_check → paid | paid_over | wrong_amount | expired | cancelled`
  (`under_review` in between). `helpers::is_payment_paid(&status)` is true for `paid`/`paid_over`;
  `wrong_amount` (underpaid) waits for `payments().resolve(…)`.
- Payout: `pending → approved → awaiting_cosign → broadcasting → sent → confirmed | failed | cancelled`.
- `add_amounts`, `subtract_amounts`, `compare_amounts`, `amounts_equal`, `is_zero_amount` — exact
  decimal arithmetic on `Money`. `Money` has no `Ord`: string order is not numeric order.

## Sandbox / testing

With a `test_` key the gateway keeps a full merchant that never touches a chain:

```rust
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
```

A rehearsal delivery carries `test: true` in the signed body and `X-Webhook-Test: true` in the
headers, surfaced as `delivery.is_test` — never credit an order on one.

## Webhooks

Register an endpoint with `webhooks().register(…)` — the signing secret is returned once — and
verify every delivery over the **raw** bytes, before any parsing:

```rust
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
```

- **Duplicates and order.** `delivery.id` (`X-Webhook-Id`) is stable across retries — deduplicate
  on it. `event.sequence()` orders events; `is_stale_event` drops an out-of-order one.
- **Rotation.** After `webhooks().rotate_secret()` pass `.previous_secret(old)` until
  `previous_secret_valid_until` has passed.
- **Unknown event types** arrive as `WebhookEvent::Other(Value)` (the enum is `#[non_exhaustive]`);
  the known ones are `Payment`, `Payout`, `Wallet` and `Conversion`, over the generated models.
- **Bad payload is not a bad signature.** A verified delivery whose body cannot be read is
  `webhook.bad_payload` (`kind() == Contract`). Answer 401 only for signature failures.

The `oblodai::webhooks` module needs no client and no API key; it builds with
`--no-default-features`.

## Errors

Every failure is an `oblodai::Error`: `code()` (`payout.insufficient_funds`), `http_status()`,
`retryable()`, `retry_after()`, `request_id()`, `field()`, `synthetic()` (a proxy answered, not the
API) and `kind()` for matching (`Validation` 400, `Authentication` 401, `Permission` 403,
`NotFound` 404, `Conflict` / `IdempotencyConflict` 409, `RateLimit` 429, `Unavailable` 503,
`Internal` other 5xx, `Transport`, `Config`, `Contract`, `Signature`). Printed, it reads
`[code] message (request_id=…)`. Branch on the code, not on the message:

```rust
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
```

Codes the SDK raises itself, never the gateway: `sdk.missing_credentials`, `sdk.bad_config`,
`sdk.bad_header`, `sdk.bad_path_param`, `sdk.bad_idempotency_key`, `sdk.idempotency_unsupported`,
`sdk.float_amount`, `sdk.bad_params`, `sdk.bad_amount`, `sdk.bad_envelope`,
`sdk.response_too_large`, `sdk.job_timeout`, `sdk.no_download`, `webhook.bad_payload`, and the
`transport.timeout` / `transport.network` / `transport.deadline` family.

## Retries, idempotency and timeouts

- **Safe to repeat** comes from the contract: `GET` and the operations marked `x-retry-safe`, or a
  write the gateway deduplicates by `Idempotency-Key` (`x-idempotent`). No path-shape heuristics.
- **Idempotency keys** are attached automatically on deduplicated routes — one per logical call,
  reused on every retry — so a timeout can never produce a second payout.
- **When a retry happens.** Only when the API says `retryable: true`; answers without an API
  envelope (a proxy 502/503) and transport failures only when repeating is safe. `Retry-After` is
  honoured, otherwise exponential backoff with jitter. Defaults: 2 retries, 250 ms → 4 s.
- **Clock skew** is corrected once, from the server's `Date` header, and reverted if it did not help.
- **Redirects are never followed**; response bodies are capped (8 MiB JSON, 64 MiB documents).
- **Bound a call with `.deadline(…)`, not by dropping the future**: the auto-generated key lives in
  that future. If a retry has to survive a restart, pass your own `.idempotency_key(…)`.

## Configuration

`Client::new(public_id, secret)`, `Client::from_env()` or `Client::builder()`:

| option                          | default                    | meaning                                                   |
| ------------------------------- | -------------------------- | --------------------------------------------------------- |
| `.public_id(…)` / `.secret(…)`  | —                          | the API key; give both or neither                         |
| `.base_url(…)`                  | `https://api.oblodai.com`  | API origin; a path prefix is kept                         |
| `.timeout(…)` / `.deadline(…)`  | 30 s / 90 s                | one attempt / the whole call                              |
| `.max_retries(n)`, `.retry(…)`  | 2 retries                  | `0` disables retries                                      |
| `.header(name, value)`          | —                          | an extra header on every request                          |
| `.hooks(Hooks)`                 | none                       | `on_request` / `on_response`, once per attempt            |
| `.admin_token(…)`               | —                          | `X-Admin-Token` on the provisioning route only            |
| `.allow_insecure_base_url(…)`   | `false`                    | permit plain `http` beyond loopback                       |
| `.logger(…)`                    | none                       | a structured logger; secrets are redacted before it       |
| `.http_backend(…)`              | `reqwest`                  | replace the HTTP layer                                    |

Environment: `OBLODAI_PUBLIC_ID`, `OBLODAI_SECRET`, `OBLODAI_BASE_URL`, `OBLODAI_ADMIN_TOKEN`,
`OBLODAI_LOG` (`debug|info|warn|error`), `OBLODAI_ALLOW_INSECURE` (`1`).

## Development

The code under `src/generated/` is generated by the backend's `tools/sdkgen` from
`services/core/api/openapi.json`; never edit it by hand — regenerate with `make sdk` in the
backend. `make ci` runs every gate: the drift check (regenerate into a temporary directory and
compare), `cargo fmt`, clippy over the feature combinations, the tests (unit, the shared
conformance suite of the backend, the examples and this README run against a fake gateway), rustdoc,
packaging and the MSRV check.

```sh
OBLODAI_BACKEND=../oblodai-backend make ci
OBLODAI_LIVE_URL=http://127.0.0.1:8095 make live   # against a real gateway
```

See [AGENTS.md](AGENTS.md) for a condensed guide aimed at coding agents,
[CHANGELOG.md](CHANGELOG.md) for what changed, and [MIGRATION-2.0.md](MIGRATION-2.0.md) for the
move from 1.x.

## License

MIT — see [LICENSE](LICENSE).
