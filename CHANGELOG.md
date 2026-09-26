# Changelog

All notable changes to this crate are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the versions follow
[Semantic Versioning](https://semver.org/).

## Unreleased

### Added

- `client.cli_login()` — `start`, `poll`, `logout_cli`: the browser login of the `oblodai` CLI
  (OAuth 2.0 device authorization) and logout of its key.
- `Error::details()`: the machine-readable facts of an error envelope's new `details` object (for
  example `cli.permission_denied` carries `required_role` and `role`); only string values are kept.
- Every method's documentation names the minimum team role a CLI key needs to call it.

## [2.0.0] — 2026-09-25

Generated from the gateway's OpenAPI contract. See MIGRATION-2.0.md for every renamed method.

### Added

- A float anywhere in a request body (in `extra` or a `serde_json::Value` field) outside
  `models::NON_MONEY_NUMBERS` is refused before the network with `sdk.float_amount`.

- Call options `.max_retries(n)` and `.request_id(id)`; every call sends `X-Request-ID` (a fresh
  UUID unless given), the same on every attempt.
- `.with_raw_response()` → `RawApiResponse` (status, headers, request id, `parse()`).
- `Client::with_options(ClientOptions)`, `ClientBuilder::max_retries`, `ClientBuilder::hooks`
  (`on_request` / `on_response` per attempt, the signature redacted).
- `Pager::by_page()` (async stream of pages; blocking iterator).
- Long-running operations: `.job()` on batches and document exports, `Job::wait()`,
  `Job::wait_with()`, `Job::download()` (`oblodai::lro`).
- `oblodai::from_json` builds a request model from JSON; a float amount is `sdk.float_amount`.
- `WebhookEvent::Conversion`.
- `WebhookDeliveryInfo::event_id` (`X-Webhook-Event-Id`, `HEADER_WEBHOOK_EVENT_ID`): the id of the
  state a delivery carries, the same across retries and resends — the key to deduplicate on
  (`id`, `X-Webhook-Id`, changes on a resend).
- `oblodai::generated::signing`: the signing protocol of the contract (`x-oblodai-signing`) —
  the header names of a signed request and of a webhook delivery, the order and separators of both
  canonical strings (`REQUEST_CANONICAL_ORDER`, `WEBHOOK_CANONICAL_ORDER` and their separators),
  `SIGNATURE_ALGORITHM`, `SKEW_SECONDS`, `MAX_BODY`, `MAX_IDEMPOTENCY_KEY_LENGTH`, and the rehearsal
  header `HEADER_WEBHOOK_TEST` (`webhook.test_header`). Signing, webhook verification and the
  idempotency-key check use only these; the 1.x names
  (`core::signing::HEADER_*`, `webhooks::HEADER_WEBHOOK_*`, `SIGNATURE_SKEW_SECONDS`,
  `DEFAULT_TOLERANCE_SECONDS`, `MAX_IDEMPOTENCY_KEY_LENGTH`) stay, now equal to the generated values.
  A header renamed in the gateway reaches the SDK with `make sdk`.
- Tests: the backend's shared conformance suite (`tests/conformance.rs`, signing and webhook vectors
  from the spec's `x-oblodai-signing`), the examples and every README block run against a fake
  gateway; `make ci` runs every gate, the drift check of `src/generated` included.

### Changed

- **Namespaces, methods, models and enums are generated** from `services/core/api/openapi.json` by
  the backend's `tools/sdkgen` into `src/generated/` — 120 operations, one method each, named
  `client.<resource>().<method>()` after the `operationId`. `names.lock` pins the names; a rename
  fails the generator as a breaking change. The hand-written resources, the `contract/` snapshot
  and `scripts/codegen.py` are gone.
- **Models** live in `oblodai::models`, enums in `oblodai::enums`; request models have
  `new(required…)`, fields a newer gateway adds land in `extra`, and every enum has
  `Other(String)`. `Debug` of a model hides the values of secret-looking fields, in `extra` too.
- **One builder, `Request<Tr, T>`,** for ordinary and document routes (`FileBuilder` and
  `RequestBuilder` are gone). `.header(..)` is `.extra_header(..)`, `.send_raw()` is
  `.send_json()`.
