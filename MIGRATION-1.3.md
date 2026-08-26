# Migrating to 1.3

1.3 is a rewrite. The 1.x line signed requests with a four-field recipe the gateway no longer
accepts (every call has returned 401 since the five-field recipe shipped), and its models described
an earlier vocabulary. 1.3 is generated from the gateway's contract snapshot and verified against it.

## Signing (automatic)

Nothing to do — the SDK signs `ts \n METHOD \n path+query \n Idempotency-Key \n body`. If you
computed signatures yourself, use `oblodai::sign_request` instead.

## The client

| 1.x                                        | 1.3                                                                              |
| ------------------------------------------ | -------------------------------------------------------------------------------- |
| `Client::new(Config { .. })`               | `Client::builder().public_id(..).secret(..).build()?` / `Client::from_env()?`     |
| synchronous by default                     | async by default; `features = ["blocking"]` for `oblodai::blocking::Client`       |
| `client.payments.create(..)`               | `client.payments().create(..).await?`                                            |
| `Config { timeout, max_attempts, .. }`     | `.timeout(..)`, `.deadline(..)`, `.retry(RetryOptions { .. })`, or per call       |
| `Transport` trait over the whole call      | `HttpBackend` (async) / `BlockingHttpBackend` (sync) — one request, one answer    |
| `Error::Api { code, message }`             | one `Error` with `code()`, `http_status()`, `retryable()`, `retry_after()`, `request_id()`, `field()`, `synthetic()`, `kind()` |

Per-call options are set on the returned builder, not on the client:

```rust
client.payouts().create(params)
    .idempotency_key("payout-42")
    .timeout(Duration::from_secs(10))     // one attempt
    .deadline(Duration::from_secs(45))    // the whole call, retries and pauses included
    .header("X-Request-Trace", "abc123")  // this call only
    .await?;
```

`.timeout`, `.deadline` and `.header` are on all three builders (`RequestBuilder`, `FileBuilder`,
`Pager`). `.idempotency_key` is on `RequestBuilder` only — a `Pager` must not key its pages, and
neither `batches().info` nor any document route is deduplicated by the gateway.

## One API key

A merchant has one API key — public id `oblodai_<hex>` (sandbox `test_oblodai_<hex>`) and secret
`oblodai_live_<hex>` (sandbox `oblodai_test_<hex>`) — and it signs every route. The payout
credential pair is gone: `payout_public_id` / `payout_secret`, `OBLODAI_PAYOUT_PUBLIC_ID` /
`OBLODAI_PAYOUT_SECRET`, the per-call `prefer_payout_key` option and the payout-key fallback on
`batches().info` no longer exist. Drop them; keep `public_id` / `secret`, and `admin_token` if you
provision merchants on a self-hosted gateway. Onboarding responses now carry `api_key` only —
`payment_key` and `payout_key` are gone from `MerchantOnboarded` and `SandboxStore`, and
`ApiKeyPair` has no `kind` field. `merchant.wrong_key_kind` is no longer in the error catalogue; it
can still reach a merchant who kept a legacy `oblodai_pk_…`/`oblodai_wk_…` pair.

Bound a call with `.deadline(..)`, not with `tokio::time::timeout` or `select!`. Dropping the future
cancels the call, and the auto-generated idempotency key lives inside it: a request already on the
wire may still reach the gateway, and re-issuing it mints a new key that cannot be deduplicated
against the first. If a retry has to survive a cancellation or a restart, pass your own
`.idempotency_key(..)`.

## Renamed and reshaped

