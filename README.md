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
transfers, webhooks. Request signing, response parsing, typed errors, idempotency and retries — out
of the box. Rust 2021, **MSRV 1.86**, async on `reqwest` + `tokio` with rustls (no OpenSSL); a
synchronous client sits behind a feature flag, and with `--no-default-features` the contract types,
the money helpers, webhook verification and the `HttpBackend` seam build with no `reqwest` in the
tree at all.

> **Base URL.** Defaults to `https://api.oblodai.com`. Override `base_url` and supply your own keys
> at initialisation if needed. The scheme must be `https://`; plain `http://` is accepted only for
> loopback (`http://127.0.0.1:8095`) or with the explicit `allow_insecure_base_url` option.

## Installation

```toml
[dependencies]
oblodai = "1.3"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
futures-util = "0.3"   # only for `.stream()` — the `StreamExt` adapters live there
```

Requires Rust **1.86** or newer, checked in CI on exactly that toolchain. The floor comes from the
dependency tree (`reqwest` → `url` → `idna`/`icu`), not from the SDK's own code; raising it is a
minor-version change.

Feature flags:

| feature           | what it adds                                                                                                                                                   |
| ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `reqwest-client`  | *(default)* the async `Client` over `reqwest` with rustls                                                                                                      |
| `blocking`        | a synchronous `blocking::Client` over the same pure core                                                                                                       |
| `native-roots`    | also trust the OS certificate store — needed behind a TLS-inspecting proxy or against a gateway with a private CA; without it the bundled webpki roots are used |

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

## Where to get keys