- **Errors print `[code] message (request_id=…)`**, and every error that reached the network
  carries the call's request id.
- `RouteSpec` is keyed by `operation_id` (`routes::route("createPayment")`), with `list_kind`.
- **The API facts the runtime used to list by hand are generated too:** the long-running
  operations (`lro::LRO`, `TERMINAL_STATUSES` and the `JobAck`/`JobStatus` implementations, from
  `x-sdk-poll`), the webhook kinds, event names and `WebhookEvent` (`webhooks::KNOWN_EVENT_KINDS`,
  `WEBHOOK_EVENTS`), the status classes (`PaymentStatus::FINAL`/`SUCCESS`, `is_final()`,
  `is_success()`, from `x-status-classes`; `helpers::FINAL_PAYMENT_STATUSES` is now a slice) and
  the non-money numbers of request bodies (`models::NON_MONEY_NUMBERS`). The method tables of
  README.md and README.ru.md and `names.lock` are written by the generator as well.

- **`WebhookEvent::object_id()`** reads the id field the contract declares for each event kind
  (`uuid`, or `id` on a conversion); `uuid()` is a deprecated alias of it. On an event type this
  version does not model it is empty — the id field is no longer guessed from `uuid`/`id` — as in
  the other SDKs; the raw body is in `raw()`.

### Removed

- Method aliases (`get` = `info`, `list` = `history`, …), `merchants().create` (not part of the
  merchant API contract), `Lookup`/`PaymentLookup`/`PayoutLookup`/`IdRef`/`PageParams` and the
  document query helpers, `ERROR_CODES` and the other constants of the contract snapshot.

## [1.3.0] — 2026-08-26

Rewrite generated from the gateway's contract snapshot (core `2cc44c16f516`, 107 merchant routes,
469 error codes). See MIGRATION-1.3.md.

### Removed

- **One API key; the payout credential pair and the payout-key option are gone.** The gateway signs
  every merchant route with the merchant's single API key, so `ClientBuilder::payout_public_id` /
  `payout_secret`, `OBLODAI_PAYOUT_PUBLIC_ID` / `OBLODAI_PAYOUT_SECRET`, the per-call
  `prefer_payout_key` on every builder and the payout-key fallback on `batches().info` (with its
  `BatchInfoCall` type — `batches().info` is now an ordinary `RequestBuilder`) are removed. Six
  environment variables remain: `OBLODAI_PUBLIC_ID`, `OBLODAI_SECRET`, `OBLODAI_ADMIN_TOKEN`,
  `OBLODAI_BASE_URL`, `OBLODAI_LOG`, `OBLODAI_ALLOW_INSECURE`. `RouteAuth` is now
  `Public` / `Key` / `Onboard`, onboarding returns `api_key` only (`payment_key`, `payout_key` and
  `ApiKeyPair::kind` are gone), and `merchant.wrong_key_kind` has left the error catalogue — it can
  still reach a merchant holding a legacy `oblodai_pk_…`/`oblodai_wk_…` pair.

### Security

- **Webhook verification checks the MAC before the freshness window.** The order used to be the
  other way round, so an unauthenticated caller could learn whether a timestamp was inside the
  tolerance without holding the secret. The window is now a post-authentication check.
- **An empty webhook secret is refused before any hashing** (`Config`, field `secret`) instead of
  computing an HMAC with the empty key, which would "verify" a delivery an attacker signed the same
  way. An empty `previous_secret` and a negative `tolerance_seconds` are refused likewise;
  `tolerance_seconds(0)` still means "no freshness check".
- **A verified-but-unreadable body is `webhook.bad_payload` with `kind() == Contract`**, not
  `webhook.bad_signature`. A receiver that answers 401 to forgeries no longer answers 401 to an
  authentic delivery and earn ~26 hours of gateway retries.