| 1.x                                              | 1.3                                                                                                                  |
| ------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------- |
| `payment.payment_status`                         | `payment.status` (`created`, `confirm_check`, `paid`, `paid_over`, `wrong_amount`, `expired`, `cancelled`, `select`) |
| payout statuses `check/process/paid/fail/cancel` | `pending/approved/awaiting_cosign/broadcasting/sent/confirmed/failed/cancelled`                                      |
| `paginate.count`                                 | `paginate.total`, plus `has_pages`                                                                                   |
| `webhooks.deliveries()` → `{deliveries}`         | `Pager<Tr, WebhookDelivery>` (`items` + `paginate`; `Tr` is the transport, inferred at the call site)                |
| `payout_links.list()` → `{links}`                | `Pager<Tr, PayoutLink>`                                                                                              |
| `calculate` → `to_amount`, `merchant_amount`     | `PayoutCalculation`: `amount`, `commission`, `payer_amount`, `fee_bearer`, `fee_type` (null when unpriceable)        |
| batch items `{status, error}` / `{success}`      | `BatchElement`: `{idx, ok, order_id, result, message, error_code, http_status}`                                      |
| payout link `expires_in_hours`                   | `expires_in_seconds`; new: `passcode`, `fee_bearer`, `title`, `note`                                                 |
| payment link `amount_min/amount_max/expires_in`  | `min_amount/max_amount/expires_in_seconds`                                                                           |
| split `refund_hold_hours`                        | `refund_hold_seconds`                                                                                                |
| auto-withdraw `min`                              | `min_amount`                                                                                                         |
| `resend` → `{result}`, replay → `{requeued}`     | `{ ok }`                                                                                                             |
| `payments().resolve` → payout only                | `Resolution` carries `resolution` (`accept` \| `refund`) — the branch the gateway actually took — plus the payout when it refunded |
| `client.rates.currencies()`                      | `client.catalog().currencies()` / `client.catalog().exchange_rates()`                                                |
| `client.links.*`                                 | `client.payment_links().*`                                                                                           |
| `client.idempotency.*`                           | `.idempotency_key(..)` on any call                                                                                   |
| `wallets.refund_blocked`                         | `wallets().refund_blocked_deposit`                                                                                   |
| `settings.get_api_allowlist`                     | `settings().list_api_allowlist`                                                                                      |
| `documents.get/balance/fees/batch/link/split/referrals` | `download` / `balance_certificate` / `fee_schedule` / `batch_report` / `link_report` / `split_report` / `referrals_report` |

## Types

- Amounts are `Money` — a newtype over `String`. Build one with `"25".into()`, read it with
  `.as_str()`, and do arithmetic with `oblodai::helpers::{add_amounts, subtract_amounts, …}`.
  Never parse it as `f64`.
- Statuses, networks and fee bearers are enums with an `Other(String)` variant, so a value a newer
  gateway introduces still decodes. Build one from a string: `network: Some("tron".into())`.
- Request bodies are generated structs with `Default`: set what you need and finish with
  `..Default::default()`.
- Nullable wire fields are `Option<T>`; fields the gateway may omit entirely are `Option<T>` too and
  are never sent when `None`.

## New

`payments().cancel`, `payouts().cancel`, `payouts().validate`, `payments().batch`,
`refunds().resolve`, `settings().*_fee_config`, `splits().set_opt_in`, `webhooks().rotate_secret`,
`payout_links().cheque`, the whole `documents()` namespace, payer-facing
`public_view`/`select`/`public_qr`/`checkout`/`claim`, `Pager` streaming, `verify_webhook` with
rotation support, `is_stale_event`, money helpers, and the `blocking` client.

## Webhooks

```rust
use oblodai::webhooks::{verify_webhook_delivery, Headers, VerifyOptions};

let delivery = verify_webhook_delivery(raw_body, &Headers::from_pairs(headers),
    &VerifyOptions::new(secret))?;
```

It replaces the previous verifier; it also accepts `.previous_secret(old)` during rotations and
rejects stale timestamps (±300 s) by default.

## What changed inside 1.3

Everything below landed after the first 1.3 draft; if you integrated against it, read this.

### Provisioning: the `merchants()` namespace and the admin token

`merchants().create(..)` and `merchants().create_sandbox(id)` provision a merchant on a self-hosted
gateway. They are `auth: onboard` — not HMAC-signed — and carry `X-Admin-Token` from
`.admin_token(..)` or `OBLODAI_ADMIN_TOKEN`. The token is sent on those routes **and nowhere else**,
and a caller header of the same name can no longer shadow or duplicate it.

### Rehearsal deliveries carry a test flag

`webhooks().test(..)` and the sandbox send deliveries signed exactly like live ones. They carry
`test: true` in the signed body (and `X-Webhook-Test: true`), surfaced as `delivery.is_test`,
`event.is_test()` and `is_test_event(&event)`. Never credit an order on one.

