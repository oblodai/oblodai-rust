# Oblodai Rust SDK — guide for coding agents

Crate `oblodai` (2.0). Namespaces, methods, models and enums are generated from the gateway's
OpenAPI contract into `src/generated/` (never edit by hand; `make sdk` in the backend regenerates,
`make drift` here checks). Names are pinned in `names.lock`; 1.x → 2.0 names are in
`MIGRATION-2.0.md`.

## Non-negotiables

- Call shape: `client.<resource>().<method>(args…)` returns a builder; nothing is sent until it is
  `.await`ed (async `Client`) or `.send()` (`blocking::Client`). Resources: `payments`,
  `payment_links`, `refunds`, `payouts`, `payout_links`, `batches`, `splits`, `wallets`, `account`,
  `webhooks`, `settings`, `api_allowlist`, `referrals`, `documents`, `checkout`, `sandbox`.
- Bodies are typed models from `oblodai::models`: `PaymentRequest::new("25", "USDT")` sets the
  required fields, `PaymentRequest { network: Some("tron".into()), ..PaymentRequest::new(..) }`
  the optional ones. Query parameters of `GET` routes are one struct per method in
  `oblodai::generated::resources` (`GetStatementDocumentQuery`, …). Path ids are plain strings.
- Amounts are `Money` (a decimal string): `"25".into()`, never `25.0` — there is no `From<f64>`.
  A float arriving through `oblodai::from_json` is `sdk.float_amount` before anything is sent.
  Compare with `oblodai::helpers::{compare_amounts, amounts_equal}`; `Money` has no `Ord`.
- Call options on every builder: `.idempotency_key(k)`, `.timeout(Duration)` (one attempt),
  `.deadline(Duration)` (the whole call), `.max_retries(n)`, `.extra_header(n, v)`,
  `.request_id(id)` (`X-Request-ID`; a fresh UUID otherwise, the same on every attempt).
- Idempotency keys are generated automatically on the routes the gateway deduplicates
  (`RouteSpec::idempotent`) and reused across retries. A key on any other route is refused with
  `sdk.idempotency_unsupported` before sending — except `sandbox().faucet()`, whose body field
  `idempotency_key` takes the option (no header).
- Whether a failed call is re-sent comes from the contract (`RouteSpec::safe` — `GET` or
  `x-retry-safe` — or a keyed idempotent route). No heuristics.
- Bound a call with `.deadline(..)`, never by dropping the future (the auto key dies with it).
- Lists return `Pager`: `.await` = one page (`Page { items, paginate }`), `.stream()` = every
  item, `.by_page()` = page by page, `.all(max)` = a `Vec`; blocking: `.iter()`, `.by_page()`.
- Long-running operations (`batches().create_*`, `payouts().create_transfer_batch`,
  `documents().create_job`): `.job().await?` → `Job`; `job.wait()` polls to a terminal status
  (returned, not raised; `sdk.job_timeout` after 5 min), `job.download()` for document jobs.
- `.with_raw_response()` → `RawApiResponse` (`status()`, `headers()`, `request_id()`, `parse()`);
  `client.with_options(ClientOptions::new()…)` is a copy with other settings;
  `ClientBuilder::hooks(Hooks::new().on_request(..).on_response(..))` sees every attempt.
- One API key. `public_id` + `secret` (or `OBLODAI_PUBLIC_ID` / `OBLODAI_SECRET`) sign every
  merchant route. The admin token (`admin_token` / `OBLODAI_ADMIN_TOKEN`) goes only to
  `sandbox().onboard_store` (`X-Admin-Token`); `checkout()` routes carry no credential.

## Errors

`Err(err)` → `oblodai::Error`; `err.to_string()` is `[code] message (request_id=…)`. Methods:
`code()` (`family.reason`), `http_status()`, `retryable()` (authoritative — the SDK already retried
what it should), `retry_after()`, `request_id()` (quote to support), `field()` (400s),
`synthetic()` (a proxy answered, not the API), and `kind()`: `Validation` 400, `Authentication`
401, `Permission` 403, `NotFound` 404, `Conflict` / `IdempotencyConflict` 409, `RateLimit` 429,
`Unavailable` 503, `Internal` other 5xx, `Transport` (no response), `Config` (before sending),
`Contract` (unreadable answer), `Signature` (webhooks), `Api` (any other status).

