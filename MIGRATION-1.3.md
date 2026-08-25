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
    .timeout(Duration::from_secs(10))
    .await?;
```

## Renamed and reshaped

| 1.x                                              | 1.3                                                                                                                  |
| ------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------- |
| `payment.payment_status`                         | `payment.status` (`created`, `confirm_check`, `paid`, `paid_over`, `wrong_amount`, `expired`, `cancelled`, `select`) |
| payout statuses `check/process/paid/fail/cancel` | `pending/approved/awaiting_cosign/broadcasting/sent/confirmed/failed/cancelled`                                      |
| `paginate.count`                                 | `paginate.total`, plus `has_pages`                                                                                   |
| `webhooks.deliveries()` → `{deliveries}`         | `Pager<WebhookDelivery>` (`items` + `paginate`)                                                                      |
| `payout_links.list()` → `{links}`                | `Pager<PayoutLink>`                                                                                                  |
| `calculate` → `to_amount`, `merchant_amount`     | `PayoutCalculation`: `amount`, `commission`, `payer_amount`, `fee_bearer`, `fee_type` (null when unpriceable)        |
| batch items `{status, error}` / `{success}`      | `BatchElement`: `{idx, ok, order_id, result, message, error_code, http_status}`                                      |
| payout link `expires_in_hours`                   | `expires_in_seconds`; new: `passcode`, `fee_bearer`, `title`, `note`                                                 |
| payment link `amount_min/amount_max/expires_in`  | `min_amount/max_amount/expires_in_seconds`                                                                           |
| split `refund_hold_hours`                        | `refund_hold_seconds`                                                                                                |
| auto-withdraw `min`                              | `min_amount`                                                                                                         |
| `resend` → `{result}`, replay → `{requeued}`     | `{ ok }`                                                                                                             |
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