Keys live in the dashboard at [my.oblodai.com](https://my.oblodai.com) → **API keys**. A key is a
public id plus a secret:

| key     | public id            | secret                |
| ------- | -------------------- | --------------------- |
| live    | `oblodai_<hex>`      | `oblodai_live_<hex>`  |
| sandbox | `test_oblodai_<hex>` | `oblodai_test_<hex>`  |

A merchant has **one API key**, and it signs every route the SDK can call: invoices and payment
links, payouts, refunds and cheques, settings, wallets, reports. There is nothing to choose per
call:

```rust
let client = oblodai::Client::builder()
    .public_id(public_id)
    .secret(secret)
    .build()?;
```

A **sandbox key** drives a chainless copy of the gateway — fake balance from a faucet, simulated
deposits, real webhooks — and comes from the sandbox onboarding (`merchants().sandbox(…)`, or the
dashboard). Integrate against it first; live and sandbox keys are separate and neither can touch the
other's data.

A second credential, the **onboarding admin token**, exists only on a self-hosted gateway: it is
sent as `X-Admin-Token` on the `merchants()` provisioning routes and nowhere else. Set it with
`.admin_token(…)` or `OBLODAI_ADMIN_TOKEN`.

Only a merchant onboarded long before the single-key cleanup can still hold a legacy split pair
(`oblodai_pk_…` for money in, `oblodai_wk_…` for money out); such a pair is refused on the other
half's routes with a 403 `merchant.wrong_key_kind`. Replace it with the merchant's API key.

## Quick start

Create a client from the environment and accept a payment:

```rust
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
```

`Client::from_env()` reads `OBLODAI_PUBLIC_ID` / `OBLODAI_SECRET` (and the other `OBLODAI_*`
variables below) but does not require them: a client with no credentials builds fine and fails on
the first signed call with `sdk.missing_credentials`.

To price in fiat, charge in one currency and settle in another: `amount: "25".into(),
currency: "USD".into(), to_currency: Some("USDT".into())` — `currency` is what you charge,
`to_currency` the asset the payer sends.

Sending money out uses the same key:

```rust
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
```

More runnable programs live in [`examples/`](examples): an invoice watched to settlement, a payout
quoted and dry-run before it is sent, and a webhook receiver.

## Sandbox / testing

With a `test_` key the gateway keeps a full merchant that never touches a chain. Credit yourself,
simulate the deposit, rehearse the webhook, then wipe it:

```rust
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
```

`sandbox().replay(delivery_id)` re-sends a delivery that has already reached a terminal state.
`sandbox().reset()` cancels open invoices and zeroes balances. A rehearsal delivery
carries `test: true` in the signed body and `X-Webhook-Test: true` in the headers, surfaced as
`delivery.is_test` — never credit an order on one.

## Method overview

Sixteen namespaces cover all **107 merchant routes**.

| namespace         | methods                                                                                                                                                                                                  | routes                                                                                                                                                                                                                                                    |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `payments()`      | create · info/get · cancel · history/list · batch · qr · services · send_email · resend · public_view · select · public_qr                                                                                | `POST /v1/payment` · `/payment/info` · `/payment/cancel` · `/payment/history` · `/payment/batch` · `/payment/qr` · `/payment/services` · `/payment/send-email` · `/payment/resend` · `GET /v1/pay/{id}` · `POST /v1/pay/{id}/select` · `GET /v1/pay/{id}/qr` |
| `refunds()`       | create · resolve · batch                                                                                                                                                                                 | `POST /v1/payment/refund` · `/payment/resolve` · `/refund/batch`                                                                                                                                                                                           |
| `payouts()`       | create · validate · calculate · info/get · cancel · approve · history/list · mass · batch · services · get/set_fee_config · get/set_refund_fee_config                                                     | `POST /v1/payout` · `/payout/validate` · `/payout/calculate` · `/payout/info` · `/payout/cancel` · `/payout/approve` · `/payout/history` · `/payout/mass` · `/payout/batch` · `/payout/services` · `/payout/fee-config/{get,set}` · `/payout/refund-fee-config/{get,set}` |
| `payout_links()`  | create · info/get · list · cancel · batch · cheque · claim_preview · claim                                                                                                                                | `POST /v1/payout/link` · `/payout/link/info` · `/payout/link/list` · `/payout/link/cancel` · `/payout/link/batch` · `/payout/link/cheque` · `GET /v1/claim/{token}` · `POST /v1/claim/{token}`                                                              |
| `payment_links()` | create · info/get · list · toggle · public_view · checkout                                                                                                                                                | `POST /v1/payment/link` · `/payment/link/info` · `/payment/link/list` · `/payment/link/toggle` · `GET /v1/link/{id}` · `POST /v1/link/{id}/checkout`                                                                                                        |
| `batches()`       | info                                                                                                                                                                                                     | `POST /v1/batch/info`                                                                                                                                                                                                                                     |
| `transfers()`     | to_personal · to_user · batch                                                                                                                                                                            | `POST /v1/transfer/to-personal` · `/transfer/to-user` · `/transfer/batch`                                                                                                                                                                                 |
| `wallets()`       | create · qr · block · refund_blocked_deposit                                                                                                                                                             | `POST /v1/wallet` · `/wallet/qr` · `/wallet/block` · `/wallet/blocked-address-refund`                                                                                                                                                                      |
| `webhooks()`      | register · rotate_secret · deliveries · test · test_legacy *(deprecated)*                                                                                                                                | `POST /v1/webhooks` · `/webhooks/rotate-secret` · `/webhooks/deliveries` · `/test-webhook/{payment,payout,wallet}` · `/payment/testing-webhook`                                                                                                             |
| `documents()`     | statement · ledger · balance_certificate · fee_schedule · split_report · batch_report · link_report · wallet_statement · referrals_report · create_job · job_info · job_file · download                    | `GET /v1/documents/statement` · `/documents/ledger` · `/documents/balance` · `/documents/fees` · `/documents/split` · `/documents/batch` · `/documents/link` · `/documents/wallet/statement` · `/documents/referrals` · `POST /v1/documents/jobs` · `/documents/jobs/info` · `GET /v1/documents/jobs/file` · `/documents/{kind}/{id}` |
| `splits()`        | create_rule · list_rules · delete_rule · get/set_config · get/set_opt_in                                                                                                                                 | `POST /v1/split/rule` · `/split/rule/list` · `/split/rule/delete` · `/split/config/{get,set}` · `/split/recipient/optin/get` · `/split/recipient/optin`                                                                                                     |
| `settings()`      | set_discount · list_discounts · get/set_accuracy · get/set_auto_refund · list/set_accepted · get/set_payment_fee_config · list/set/delete_auto_withdraw · list/add/remove/enable_api_allowlist             | `POST /v1/payment/discount/{set,list}` · `/payment/accuracy/{get,set}` · `/payment/autorefund/{get,set}` · `/payment/accepted/{list,set}` · `/payment/fee-config/{get,set}` · `/auto-withdraw/{list,set,delete}` · `/api-allowlist/{list,add,remove,enable}` |
| `account()`       | balance · referral · vrcs/set_vrcs *(the reference's `vrcs(enabled?)`, split in two because Rust has no optional arguments)*                                                                              | `POST /v1/balance` · `/referral/info` · `/vrcs`                                                                                                                                                                                                            |
| `catalog()`       | currencies · exchange_rates                                                                                                                                                                              | `GET /v1/currencies` · `POST /v1/exchange-rate/list`                                                                                                                                                                                                       |
| `sandbox()`       | faucet · deposit · webhooks · replay · reset                                                                                                                                                             | `POST /v1/sandbox/faucet` · `/sandbox/deposit` · `GET /v1/sandbox/webhooks` · `POST /v1/sandbox/webhooks/replay` · `/sandbox/reset`                                                                                                                         |
| `merchants()`     | create · create_sandbox (provisioning; `admin_token` on a self-hosted gateway)                                                                                                                            | `POST /v1/merchants` · `/merchants/{id}/sandbox`                                                                                                                                                                                                           |

Every method returns a builder that is also a future: `.await` it, or set per-call options first.

Lookups accept a bare uuid or a `Lookup`: `payments().info("uuid")`,
`payments().info(Lookup::order_id("order-1001"))`. Where the reference SDK accepts `string | model`,
the Rust methods take `impl Into<IdRef>`, so `payout_links().info(&link)` and
`payout_links().info("lnk_1")` both work.

`documents()` mostly answers with `FileResult { bytes, content_type, filename }`; the two job routes
are ordinary JSON — `create_job` and `job_info` return `DocumentJob`, and only `job_file` hands back
bytes.

### Lists

List methods return a `Pager`. Nothing is requested until you consume it.

```rust
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
```

### Statuses

- Payment: `select → created → confirm_check → paid | paid_over | wrong_amount | expired | cancelled`.
  `is_payment_paid(&status)` is true for `paid`/`paid_over`; `wrong_amount` (underpaid) waits for
  `refunds().resolve(…)`; `is_payment_final` covers the rest.
- Payout: `pending → approved → awaiting_cosign → broadcasting → sent → confirmed | failed | cancelled`,
  with `is_payout_final`.

Prefer webhooks for state changes; poll `info` only as a fallback. Every enum carries an
`Other(String)` variant, so a value a newer gateway introduces still decodes.

### Amounts

`add_amounts`, `subtract_amounts`, `compare_amounts`, `amounts_equal`, `is_zero_amount` — exact
decimal arithmetic on the string amounts the API uses. Never parse a `Money` as `f64`.

`Money` deliberately implements neither `Ord` nor `PartialOrd`: the derived versions compare the
decimal *strings*, so `"9.00" > "10.00"` would be true. `if amount > threshold` does not compile —
use `compare_amounts`. Derived `PartialEq` is exact string equality, so use `amounts_equal` when
trailing zeros may differ. Anything that is not a decimal string of at most 64 characters is an
`AmountError`; no input panics.

## Webhooks

Register an endpoint with `webhooks().register(url)` — the signing secret is returned once — and
verify every delivery over the **raw** bytes, before any parsing:

```rust
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
```

- **Rehearsals.** `webhooks().test()` and the sandbox sign deliveries exactly like live ones and set
  `test: true` (plus `X-Webhook-Test: true`). Check `delivery.is_test` (or
  `is_test_event(&delivery.event)` / `event.is_test()`) and never act on one as if money moved.
- **Duplicates and order.** `delivery.id` (`X-Webhook-Id`) is stable across retries — deduplicate on
  it. `event.sequence()` (an `Option<i64>`) orders events; `is_stale_event` drops an out-of-order
  one and is never true when the sequence is missing.
- **Rotation.** After `webhooks().rotate_secret()` pass `.previous_secret(old)` for at least 26
  hours, until `previous_secret_valid_until` has passed.
- **Unknown event types.** `WebhookEvent` is `#[non_exhaustive]` and has an `Other(Value)` arm: an
  event type newer than this snapshot arrives intact instead of failing, so match with a `_` arm.
  `is_known_event(&delivery.event)` (or `event.is_known()`) says whether this snapshot models the
  event's `type`; `event.raw()` hands back the body of one it does not.
- **Bad payload is not a bad signature.** A delivery whose signature verified but whose body cannot
  be read is `webhook.bad_payload` with `kind() == Contract` — deliberately *not* a signature
  failure. Answer **401 only for signature failures**, so a receiver that rejects forgeries does not
  401 an authentic event and earn 26 hours of retries.
- An empty secret or a negative tolerance is a `Config` error raised before any hashing;
  `tolerance_seconds(0)` disables the freshness check (the default is ±300 s).

The `oblodai::webhooks` module needs no client and no API key; it is available with
`--no-default-features`. Event types are `invoice.<status>`, `payout.<status>` and `wallet.paid`;
the body's `type` is `payment | payout | wallet`, and `WebhookEvent` is the matching enum.

## Errors

Every failure is an `oblodai::Error` carrying the API's error envelope: `code()`
(`payout.insufficient_funds`), `http_status()`, `retryable()`, `retry_after()`, `request_id()`,
`field()`, `synthetic()` (a proxy answered, not the API), and a `kind()` for matching:

| `kind()`                          | HTTP        | when                                                       |
| --------------------------------- | ----------- | ---------------------------------------------------------- |
| `Validation`                      | 400         | the request was rejected; `field()` names the offender      |
| `Authentication`                  | 401         | bad signature, missing or unknown key                       |
| `Permission`                      | 403         | the key may not do this (feature off, IP not allowlisted)   |
| `NotFound`                        | 404         | no such object                                              |
| `Conflict` / `IdempotencyConflict` | 409         | state conflict; a key reused with a different body          |
| `RateLimit`                       | 429         | throttled; honour `retry_after()`                           |
| `Unavailable`                     | 503         | gateway busy or in maintenance — retryable                  |
| `Internal`                        | other 5xx   | gateway fault                                               |
| `Api`                             | any other   | a status the gateway answered with that maps nowhere else   |
| `Transport`                       | —           | no response: timeout, connection, deadline                  |
| `Config`                          | —           | rejected before anything was sent                           |
| `Contract`                        | —           | the answer could not be read as an envelope                 |
| `Signature`                       | —           | webhook verification failed                                 |

`retryable()` is authoritative — the SDK already retried what it should; `retry_after()` says how
long to wait; quote `request_id()` to support. The raw body is never printed by `Debug` and never
serialized (`serde_json::to_value(&err)` keeps the message and drops the body).

The envelope is decoded field by field: one malformed field (a float `retry_after`, a numeric
`request_id`) never costs the `code` you branch on, and only a literal `true`/`false` overrides the
gateway's `retryable`.

Branch on the code, not on the message:

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

Codes worth handling: `payout.insufficient_funds` and `payout.funds_maturing` (both retryable),
`idempotency.key_reused`, `invoice.not_payable`, `payment.not_found`,
`merchant.bad_signature`, `request.rate_limited`. The full catalogue — **469 codes** — ships as
`oblodai::ERROR_CODES`, and every money-moving method lists the ones worth branching on in its own
rustdoc.

Codes the SDK raises itself, never the gateway: `sdk.missing_credentials`, `sdk.bad_config`,
`sdk.bad_header`, `sdk.bad_path_param`, `sdk.bad_idempotency_key`, `sdk.idempotency_unsupported`,
`sdk.bad_amount`, `sdk.bad_envelope`, `sdk.response_too_large`, `webhook.bad_payload`, and the
`transport.timeout` / `transport.network` / `transport.deadline` family.

## Retries, idempotency and timeouts

- **Safe to repeat** comes from the contract's own per-route `safe` flag, hand-classified by the
  gateway and exposed as `RouteSpec::safe`. There is no path-shape heuristic anywhere in the SDK.
- **Idempotency keys** are attached automatically on create-type routes — one per logical call,
  reused on every retry — so a timeout can never produce a second payout. Pass your own with
  `.idempotency_key(…)` to make retries safe across restarts as well; on routes the gateway does not
  deduplicate the SDK refuses a key with `sdk.idempotency_unsupported` before anything is sent.
- **When a retry happens.** An error is retried only when the API says `retryable: true`. Answers
  without an API envelope (a proxy 502/503) and transport failures are retried only on read routes
  or keyed writes. `Retry-After` is honoured (delta-seconds or HTTP-date, clamped — never negative,
  never overflowing), otherwise exponential backoff with jitter.
- **Knobs.** `ClientBuilder::retry(RetryOptions { max_retries, base_delay_ms, max_delay_ms,
  max_retry_after_ms })` — defaults 2 / 250 ms / 4 s / 30 s, and `max_retries: 0` disables retries.
- **Clock skew** is corrected once, from the server's `Date` header on a signature failure, and the
  correction is reverted if the re-signed attempt did not get past authentication. Offsets beyond
  ±24 h are treated as a broken proxy and ignored.
- **Redirects are never followed** — an answer from a different origin is reported, not accepted.
- **Response bodies are capped** at 8 MiB on envelope routes and 64 MiB on the `bare` document
  routes; anything larger is `sdk.response_too_large`.

Per-call options are set on the builder before `.await`:

```rust
client
    .payouts()
    .create(params)
    .idempotency_key("payout-42")
    .timeout(Duration::from_secs(10)) // one attempt
    .deadline(Duration::from_secs(45)) // the whole call, retries and pauses included
    .header("X-Request-Trace", "abc123") // this call only
    .await?;
```

Which options a builder has follows from what the route is:

| builder                                  | `.timeout` | `.deadline` | `.header` | `.idempotency_key`                                    |
| ---------------------------------------- | ---------- | ----------- | --------- | ----------------------------------------------------- |
| `RequestBuilder` (every ordinary route)  | ✓          | ✓           | ✓         | ✓                                                     |
| `FileBuilder` (`documents()`, `cheque`)  | ✓          | ✓           | ✓         | deprecated — no document route is deduplicated        |
| `Pager` (every list)                     | ✓          | ✓           | ✓         | — a key per page would make the gateway replay page 1 |

**Bound a call with `.deadline(…)`, not by dropping the future.** Dropping cancels the call, and the
auto-generated idempotency key lives in that future — a request already on the wire may still reach
the gateway, and re-issuing it would mint a *new* key the gateway cannot deduplicate against it. If
a retry has to survive a cancellation or a process restart, pass your own `.idempotency_key(…)`.

## Configuration

`Client::new(public_id, secret)`, `Client::from_env()` or `Client::builder()`. Every builder option
falls back to an environment variable:

| option                          | default                    | meaning                                                                |
| ------------------------------- | -------------------------- | ---------------------------------------------------------------------- |
| `.public_id(…)` / `.secret(…)`  | —                          | the API key (`X-Public-Id` plus the signing secret); give both or neither |
| `.base_url(…)`                  | `https://api.oblodai.com`  | API origin; a path prefix (`https://gw.corp/oblodai`) is kept           |
| `.timeout(…)`                   | 30 s                       | per attempt                                                            |
| `.deadline(…)`                  | 90 s                       | the whole call, retries and pauses included                            |
| `.retry(RetryOptions { … })`    | 2 retries                  | backoff policy; `max_retries: 0` disables retries                      |
| `.header(name, value)`          | —                          | an extra header on every request; signed headers are never overridden   |
| `.admin_token(…)`               | —                          | `X-Admin-Token`, sent on the `merchants()` routes and nowhere else      |
| `.allow_insecure_base_url(…)`   | `false`                    | permit a plain-`http` base URL beyond loopback                          |
| `.logger(…)`                    | none                       | a structured logger                                                    |
| `.http_backend(…)` / `.blocking_http_backend(…)` | `reqwest` | replace the HTTP layer (a proxy-aware client, a recording stub)         |
| `.clock(…)`                     | system clock               | the signing clock, for tests                                           |
| `.env(…)`                       | the process environment    | read the fallbacks from a map instead                                  |

These six variables are read, and no others:

| variable                    | option                        | meaning                                                            |
| --------------------------- | ----------------------------- | ------------------------------------------------------------------ |
| `OBLODAI_PUBLIC_ID`         | `.public_id(…)`               | API key, public half (`X-Public-Id`)                               |
| `OBLODAI_SECRET`            | `.secret(…)`                  | API key, secret half                                               |
| `OBLODAI_ADMIN_TOKEN`       | `.admin_token(…)`             | `X-Admin-Token`, sent on the `merchants()` routes and nowhere else |
| `OBLODAI_BASE_URL`          | `.base_url(…)`                | API origin; defaults to `https://api.oblodai.com`                  |
| `OBLODAI_LOG`               | `.logger(…)`                  | `debug\|info\|warn\|error` — installs a stderr logger              |
| `OBLODAI_ALLOW_INSECURE`    | `.allow_insecure_base_url(…)` | `1` permits a plain-http base URL beyond loopback                  |

### Self-hosted or local gateway

`base_url("http://localhost:8095")` works out of the box; other plain-http hosts need
`.allow_insecure_base_url(true)` (or `OBLODAI_ALLOW_INSECURE=1`). A path prefix in `base_url` is
kept and every route is appended to it.

### Secrets and logging

Secret-looking values are replaced with `[redacted]` before they reach *any* logger, including one
you supply. Response models that carry a one-time secret (`WebhookEndpoint.secret`,
`WebhookSecretRotated.secret`, `ApiKeyPair.secret`,
`PayoutLink.claim_token`/`claim_url`/`passcode`) redact them in `Debug` too, so
`tracing::info!(?response)` is safe; serialization still carries them, because you have to be able
to store what the gateway showed you once.

## The contract snapshot

`contract/` is exported by the gateway's own test suite: the route registry, request DTO schemas
with English field docs, enums, every error code, signing vectors, golden response bodies recorded
from a live gateway and real signed webhook deliveries. This snapshot: **107 merchant routes, 469
error codes**, exported from core `2cc44c16f516`. `src/contract/{routes,enums,requests,version}.rs`
are generated from it, and `oblodai::ROUTES`, `ERROR_CODES`, `NETWORKS`, `PAYMENT_STATUSES`,
`PAYOUT_STATUSES` and `EVENT_TYPES` expose it at runtime.

```sh
python3 scripts/codegen.py            # regenerate after refreshing contract/
python3 scripts/codegen.py --check    # CI gate: fail when the two disagree
```

`tests/contract_routes.rs` asserts every field of every route against `contract/contract.json`, so
generated code and snapshot cannot drift apart unnoticed.

## Development

```sh
git clone https://github.com/oblodai/oblodai-rust.git
cd oblodai-rust

cargo fmt --all --check
python3 scripts/codegen.py --check
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --all-targets --no-default-features -- -D warnings   # the feature matrix CI runs
cargo test --all-features                                         # unit + contract tests, no network
RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features

# against a real gateway
OBLODAI_LIVE_URL=http://127.0.0.1:8095 cargo test --all-features -- --ignored --test-threads=1
```

Every Rust block in this file is compiled by `tests/doc_snippets.rs`, which also asserts that
README.ru.md carries the same code blocks byte for byte.

See [AGENTS.md](AGENTS.md) for a condensed guide aimed at coding agents,
[CHANGELOG.md](CHANGELOG.md) for what changed, and [MIGRATION-1.3.md](MIGRATION-1.3.md) for the move
from 1.x.

## License

MIT — see [LICENSE](LICENSE).
