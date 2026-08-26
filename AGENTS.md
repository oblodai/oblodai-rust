# Oblodai Rust SDK — guide for coding agents

Crate `oblodai` (1.3). Everything below is verified against the gateway's contract snapshot shipped
in `contract/contract.json`.

## Non-negotiables

- Amounts are decimal strings wrapped in `Money`: `amount: "25".into()`, never `25.0`. Do not parse
  them as `f64`; use `oblodai::helpers::{add_amounts, subtract_amounts, compare_amounts}`.
- Every method returns a builder that implements `IntoFuture`. `.await` it, or set per-call options
  first. `.timeout(..)` and `.deadline(..)` exist on all three builders (`RequestBuilder`,
  `FileBuilder`, `Pager`). `.idempotency_key(..)` exists only on `RequestBuilder` — a `Pager` must
  not key its pages, every document route is not deduplicated, and `FileBuilder::idempotency_key` is
  deprecated for exactly that reason. `.header(name, value)` is on all three too, for a header on
  this call only.
- Bound a call with `.deadline(..)`, never by dropping the future: the auto-generated idempotency key
  lives in the future and dies with it, so a re-issued call cannot be deduplicated against the one
  that may already be in flight. Supply `.idempotency_key(..)` when a retry must survive a restart.
- One API key. `public_id` + `secret` (or `OBLODAI_PUBLIC_ID` / `OBLODAI_SECRET`) sign every
  merchant route — money in and money out alike. There is nothing to choose per call. The admin
  token (`admin_token` / `OBLODAI_ADMIN_TOKEN`) goes only to the `merchants()` provisioning routes;
  public routes carry no credential at all. Only a merchant still holding a legacy split pair
  (`oblodai_pk_…` / `oblodai_wk_…`) can see a 403 `merchant.wrong_key_kind`.
- List methods return `Pager`: `.await` = one page (`Page { items, paginate }`), `.stream()` = every
  item as a `futures_core::Stream`, `.all(max)` = a `Vec`. Nothing is requested until consumed.
- Idempotency keys are generated automatically on create routes and reused across retries. Passing
  `.idempotency_key(..)` to a route the gateway does not deduplicate fails with
  `sdk.idempotency_unsupported` before anything is sent.
- Whether a failed request may be re-sent comes from `RouteSpec::safe`, taken verbatim from the
  contract's own per-route flag. No heuristic, no path-shape guess.
- Amounts: `Money` has no `Ord`/`PartialOrd` (string order is not numeric order) — use
  `helpers::compare_amounts` / `amounts_equal`. Every helper refuses input that is not a decimal
  string of at most 64 characters with `AmountError`; nothing panics.
- Request structs derive `Default`: fill the fields you need, finish with `..Default::default()`.

## Naming

| intent            | call                                                                                                            |
| ----------------- | ---------------------------------------------------------------------------------------------------------------- |
| fetch one         | `.info(uuid)` or `.info(Lookup::order_id(..))` (alias `.get`)                                                    |
| fetch many        | `.history(params)` on payments/payouts (alias `.list`), `.list(params)` elsewhere; `payment_links().info(link, PageParams)` pages the invoices a link spawned (alias `.get`, same signature) |
| create            | `.create(params)`; webhooks: `.register(url)`                                                                    |
| many, synchronous | `payouts().mass` (≤100), `payout_links().batch` (≤500) — per-element `{ idx, ok, result, message }`              |
| many, async       | `payments().batch`, `payouts().batch`, `refunds().batch`, `transfers().batch` — ≤5000, poll `batches().info`     |
| documents         | `documents().*` → `FileResult { bytes, content_type, filename }`, except `create_job`/`job_info` → `DocumentJob` |
| provisioning      | `merchants().create(..)`, `merchants().create_sandbox(id)` — no HMAC; `admin_token` on self-hosted gateways      |
| payer-facing      | `payments().public_view/select/public_qr`, `payment_links().public_view/checkout`, `payout_links().claim_preview/claim`, `documents().download` — no credentials |