- **One-time secrets never print.** `WebhookEndpoint.secret`, `WebhookSecretRotated.secret`,
  `ApiKeyPair.secret` and `PayoutLink.claim_token` / `claim_url` / `passcode` have hand-written
  `Debug` impls that redact, so `tracing::info!(?response)` cannot leak a signing key or a cheque
  passcode. Serialization is unchanged — you have to be able to store what was shown once.
- **Redaction happens before the values reach any logger,** including one supplied through
  `ClientBuilder::logger`. It used to happen inside the built-in stderr logger only.
- **`Accept`, `User-Agent` and `X-Admin-Token` can no longer be duplicated by a caller header.**
  The HTTP backend appends headers, so a caller-supplied `Accept` put two `accept` lines on the
  wire, and a caller-supplied `X-Admin-Token` shadowed the configured one. Caller headers are now
  deduplicated case-insensitively and SDK-owned names win. `X-Admin-Token` is sent on the
  `merchants()` (onboard) routes and nowhere else.
- **A caller header with CR/LF or a non-ASCII value is a `Config` error** (`sdk.bad_header`) raised
  before the request is built, instead of being handed to the HTTP client.
- **Response bodies are read with a cap** — 8 MiB on envelope routes, 64 MiB on `bare` document
  routes — reported as a `Contract` error instead of buffering whatever the peer sends. A redirect
  followed by an injected HTTP client is detected (the answer did not come from the signed origin)
  and reported as "unexpected redirect".

### Fixed

- **The error envelope is decoded field by field.** One malformed field used to discard the whole
  envelope: a JSON float in `retry_after` turned `payout.funds_maturing` into `internal`, dropped
  `request_id`, and — because the fallback keyed off the status — flipped the gateway's
  authoritative `retryable: false` to `true` on a 503, so the SDK retried what the gateway said not
  to retry. Now `code` (non-empty string), `message`, `field`, `request_id` (strings), `retryable`
  (a literal boolean only) and `retry_after` (integer, float or numeric string) are each read on
  their own. An envelope with no usable `code` is reported as synthetic but still keeps a string
  `request_id`.
- **`retry_after` is clamped into `[0, 86400]`** at parse time, from the envelope and from the
  `Retry-After` header alike (delta-seconds or HTTP-date): a date in the past is `0`, a date in the
  year 9999 is capped, a 30-digit header is capped. The delay actually slept is still bounded by
  `RetryOptions::max_retry_after_ms`.
- **The money helpers no longer panic.** The common scale was passed to a format width, which is a
  `u16`, so a fraction of 65 536 digits aborted the process from inside a `#![forbid(unsafe_code)]`,
  `Result`-returning crate. Padding is explicit now, and any input longer than
  `helpers::MAX_AMOUNT_LEN` (64) is an `AmountError`.
- **`Money` no longer derives `Ord`/`PartialOrd`.** They compared the decimal *strings*, so
  `Money("9.00") > Money("10.00")` was `true` and sorting amounts produced `["10", "2", "9"]`.
  `if amount > threshold` is now a compile error; use `helpers::compare_amounts`.
- **Clock-skew correction is concurrency-safe.** A call now compares a measured offset against the
  offset *its own attempt was signed with*, and reverts a correction only if the shared offset is
  still the one it installed (a compare-and-exchange). Concurrent calls no longer roll each other's
  correction back.
- **`cargo build --no-default-features` compiles.** The async `Transport` used `tokio::time::sleep`
  while `tokio` was optional behind `reqwest-client`, so the documented "plug in your own
  `HttpBackend`" seam did not build without `reqwest`. `tokio` (feature `time`) is now a plain
  dependency, and CI has a feature matrix.
- **The MSRV is one number.** `Cargo.toml`, `README.md`, `src/lib.rs` and this file all say **1.86**
  (it said 1.75 here), and the `msrv` CI job sets `RUSTUP_TOOLCHAIN` so `rust-toolchain.toml` cannot
  silently make it check on the newest compiler; the job asserts the version it actually runs.
- **`Headers::insert` replaces rather than appends,** so setting the same header twice no longer
  leaves the first value shadowing the second.
- **The signature header is trimmed** and upper-case hex is accepted; a `0x` prefix is refused with
  a message that says so, instead of failing as a generic mismatch.
