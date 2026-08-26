<div align="center">

# Oblodai Rust SDK

Official Rust client for the [Oblodai](https://oblodai.com) crypto payment gateway: invoices,
payouts, refunds, payout links, static wallets, webhooks and documents — the whole merchant API,
typed end to end and verified against the gateway's own contract snapshot.

[![crates.io](https://img.shields.io/crates/v/oblodai.svg)](https://crates.io/crates/oblodai)
[![docs.rs](https://img.shields.io/docsrs/oblodai)](https://docs.rs/oblodai)
[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

</div>

- Rust 2021, **MSRV 1.86** (checked in CI on exactly that toolchain; the floor comes from the dependency tree — `reqwest` → `url` → `idna`/`icu`). Async on `reqwest` + `tokio` (rustls, no OpenSSL); a `blocking` client behind a feature.
- Every route the gateway exposes has a method here; request bodies, enums and error codes are generated from the gateway.
- Retries driven by the API's own `retryable` flag, automatic idempotency keys, clock-skew correction.
- `oblodai::webhooks`: signature verification that needs no client and no API key.

```toml
[dependencies]
oblodai = "1.3"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
futures-util = "0.3"   # only for `.stream()` — the `StreamExt` adapters live there
```

Feature flags: `reqwest-client` (default, the async client), `blocking` (a synchronous client over
the same pure core), `native-roots` (also trust the OS certificate store — needed behind a
TLS-inspecting proxy or against a gateway with a private CA; without it the client trusts the
bundled webpki roots only). With `--no-default-features` you still get the contract types, the money
helpers, webhook verification and the `HttpBackend` seam, and no `reqwest` in the tree.

## Start in the sandbox

Get your keys in the Oblodai dashboard. A **sandbox key** (`pk_test_…` / `wk_test_…`) drives a chainless copy of the
gateway — fake balance from a faucet, simulated deposits, real webhooks — so integrate against it first.

```rust
use oblodai::{Client, contract::requests::PaymentRequest};

#[tokio::main]
async fn main() -> oblodai::Result<()> {
    // Reads OBLODAI_PUBLIC_ID / OBLODAI_SECRET (and the other OBLODAI_* below). It does not
    // require them: a client with no credentials builds fine and fails on the first signed call
    // with `sdk.missing_credentials`.
    let client = Client::from_env()?;

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
`payout_links` (the merchant side: `create`, `info`/`get`, `list`, `cancel`, `batch`, `cheque` —
the recipient-facing `claim_preview` and `claim` are public and need no key at all), `transfers`,
`splits`, `wallets().refund_blocked_deposit`, auto-withdraw, the IP allow-list,
`webhooks().rotate_secret`, `webhooks().test(WebhookKind::Payout, …)`, `sandbox().faucet`/`reset`.
Pass both pairs and the SDK picks the right one per call:

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
| `webhooks()`                    | register · rotate_secret · deliveries · test · test_legacy *(deprecated)*                                                                                                                    |
| `documents()`                   | statement · ledger · balance_certificate · fee_schedule · split_report · batch_report · link_report · wallet_statement · referrals_report · create_job · job_info · job_file · download      |
| `splits()`                      | create_rule · list_rules · delete_rule · get/set_config · get/set_opt_in                                                                                                                    |
| `settings()`                    | set_discount · list_discounts · get/set_accuracy · get/set_auto_refund · list_accepted · set_accepted · get/set_payment_fee_config · list/set/delete_auto_withdraw · list/add/remove/enable_api_allowlist |
| `account()` / `catalog()`       | balance · referral · vrcs/set_vrcs *(the reference's `vrcs(enabled?)`, split in two because Rust has no optional arguments)* · currencies · exchange_rates                                   |
| `sandbox()`                     | faucet · deposit · webhooks · replay · reset                                                                                                                                                |
| `merchants()`                   | create · create_sandbox (provisioning; `admin_token` on a self-hosted gateway)                                                                                                              |

Every method returns a builder that is also a future: `.await` it, or set per-call options first.
Which options a builder has follows from what the route is:

| builder                                  | `.timeout` | `.deadline` | `.prefer_payout_key` | `.header` | `.idempotency_key`                                    |
| ---------------------------------------- | ---------- | ----------- | -------------------- | --------- | ----------------------------------------------------- |
| `RequestBuilder` (every ordinary route)  | ✓          | ✓           | ✓                    | ✓         | ✓                                                     |
| `FileBuilder` (`documents()`, `cheque`)  | ✓          | ✓           | ✓                    | ✓         | deprecated — no document route is deduplicated        |
| `Pager` (every list)                     | ✓          | ✓           | ✓                    | ✓         | — a key per page would make the gateway replay page 1 |
| `BatchInfoCall` (`batches().info`)       | ✓          | ✓           | ✓                    | ✓         | — the route is not deduplicated                       |

Lookups accept a bare uuid or a `Lookup`: `payments().info("uuid")`,
`payments().info(Lookup::order_id("order-1001"))`. Where the reference SDK accepts `string | model`,
the Rust methods take `impl Into<IdRef>`, so `payout_links().info(&link)` and
`payout_links().info("lnk_1")` both work.

`documents()` mostly answers with `FileResult { bytes, content_type, filename }`; the two job
routes are ordinary JSON — `create_job` and `job_info` return `DocumentJob`, and only `job_file`
hands back bytes.

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
`Contract`, `Signature`, `Api` (any other status the gateway answered with). Codes the SDK raises
itself: `sdk.missing_credentials`, `sdk.bad_config`, `sdk.bad_header`, `sdk.bad_path_param`,
`sdk.bad_idempotency_key`, `sdk.idempotency_unsupported`, `sdk.bad_amount`, `sdk.bad_envelope`,
`sdk.response_too_large`, `webhook.bad_payload`, and the `transport.*` family. Quote `request_id()` to
support. The raw body is never printed by `Debug` and never serialized.

The envelope is decoded field by field: one malformed field (a float `retry_after`, a numeric
`request_id`) never costs the `code` you branch on, and only a literal `true`/`false` overrides the
gateway's `retryable`. Money-moving methods list the codes worth branching on in their own rustdoc.

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
  `Retry-After` is honoured (delta-seconds or HTTP-date, clamped — never negative, never overflowing).
- Whether re-sending is safe comes from the contract's own per-route `safe` flag, hand-classified by
  the gateway. There is no path-shape heuristic anywhere in the SDK.
- `ClientBuilder::retry(RetryOptions { max_retries, base_delay_ms, max_delay_ms, max_retry_after_ms })`;
  `.timeout(…)` per attempt, `.deadline(…)` per call.
- **Bound a call with `.deadline(…)`, not by dropping the future.** Dropping cancels the call, and
  the auto-generated idempotency key lives in that future — a request already on the wire may still
  reach the gateway, and re-issuing it would mint a *new* key the gateway cannot deduplicate against
  it. If a retry has to survive a cancellation or a process restart, pass your own
  `.idempotency_key(…)`.

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
`is_known_event(&delivery.event)` (or `event.is_known()`) says whether this snapshot models the
event's `type`; `event.raw()` hands back the body of one it does not.
`delivery.id` (`X-Webhook-Id`) is stable across retries — use it to
deduplicate; `event.sequence()` (an `Option<i64>`) orders events (`is_stale_event`, which is never
true when the sequence is missing). After `webhooks().rotate_secret()` pass `.previous_secret(old)`
for at least 26 hours. The module needs no client and no API key.

`WebhookEvent` is `#[non_exhaustive]` and has an `Other(Value)` arm: an event type newer than this
snapshot arrives intact instead of failing, so match with a `_` arm. An empty secret or a negative
tolerance is a `Config` error raised before any hashing; `tolerance_seconds(0)` disables the
freshness check. A delivery whose signature verified but whose body cannot be read is
`webhook.bad_payload` with `kind() == Contract` — deliberately *not* a signature failure, so a
receiver that answers 401 on forgeries does not 401 an authentic event and earn 26 hours of retries.

### Money helpers

`add_amounts`, `subtract_amounts`, `compare_amounts`, `amounts_equal`, `is_zero_amount` — exact
decimal arithmetic on the string amounts the API uses. Never parse a `Money` as `f64`.

`Money` deliberately implements neither `Ord` nor `PartialOrd`: the derived versions compare the
decimal *strings*, so `"9.00" > "10.00"` would be true. `if amount > threshold` does not compile —
use `compare_amounts`. Derived `PartialEq` is exact string equality, so use `amounts_equal` when
trailing zeros may differ. Anything that is not a decimal string of at most 64 characters is an
`AmountError`; no input panics.

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
Redirects are never followed — an answer from a different origin is reported, not accepted.

### Environment variables

Every builder option falls back to an `OBLODAI_*` variable. These eight are read, and no others:

| variable                    | option                        | meaning                                                        |
| --------------------------- | ----------------------------- | -------------------------------------------------------------- |
| `OBLODAI_PUBLIC_ID`         | `.public_id(…)`               | payment key, public half (`X-Public-Id`)                       |
| `OBLODAI_SECRET`            | `.secret(…)`                  | payment key, secret half                                       |
| `OBLODAI_PAYOUT_PUBLIC_ID`  | `.payout_public_id(…)`        | payout key, public half                                        |
| `OBLODAI_PAYOUT_SECRET`     | `.payout_secret(…)`           | payout key, secret half                                        |
| `OBLODAI_BASE_URL`          | `.base_url(…)`                | API origin; defaults to `https://api.oblodai.com`              |
| `OBLODAI_ADMIN_TOKEN`       | `.admin_token(…)`             | `X-Admin-Token`, sent on the `merchants()` routes and nowhere else |
| `OBLODAI_ALLOW_INSECURE`    | `.allow_insecure_base_url(…)` | `1` permits a plain-http base URL beyond loopback              |
| `OBLODAI_LOG`               | `.logger(…)`                  | `debug\|info\|warn\|error` — installs a stderr logger          |

Secret-looking values are replaced with `[redacted]` before they reach *any* logger, including one
you supply. Response models that carry a one-time secret (`WebhookEndpoint.secret`,
`WebhookSecretRotated.secret`, `ApiKeyPair.secret`, `PayoutLink.claim_token`/`claim_url`/`passcode`)
redact them in `Debug` too, so `tracing::info!(?response)` is safe; serialization still carries them,
because you have to be able to store what the gateway showed you once.

## The contract snapshot

`contract/` is exported by the gateway's own test suite: the route registry, request DTO schemas with
English field docs, enums, every error code, signing vectors, golden response bodies recorded from a
live gateway and real signed webhook deliveries. This snapshot: **107 merchant routes, 471 error
codes**, exported from core `7ec04293c426`. `src/contract/{routes,enums,requests,version}.rs`
are generated from it.

```sh
python3 scripts/codegen.py            # regenerate after refreshing contract/
python3 scripts/codegen.py --check    # CI gate: fail when the two disagree
```

## Development

```sh
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --all-targets --no-default-features -- -D warnings   # the feature matrix CI runs
cargo test --all-features             # unit + contract tests, no network
python3 scripts/codegen.py --check

# against a real gateway
OBLODAI_LIVE_URL=http://127.0.0.1:8095 cargo test --all-features -- --ignored --test-threads=1
```

License: MIT.
