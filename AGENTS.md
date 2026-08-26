# Oblodai Rust SDK — guide for coding agents

Crate `oblodai` (1.3). Everything below is verified against the gateway's contract snapshot shipped
in `contract/contract.json`.

## Non-negotiables

- Amounts are decimal strings wrapped in `Money`: `amount: "25".into()`, never `25.0`. Do not parse
  them as `f64`; use `oblodai::helpers::{add_amounts, subtract_amounts, compare_amounts}`.
- Every method returns a builder that implements `IntoFuture`. `.await` it, or set per-call options
  first: `.idempotency_key(..)`, `.timeout(..)`, `.deadline(..)`, `.prefer_payout_key(..)`.
- Two key kinds. The **payout key** is required for: `payouts()`, `refunds()`, `payout_links()`,
  `transfers()`, `splits()`, `wallets().refund_blocked_deposit`, `settings().*_auto_withdraw`,
  `settings().*_api_allowlist`, `webhooks().rotate_secret`, `webhooks().test(Payout, …)`,
  `sandbox().faucet`, `sandbox().reset`. Configure it with `payout_public_id` / `payout_secret` (or
  `OBLODAI_PAYOUT_*`); a wrong kind is a 403 `merchant.wrong_key_kind`.
- List methods return `Pager`: `.await` = one page (`Page { items, paginate }`), `.stream()` = every
  item as a `futures_core::Stream`, `.all(max)` = a `Vec`. Nothing is requested until consumed.
- Idempotency keys are generated automatically on create routes and reused across retries. Passing
  `.idempotency_key(..)` to a route the gateway does not deduplicate fails with
  `sdk.idempotency_unsupported` before anything is sent.
- Request structs derive `Default`: fill the fields you need, finish with `..Default::default()`.

## Naming

| intent            | call                                                                                                            |
| ----------------- | ---------------------------------------------------------------------------------------------------------------- |
| fetch one         | `.info(uuid)` or `.info(Lookup::order_id(..))` (alias `.get`)                                                    |
| fetch many        | `.history(params)` on payments/payouts (alias `.list`), `.list(params)` elsewhere                                |
| create            | `.create(params)`; webhooks: `.register(url)`                                                                    |
| many, synchronous | `payouts().mass`, `payout_links().batch` — ≤100, per-element `{ idx, ok, result, message }`                      |
| many, async       | `payments().batch`, `payouts().batch`, `refunds().batch`, `transfers().batch` — ≤5000, poll `batches().info`     |
| documents         | `documents().*` → `FileResult { bytes, content_type, filename }`                                                 |
| provisioning      | `merchants().create(..)`, `merchants().create_sandbox(id)` — no HMAC; `admin_token` on self-hosted gateways      |
| payer-facing      | `payments().public_view/select/public_qr`, `payment_links().public_view/checkout`, `payout_links().claim_preview/claim` — no credentials |

## Errors

`Err(err)` → `oblodai::Error` with `code()` (`family.reason`), `http_status()`, `retryable()`
(authoritative — the SDK already retried what it should), `retry_after()`, `request_id()` (quote to
support), `field()` (400s), `synthetic()` (a proxy answered, not the API), and `kind()`:
`Validation` 400, `Authentication` 401, `Permission` 403, `NotFound` 404, `Conflict` /
`IdempotencyConflict` 409, `RateLimit` 429, `Unavailable` 503, `Internal` other 5xx, `Transport`
(no response), `Config` (before sending), `Contract` (unreadable envelope), `Signature` (webhooks).
`serde_json::to_value(&err)` keeps the message and drops the raw body.

Codes worth handling: `payout.insufficient_funds` (retryable), `payout.funds_maturing` (retryable),
`idempotency.key_reused`, `invoice.not_payable`, `payment.not_found`, `merchant.wrong_key_kind`,
`merchant.bad_signature`, `request.rate_limited`. Full list: `oblodai::ERROR_CODES`.

## Statuses

- Payment: `select → created → confirm_check → paid | paid_over | wrong_amount | expired | cancelled`.
  `is_payment_paid` = paid/paid_over. `wrong_amount` needs `refunds().resolve(..)`.
- Payout: `pending → approved → awaiting_cosign → broadcasting → sent → confirmed | failed | cancelled`.
- Webhook event types: `invoice.<status>`, `payout.<status>`, `wallet.paid`; the body's `type` is
  `payment | payout | wallet` and `WebhookEvent` is the matching enum.

## Webhooks

```rust
use oblodai::webhooks::{verify_webhook_delivery, Headers, VerifyOptions};
let delivery = verify_webhook_delivery(raw_body, &Headers::from_pairs(headers),
    &VerifyOptions::new(secret))?;
```

Verify over the **raw** bytes. `delivery.is_test` is true for rehearsal deliveries (`test: true` in
the signed body, or `X-Webhook-Test: true`) — never treat them as money. Deduplicate on
`delivery.id` (`X-Webhook-Id`); drop out-of-order events with
`is_stale_event(&delivery.event, last_sequence)`. During a rotation pass `.previous_secret(old)`
for ≥26 h.

## Machine-readable surface

`oblodai::ROUTES` (107 routes: key, method, path, auth, idempotent, safe, bare, list),
`oblodai::contract::requests` (a struct per route body), `ERROR_CODES`, `NETWORKS`,
`PAYMENT_STATUSES`, `PAYOUT_STATUSES`, `EVENT_TYPES`, and `contract/` itself (schemas, golden
response bodies per route, error samples, signed webhook samples).