- **`documents().download` takes a query object** (`SignedLinkQuery { exp, sig, lang }`) rather than
  hoisting `exp` and `sig` into positional arguments, matching the reference SDK.

### Added

- **`safe` comes from the contract.** Every route carries the gateway's own hand-classified
  read-only flag, and the codegen fails if any route lacks it. The path-suffix regex and its dead
  exception list are gone. `tests/contract_routes.rs` now asserts `method`, `path`, `auth`,
  `idempotent`, `safe`, `bare` and `list` per route against `contract/contract.json`, with a test
  that proves a flipped flag is caught.
- **`WebhookEvent` is `#[non_exhaustive]` with an `Other(Value)` arm.** An event `type` newer than
  this snapshot arrives intact instead of failing verification; `event_kind()`, `uuid()`,
  `sequence()`, `is_test()` and `is_stale_event` all work on it. `sequence()` is now
  `Option<i64>`, and an event without one is never stale.
- **`#[must_use]` on `RequestBuilder`, `FileBuilder` and `Pager`** — a dropped builder sent nothing
  and warned about nothing.
- **`FileBuilder::idempotency_key` is deprecated**, since no `bare` route is deduplicated.
- **`PageParams`** and paging where it was missing: `payment_links().info(link, page)` and its alias
  `get` now have identical signatures and both page the invoices a link spawned, and
  `sandbox().webhooks(page)` pages like every other list.
- **`IdRef`** — `payouts().cancel/approve`, `payout_links().info/get/cancel` and
  `payment_links().info/get/toggle` accept either a bare id or the object itself, matching the
  reference's `string | model`.
- **Per-method error-code lists** in the rustdoc of every money-moving method
  (`payments().create`, `payouts().create/mass/batch/approve`, `refunds().create/resolve/batch`,
  `payout_links().create/cancel/batch/cheque/claim`, `payment_links().create/checkout`,
  `wallets().create/refund_blocked_deposit`, `transfers().*`, `splits().create_rule`), taken from the
  contract's error catalogue and gated by a test that refuses a code the catalogue does not
  contain.
- **`native-roots` feature** — also trust the OS certificate store. The default client trusts the
  bundled `webpki-roots` only, which silently fails behind a TLS-inspecting proxy or against a
  gateway with a private CA; the limitation is now documented and has a switch.
- **Per-call `.header(name, value)`** on all three builders, and per-call options are documented per
  builder rather than claimed uniformly.
- **`is_known_event(&event)` / `event.is_known()` / `event.raw()`** for an event type newer than
  this snapshot.
- `#[non_exhaustive]` on `Method`, `RouteAuth`, `ListKind`, `RouteSpec` and `WebhookDeliveryInfo` —
  types only the SDK constructs, so the gateway can grow without a major version here.
- New SDK error codes: `sdk.response_too_large` and `sdk.bad_header`. `AmountError` converts into
  `Error` with code `sdk.bad_amount`, and `MAX_RETRY_AFTER_SECONDS` is the shared 86 400 s bound.
- Regression tests for every fix above: `tests/unit_hardening.rs`, `tests/unit_wire.rs`,
  `tests/unit_backend.rs` (the socket-level size cap and redirect guard), and new cases in
  `tests/unit_webhooks.rs` and `tests/contract_routes.rs`.

### Changed

- Requests are signed with the five-field recipe over path+query
  (`ts \n METHOD \n path+query \n Idempotency-Key \n body`). The 1.x line signed four fields, which
  the gateway stopped accepting — every call returned 401.
- Models, statuses, pagination and parameter names match the current API vocabulary, and are checked
  field by field against golden response bodies recorded from a live gateway. The model round-trip
  gate now requires an exact count instead of a floor.
- Every merchant route (107) is present — cancel/validate, batches, documents, fee configs, split
  opt-in, secret rotation, payer-facing checkout and claim endpoints.
- `Pager` (await one page, `.stream()` every item, `.all(max)`), `retryable`-driven retries with a
  safe-to-repeat rule, automatic idempotency keys, a per-call deadline.