## Errors

`Err(err)` → `oblodai::Error` with `code()` (`family.reason`), `http_status()`, `retryable()`
(authoritative — the SDK already retried what it should), `retry_after()`, `request_id()` (quote to
support), `field()` (400s), `synthetic()` (a proxy answered, not the API), and `kind()`:
`Validation` 400, `Authentication` 401, `Permission` 403, `NotFound` 404, `Conflict` /
`IdempotencyConflict` 409, `RateLimit` 429, `Unavailable` 503, `Internal` other 5xx, `Transport`
(no response), `Config` (before sending), `Contract` (unreadable envelope), `Signature` (webhooks),
`Api` (any other status).
`serde_json::to_value(&err)` keeps the message and drops the raw body.

Codes worth handling: `payout.insufficient_funds` (retryable), `payout.funds_maturing` (retryable),
`idempotency.key_reused`, `invoice.not_payable`, `payment.not_found`,
`merchant.bad_signature`, `request.rate_limited`. Full list: `oblodai::ERROR_CODES` (469 codes).
Every money-moving method lists its own codes in its rustdoc.

Codes the SDK raises itself, never the gateway: `sdk.missing_credentials`, `sdk.bad_config`,
`sdk.bad_header`, `sdk.bad_path_param`, `sdk.bad_idempotency_key`, `sdk.idempotency_unsupported`,
`sdk.bad_amount`, `sdk.bad_envelope`, `sdk.response_too_large`, `webhook.bad_payload`,
`transport.timeout` / `transport.network` / `transport.deadline`.

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
`is_stale_event(&delivery.event, last_sequence)` (`event.sequence()` is `Option<i64>`; a missing
sequence is never stale). During a rotation pass `.previous_secret(old)` for ≥26 h.

`WebhookEvent` is `#[non_exhaustive]` with an `Other(Value)` arm — an unknown event `type` is data,
not an error, so always match with a `_` arm; `is_known_event(&event)` / `event.is_known()` tell the
two apart and `event.raw()` hands back the body. An empty secret or a negative `tolerance_seconds` is a
`Config` error before any hashing; `0` disables the freshness check. A verified body that cannot be
read is `webhook.bad_payload` with `kind() == Contract`, never a signature failure.

## Machine-readable surface

`oblodai::ROUTES` (107 routes, 469 error codes: key, method, path, auth, idempotent, safe, bare, list — every field
equal to `contract/contract.json`, asserted per route in `tests/contract_routes.rs`),
`oblodai::contract::requests` (a struct per route body), `ERROR_CODES`, `NETWORKS`,
`PAYMENT_STATUSES`, `PAYOUT_STATUSES`, `EVENT_TYPES`, and `contract/` itself (schemas, golden
response bodies per route, error samples, signed webhook samples).

## Environment

Six variables, all optional: `OBLODAI_PUBLIC_ID`, `OBLODAI_SECRET`, `OBLODAI_BASE_URL`,
`OBLODAI_ADMIN_TOKEN`, `OBLODAI_ALLOW_INSECURE`, `OBLODAI_LOG`. `Client::from_env()` builds even with none of them set and fails on the first signed
call with `sdk.missing_credentials`.

Secrets never print: sensitive-looking log fields are `[redacted]` before they reach any logger,
including a caller-supplied one, and `WebhookEndpoint.secret`, `WebhookSecretRotated.secret`,
`ApiKeyPair.secret`, `PayoutLink.claim_token`/`claim_url`/`passcode` redact in `Debug`.
Serialization still carries them — that is how a merchant stores what was shown once.

MSRV 1.86. Features: `reqwest-client` (default), `blocking`. `--no-default-features` builds the
contract types, helpers, webhook verification and the `HttpBackend` seam with no `reqwest`.