Codes worth handling: `payout.insufficient_funds` (retryable), `payout.funds_maturing`
(retryable), `idempotency.key_reused`, `invoice.not_payable`, `payment.not_found`,
`merchant.bad_signature`, `request.rate_limited`. Every method's rustdoc lists its own codes.

Codes the SDK raises itself: `sdk.missing_credentials`, `sdk.bad_config`, `sdk.bad_header`,
`sdk.bad_path_param`, `sdk.bad_idempotency_key`, `sdk.idempotency_unsupported`,
`sdk.float_amount`, `sdk.bad_params`, `sdk.bad_amount`, `sdk.bad_envelope`,
`sdk.response_too_large`, `sdk.job_timeout`, `sdk.no_download`, `sdk.not_long_running`,
`webhook.bad_payload`, `transport.timeout` / `transport.network` / `transport.deadline`.

## Statuses

- Payment: `select → created → confirm_check → paid | paid_over | wrong_amount | expired | cancelled`
  (`under_review`). `is_payment_paid` = paid/paid_over. `wrong_amount` needs `payments().resolve(..)`.
- Payout: `pending → approved → awaiting_cosign → broadcasting → sent → confirmed | failed | cancelled`.
- Every enum has `Other(String)`: a value a newer gateway sends still decodes. Models keep unknown
  fields in `extra`.

## Webhooks

```rust
use oblodai::webhooks::{verify_webhook_delivery, Headers, VerifyOptions};
let delivery = verify_webhook_delivery(raw_body, &Headers::from_pairs(headers),
    &VerifyOptions::new(secret))?;
```

Verify over the **raw** bytes. `delivery.is_test` is true for rehearsal deliveries (`test: true` in
the signed body, or `X-Webhook-Test: true`) — never treat them as money. Deduplicate on
`delivery.event_id` (`X-Webhook-Event-Id`, stable across retries and resends), not `delivery.id`
(a resend gets a new one); drop out-of-order events with
`is_stale_event(&delivery.event, last_sequence)`. During a rotation pass `.previous_secret(old)`.
`WebhookEvent` is `Payment` / `Payout` / `Wallet` / `Conversion` (generated `*Webhook` models) or
`Other(Value)` — `#[non_exhaustive]`, match with a `_` arm. A verified body that cannot be read is
`webhook.bad_payload` with `kind() == Contract`, never a signature failure.

## Machine-readable surface

`oblodai::routes::ROUTES` and `routes::route(operation_id)` — every operation with `operation_id`,
method, path, auth, `idempotent`, `safe`, `bare` (answers with a file), `list_kind`.
`oblodai::lro::LRO` — which operations are long-running and how they are polled (generated from
`x-sdk-poll`). `oblodai::webhooks::KNOWN_EVENT_KINDS` / `WEBHOOK_EVENTS` — the webhook kinds and
event names (generated). `names.lock` —
the public method names.

## Environment

Six variables, all optional: `OBLODAI_PUBLIC_ID`, `OBLODAI_SECRET`, `OBLODAI_BASE_URL`,
`OBLODAI_ADMIN_TOKEN`, `OBLODAI_ALLOW_INSECURE`, `OBLODAI_LOG`. `Client::from_env()` builds even
with none of them set and fails on the first signed call with `sdk.missing_credentials`.

Secrets never print: sensitive-looking log fields are `[redacted]` before they reach any logger,
and `Debug` of every model hides the values of secret-looking fields (`secret`, `token`,
`passcode`, `claim_url`, …), in `extra` too. Serialization still carries them.

MSRV 1.86. Features: `reqwest-client` (default), `blocking`, `native-roots`.
`--no-default-features` builds the models, helpers, webhook verification and the `HttpBackend`
seam with no `reqwest`.