- `oblodai::webhooks` — rotation-aware `verify_webhook`, `verify_webhook_delivery`, `parse_webhook`,
  `is_stale_event`; no client and no API key needed.
- A `blocking` feature: the same method tree over the same pure core, synchronous I/O.
- `HttpBackend` / `BlockingHttpBackend` so the HTTP layer can be replaced.
- Async by default on `reqwest` + `tokio` with rustls (no OpenSSL). MSRV 1.86.
- Every method returns a builder that is also a future — per-call `idempotency_key`, `timeout` and
  `deadline` instead of client-wide settings.
- `Error` is one type with `code`, `http_status`, `retryable`, `retry_after`, `request_id`, `field`,
  `synthetic` and a `kind()`; the raw body is never printed or serialized.
- Documentation is English-only, and the claims in README/AGENTS were re-checked against the code:
  the per-call options are now stated per builder, all six `OBLODAI_*` variables are listed,
  `payout_links().batch` is ≤ 500 (not ≤ 100), `documents().create_job`/`job_info` return
  `DocumentJob` (not `FileResult`), and `payout_links().claim_preview`/`claim` are public routes
  that need no key.

## [1.2.0] — 2026-07-19

### Security

- **The base URL must be `https://`.** The SDK used to accept `http://` silently and send
  `X-Public-Id`, `X-Timestamp` and `X-Signature` in the clear — visible to any intermediary, and the
  signature was replayable. `Client::new` / `Client::with_transport` (and therefore
  `Client::from_env` and `OBLODAI_BASE_URL`) now return `Error::Config`. Loopback is the exception:
  `localhost`, anything in `127.0.0.0/8` and `::1` are accepted over `http://` so local stands keep
  working. Schemes other than `http`/`https`, and URLs with no scheme, are rejected too.

### Added

- **Configurable timeout:** `Config::timeout` (builder `Config::timeout(Duration)`, default 30 s,
  constant `oblodai::DEFAULT_TIMEOUT`). It bounds ONE HTTP attempt, not the whole call: with retries
  on, the worst case is `timeout × max_attempts` plus backoff.
- **`client.links()`** as an alias of `client.payment_links()`, so code ports between languages
  without renaming. Nothing removed or deprecated.
- **README: where to get keys** — issued in the Oblodai dashboard, the secret is shown once, and the
  sandbox gets its own test key.
- **README: the client is blocking** — stated up front, with a `tokio::task::spawn_blocking` recipe
  and a timeout section; the same warning in the `Client` rustdoc and the crate docs.
- **Typed statuses:** `PaymentStatus` and `PayoutStatus` with `from_api`/`as_str`/`is_final` (and
  `is_resolvable()` on payments). An unknown value decodes as `Unknown`, so a new gateway value does
  not break parsing. Model fields stay `String`; parse through `Payment::status()` and friends.
- **Developer sandbox:** `client.sandbox()` — `simulate_deposit`, `faucet`, `reset`,
  `list_webhooks`, `replay_webhook`. Test keys only (`test_…` / `oblodai_test_…`); a live key gets
  403 `sandbox.live_key`. Business methods work unchanged — only the key differs.
- **Signed GET:** `GET /v1/sandbox/webhooks` is signed with the same canonical string as a POST,
  with an empty body: `{ts}\nGET\n{path}\n`.
- **`oblodai::is_test_key(public_id)`.**
- **User transfers:** `account().transfer_to_user(params)` — fee-free internal transfer to a platform
  user's personal wallet (`to_user_id` is the user's **UUID, not a username**). Idempotent like every
  other money method.
- **Transfer batches:** `account().transfer_batch(transfers, on_error)` (≤ 5000) → `batch_id`;
  results through `batches().info(...)`.
- **Public pay endpoints for your own checkout:** `payments().public_get(uuid)` and
  `payments().public_select(uuid, currency, network)` — unsigned, like `/v1/link/{id}`.

### Fixed

- **Documentation blocker: webhooks were being verified with the API key secret.** The README and the
  parameters of `verify_webhook` / `construct_event` / `compute_webhook_signature` said "secret",
  which everywhere else in the README meant the API key's secret. Webhooks are verified with the
  **endpoint secret** from `webhooks().register(url)` — a different value; using the API key rejects
  100 % of deliveries. Parameters renamed to `endpoint_secret`, with a "there are two secrets" table.