### Blocked static-wallet deposits

`wallets().block(..)` stops crediting an address; deposits that land afterwards wait for a decision,
and `wallets().refund_blocked_deposit(..)` sends them back. The codes worth branching on are `wallet.bad_uuid`, `refund.no_address`, `refund.nothing_to_refund`,
`refund.dust`, `refund.destination_internal` and `payout.insufficient_funds` (retryable). There is
no `wallet.blocked` error code — `blocked` is a field of the wallet model. The route is not
deduplicated by `Idempotency-Key`: it is idempotent by state, so a retry returns the same payout.

### `safe` comes from the contract

`RouteSpec::safe` — the flag that decides whether a request may be re-sent after a transport
failure — is now the gateway's own hand-classified value, read verbatim from
`contract/contract.json`. The path-suffix heuristic that used to derive it is gone, the codegen
fails if any route lacks the flag, and every field of every route is asserted against the contract
in `tests/contract_routes.rs`. Nothing about the retry policy changed; it is now *provably* the
gateway's policy rather than a guess that happened to agree.

### `webhook.bad_payload`, and an open event enum

A delivery whose signature verified but whose body cannot be read is now
`webhook.bad_payload` with `kind() == ErrorKind::Contract`. It used to be `webhook.bad_signature`,
which made an authentic-but-unreadable delivery indistinguishable from a forgery — so a receiver
answering 401 on signature failures answered 401 to a real event and earned ~26 hours of retries.

`WebhookEvent` is `#[non_exhaustive]` and gained `Other(serde_json::Value)`: an event `type` newer
than this snapshot is returned intact instead of failing. **Every `match` on `WebhookEvent` needs a
`_` arm.** `event.is_known()` / `is_known_event(&event)` tell the two apart, and `event_kind()`,
`uuid()`, `is_test()` and `is_stale_event` all keep working on an unknown event.

`event.sequence()` is now `Option<i64>` (it was `i64`): an event with no integer `sequence` is never
reported stale, because dropping a delivery you cannot order is worse than handling it twice.

### Secrets are redacted in `Debug`

`WebhookEndpoint.secret`, `WebhookSecretRotated.secret`, `ApiKeyPair.secret` and
`PayoutLink.claim_token` / `claim_url` / `passcode` print as `[redacted]`, so `tracing::info!(?resp)`
cannot leak them. **Serialization is unchanged** — `serde_json::to_string` still carries the real
value, because these are shown once and you have to store them. The same applies to logging: values
whose key looks sensitive are replaced before they reach any `Logger`, including one you supply.

### Model and API corrections

- `Money` no longer implements `Ord`/`PartialOrd`. `if a > b` on amounts stops compiling — use
  `helpers::compare_amounts` (and `amounts_equal` for equality that ignores trailing zeros).
- Every money helper refuses input longer than 64 characters with `AmountError` instead of panicking;
  `AmountError` converts into `Error` with code `sdk.bad_amount`.
- `documents().download(kind, id, SignedLinkQuery::new(exp, sig))` takes one query object instead of
  positional `exp`/`sig` arguments.
- `payment_links().info(link, PageParams)` and its alias `get` have identical signatures and both
  page the invoices the link spawned. `sandbox().webhooks(PageParams)` pages too.
- `payouts().cancel/approve`, `payout_links().info/get/cancel` and `payment_links().info/get/toggle`
  accept either a bare id or the object itself (`impl Into<IdRef>`).
- `FileBuilder::idempotency_key` is deprecated, because no `bare` route is deduplicated and setting
  one always failed the call.
- New error codes you may see, all raised before or instead of a gateway answer:
  `sdk.bad_header` (a caller header with CR/LF or a non-ASCII value), `sdk.response_too_large`
  (a body over 8 MiB on an envelope route or 64 MiB on a document route), `sdk.bad_amount`,
  `sdk.bad_idempotency_key`.
- `Cargo.toml`, `README.md`, `src/lib.rs` and `CHANGELOG.md` all state MSRV **1.86**, and CI checks
  it on exactly that toolchain. `cargo build --no-default-features` works, so the `HttpBackend` seam
  is usable without `reqwest`.