- **`register()` is an upsert of the project's SINGLE endpoint,** not "add another receiver". Calling
  it again with a different URL returns the same `endpoint_id` and redirects deliveries; the old URL
  goes quiet. The secret is preserved (already-queued deliveries are signed with it).
- **The status vocabulary is documented in full,** including `confirm_check`, `wrong_amount_waiting`,
  `paid_over` and `select`. `wrong_amount_waiting` (partial payment, invoice still alive) is
  separated from `wrong_amount` (closed underpaid); `payments().resolve(...)` only accepts the
  latter — the gateway answers `409 resolution.not_underpaid` for the former.
- **Sandbox: a "pending" invoice was described as `check`.** Too few confirmations is
  `confirm_check`, or `wrong_amount_waiting` when the amount is short; `check` means no payment was
  seen at all.
- **`sandbox().reset()` is not a clean slate.** Balances are zeroed, but only invoices in `check`
  (internally `created`) and `select` are cancelled. An invoice with a deposit already seen is left
  alone on purpose — cancelling it would let the deposit mature into a cancelled invoice.
- **`claim_url` and `payment.url` can arrive empty.** The gateway builds them from its own public
  base URL; production will not start without one, but a local stand leaves the field empty. Build
  the link yourself from `claim_token` / `uuid`.
- **A webhook body never carries `wrong_amount_waiting`:** webhooks send the unrefined status, so a
  partial payment arrives as `confirm_check`. Only `payments().info(...)` returns the refined value.
- **`PaymentLinkPayment::status` carries the invoice's INTERNAL literals** (`created`, `expired`,
  `cancelled`), not the `payment_status` vocabulary — documented so it is not compared with
  `check`/`cancel`.
- **Duplicate payout links on an automatic retry.** `payout_links().create` and `create_batch`
  reserve balance but were sent WITHOUT an `Idempotency-Key`: a lost response led to a retry, and the
  retry to a second funded link and a second reservation. Both now take the idempotent path — the key
  is computed once, before any retry. A partially failed batch replays as-is: failed elements are not
  re-sent under the same key, send them in a new call. The gateway deduplicates these routes and
  replays the first answer (same link, same `claim_token`, `Idempotent-Replayed: true`); the balance
  is debited exactly once.
- **The idempotency layer's codes are documented** on `/v1/payout/link` and `/v1/payout/link/batch`:
  `400 idempotency.key_reused`, `400 idempotency.bad_key`, `409 idempotency.in_progress` and
  `503 idempotency.unavailable` (the store is down and the gateway fails closed — the operation is
  NOT performed). Retry classification is unchanged: the SDK replays a 503 with the SAME key; 400 and
  409 are terminal.
- **A duplicate payout-link `reference` is now `409 payoutlink.duplicate_reference`** (it used to be
  a 500, which the SDK retried as transient). The conflict is returned to the caller instead.
- **The 256 KB idempotency-cache limit is documented:** a response larger than that is not cached, so
  a retry with the same key executes again. Reachable on large payout-link batches — set a per-item
  `reference`, whose unique index is a second, durable layer of protection.
- **Sandbox docs: removed the false "a deposit matures on its own in ~10 minutes".** Nobody re-emits
  a simulated deposit, so an invoice short of confirmations never becomes `paid` — repeat
  `simulate_deposit` with the SAME `txid` and a higher `confirmations`. The ~10 minutes belong to a
  different mechanism: the maturity hold on a PAYOUT (`payout.funds_maturing`).
- **`payout.funds_maturing` described precisely:** it is terminal, and the SDK does not retry it. The
  vague word "temporary" read as the opposite.
- **Documented why `wallets().blocked_address_refund` and `payouts().approve` send no
  `Idempotency-Key`:** the first is idempotent by state (the payout reference is derived from the
  wallet id under a per-wallet lock, so a retry returns the same payout), the second is a status
  transition (only a pending payout can be approved, otherwise `409 payout.not_pending`). Wrapping
  them in the idempotency middleware would be a regression: a concurrent retry would get
  `409 idempotency.in_progress` instead of waiting and succeeding. Caveat: the address is not part of
  the payout reference, so a retry with a DIFFERENT address returns the first payout to the FIRST
  address.

## [1.1.0] — 2026-07-15

### Changed (BREAKING): idempotency

- **The automatic `order_id` is gone.** The SDK no longer writes `idem-<hex>` into
  `payments().create(...)` or `account().transfer_to_personal(...)` — `order_id` is sent exactly as
  given. Set it explicitly if you relied on the generated one.
- **Duplicate protection is the `Idempotency-Key` header now.** Every creating call
  (`/v1/payment`, `/v1/payment/refund`, `/v1/payment/resolve`, `/v1/payment/batch`,
  `/v1/refund/batch`, `/v1/payout`, `/v1/payout/mass`, `/v1/payout/batch`,
  `/v1/transfer/to-personal`) sends a UUID v4 generated once BEFORE any retry, so every internal
  retry of a call carries one key. The header is not part of the request signature.
- **Your own key:** the `idempotency_key` field of a creating call's parameters goes into the header
  and not into the body.
- Exception: `/v1/payout/link*` did not support the header, so the SDK did not send it there;
  deduplication went through a per-link `reference`. *(Superseded in 1.2.0: the gateway deduplicates
  those routes and the SDK sends the header.)*

### Added

- **Bulk operations (up to 5000 items):** `payments().create_batch(...)`,
  `payments().refund_batch(...)`, `payouts().create_batch(...)` and `batches().info(...)`.
- **Payment links:** `payment_links().create/list/info/toggle`, plus unsigned `public_get` and
  `checkout`.
- **Split payments:** `splits().create_rule/list_rules/delete_rule/get_config/set_config`.
- **Invoice by e-mail:** `payments().send_email(uuid, order_id, email)`.
- **Payout links ("crypto cheques"):** `payout_links().create/create_batch (≤500)/list/info/cancel`
  and the unsigned `claim_info(token)` / `claim(token, address, memo)`. Set `expires_in_hours`
  explicitly — the gateway clamps 0/absent to one hour.
- **Underpayment resolution:** `payments().resolve(ResolveAction::Accept | Refund, params)`.

## [1.0.2] — 2026-07-12

### Fixed (robustness)

- **The "missing `order_id`" check was normalized.** The automatic idempotency key now also fires on
  a whitespace-only string (`"   "`), and a non-string `order_id` (a number, a boolean) no longer
  passes through as-is. A real, non-empty `order_id` is left untouched.

## [1.0.1] — 2026-07-12

### Fixed (money safety)

- **Automatic idempotency key.** `payments().create(...)` and
  `account().transfer_to_personal(...)` fill in a stable `order_id` (`idem-<hex>`) when none is
  given. A retry of a non-idempotent POST after a timeout or a 503 used to be re-signed and could
  create a duplicate (the backend deduplicates by `order_id`).
- **`Retry-After` is no longer truncated to `max_delay`.** The server's advice (e.g.
  `Retry-After: 60`) is honoured as given, with an absolute ceiling of 300 s.
- **Real retry jitter** instead of a fixed fraction (`0.25 * initial_delay`): a random addition in
  `[0, initial_delay)` on top of the OS RNG (`getrandom`).
- **`payout.funds_maturing` became a terminal error** — an immediate retry cannot help while the
  funds mature.

## [1.0.0] — 2026-07-12

### Added

- First release of the official Rust SDK for the Oblodai payment gateway.
- Accepting payments, payouts and mass payouts, static wallets, refunds, webhooks, and the public
  reference data (exchange rates, the catalogue of assets and networks).
- HMAC-SHA256 request signing and webhook signature verification (constant-time comparison, replay
  protection).
- `Client::from_env()` — `OBLODAI_PUBLIC_ID` / `OBLODAI_SECRET` / `OBLODAI_BASE_URL`.
- Automatic retries with exponential backoff, honouring `Retry-After` on 429.
