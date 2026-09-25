# Migrating from 1.x to 2.0

2.0 is generated from the gateway's OpenAPI contract (`services/core/api/openapi.json`) instead of
the older contract snapshot in `contract/`, and every public method name follows one rule:
`client.<resource>().<method>(…)`, where the resource comes from the operation's tag and the method
is its `operationId` without the resource name, in snake_case. `names.lock` pins the result; the
table below is built from it. The wire does not change — only the Rust surface does.

## At a glance

- **Rust 1.86** or newer, as in 1.x.
- **Types moved.** `oblodai::contract::requests::X` and `oblodai::contract::models::X` are
  `oblodai::models::X` now, `oblodai::PaymentStatus` and the other enums are in `oblodai::enums`,
  and many types carry the contract's names: `PaymentHistoryRequest`/`PayoutHistoryRequest` →
  `HistoryRequest`, `Payment` → `PaymentView` (and `PaymentInfoResult` for `get_info`),
  `SandboxDepositRequest` → `SimulateDepositRequest`, `SandboxFaucetRequest` → `FaucetRequest`,
  `WalletRequest` → `CreateWalletRequest`, `PaymentEvent`/`PayoutEvent`/`WalletEvent` →
  `PaymentWebhook`/`PayoutWebhook`/`WalletWebhook` (plus the new `ConversionWebhook`).
- **`WebhookEvent::uuid()` → `object_id()`.** The old name still compiles, deprecated. On an event
  type this version does not model it is empty now instead of a guess; read `raw()` there.
- **Bodies are models with a constructor.** `Model::new(required…)` sets the required fields; the
  optional ones are `Option`s: `PaymentRequest { network: Some("tron".into()), ..PaymentRequest::new("25", "USDT") }`.
  `Default::default()` still works for the rest. Fields a newer gateway adds land in `extra`.
- **Lookups are the request models.** `payments().info("uuid")` / `Lookup::order_id(..)` /
  `PaymentLookup` become `payments().get_info(LookupRequest { uuid: Some(..), ..Default::default() })`;
  `payouts().approve("p1")` becomes `payouts().approve(ApproveRequest::new("p1"))`, and so on.
- **Query parameters** of the `GET` routes are one typed struct per method in
  `oblodai::generated::resources` (`GetStatementDocumentQuery`, `SandboxListWebhooksQuery`, …) in
  place of `PageParams`, `DocumentQuery`, `FormatQuery`, `PeriodQuery` and `SignedLinkQuery`.
- **Builders.** `RequestBuilder` and `FileBuilder` are one `Request<Tr, T>` (a document route is a
  `Request` of `FileResult`); `Pager` stays. Per-call options:

  | 1.x | 2.0 |
  | --- | --- |
  | `.header(name, value)` | `.extra_header(name, value)` |
  | `.timeout(Duration)` / `.deadline(Duration)` | unchanged |
  | `.idempotency_key(k)` | unchanged; on `sandbox().faucet()` it fills the body field `idempotency_key` |
  | `.send_raw()` (the `result` as JSON) | `.send_json()` |
  | — | `.max_retries(n)`, `.request_id(id)` (`X-Request-ID`), `.with_raw_response()` |
  | — | `.job()` on batches and document exports, then `job.wait()` / `job.download()` |
  | `Pager` `.await` / `.stream()` / `.all(max)` | unchanged, plus `.by_page()` |

- **Client.** New: `ClientBuilder::max_retries(n)`, `ClientBuilder::hooks(Hooks)`,
  `client.with_options(ClientOptions)`. `client.transport()` returns the transport by value.
- **Errors print** `[code] message (request_id=…)` (1.x printed the message alone), and every
  error that got as far as the network carries the call's `X-Request-ID`.
- **Routes** are keyed by `operationId`: `oblodai::routes::route("createPayout")`, and
  `RouteSpec` has `operation_id` and `list_kind` in place of `key` and `list`.
- **Removed:** the aliases (`payments().get` = `info`, `payments().list` = `history`, …),
  `merchants().create` (`POST /v1/merchants` is not part of the merchant API contract; the store
  provisioning route is `sandbox().onboard_store`), `ERROR_CODES` and the other snapshot constants
  of `contract/`. `webhooks().test(kind, ..)` is one method per kind now (see the table);
  `account().vrcs()` and `set_vrcs(..)` are one `settings().configure_vrcs(..)`.

## Method names (120 methods)

| 1.x | 2.0 | operationId | route |
| --- | --- | --- | --- |
| `account().balance()` | `account().get_balance()` | `getBalance` | `POST /v1/balance` |
| — (new) | `account().get_summary()` | `getSummary` | `POST /v1/summary` |
| `catalog().exchange_rates()` | `account().list_exchange_rates()` | `listExchangeRates` | `POST /v1/exchange-rate/list` |
| `settings().add_api_allowlist()` | `api_allowlist().add_entry()` | `addApiAllowlistEntry` | `POST /v1/api-allowlist/add` |
| `settings().list_api_allowlist()` | `api_allowlist().list()` | `listApiAllowlist` | `POST /v1/api-allowlist/list` |
| `settings().remove_api_allowlist()` | `api_allowlist().remove_entry()` | `removeApiAllowlistEntry` | `POST /v1/api-allowlist/remove` |
| `settings().enable_api_allowlist()` | `api_allowlist().set_enabled()` | `setApiAllowlistEnabled` | `POST /v1/api-allowlist/enable` |
| `payments().batch()` | `batches().create_payment()` | `createPaymentBatch` | `POST /v1/payment/batch` |
| `payouts().batch()` | `batches().create_payout()` | `createPayoutBatch` | `POST /v1/payout/batch` |
| `refunds().batch()` | `batches().create_refund()` | `createRefundBatch` | `POST /v1/refund/batch` |
| `batches().info()` | `batches().get_info()` | `getBatchInfo` | `POST /v1/batch/info` |
| `payments().public_view()` | `checkout().get()` | `getCheckout` | `GET /v1/pay/{id}` |
| — (new) | `checkout().get_onramp()` | `getCheckoutOnramp` | `GET /v1/pay/{id}/onramp` |
| `payment_links().public_view()` | `checkout().get_public_payment_link()` | `getPublicPaymentLink` | `GET /v1/link/{id}` |
| `payments().public_qr()` | `checkout().get_qr()` | `getCheckoutQr` | `GET /v1/pay/{id}/qr` |
| — (new) | `checkout().get_source_of_funds_form()` | `getSourceOfFundsForm` | `GET /v1/aml/{token}` |
| `catalog().currencies()` | `checkout().list_currencies()` | `listCurrencies` | `GET /v1/currencies` |
| `payment_links().checkout()` | `checkout().payment_link()` | `checkoutPaymentLink` | `POST /v1/link/{id}/checkout` |
| `payments().select()` | `checkout().select_method()` | `selectCheckoutMethod` | `POST /v1/pay/{id}/select` |
| — (new) | `checkout().start_onramp()` | `startCheckoutOnramp` | `POST /v1/pay/{id}/onramp` |
| — (new) | `checkout().submit_source_of_funds()` | `submitSourceOfFunds` | `POST /v1/aml/{token}` |
| `documents().create_job()` | `documents().create_job()` | `createDocumentJob` | `POST /v1/documents/jobs` |
| `documents().job_file()` | `documents().download_job_file()` | `downloadDocumentJobFile` | `GET /v1/documents/jobs/file` |
| `documents().balance_certificate()` | `documents().get_balance()` | `getBalanceDocument` | `GET /v1/documents/balance` |
| `documents().batch_report()` | `documents().get_batch()` | `getBatchDocument` | `GET /v1/documents/batch` |
| `documents().fee_schedule()` | `documents().get_fees()` | `getFeesDocument` | `GET /v1/documents/fees` |
| `documents().job_info()` | `documents().get_job()` | `getDocumentJob` | `POST /v1/documents/jobs/info` |
| `documents().ledger()` | `documents().get_ledger()` | `getLedgerDocument` | `GET /v1/documents/ledger` |
| `documents().link_report()` | `documents().get_payment_link()` | `getPaymentLinkDocument` | `GET /v1/documents/link` |
| `payout_links().cheque()` | `documents().get_payout_link_cheque()` | `getPayoutLinkCheque` | `POST /v1/payout/link/cheque` |
| `documents().referrals_report()` | `documents().get_referrals()` | `getReferralsDocument` | `GET /v1/documents/referrals` |
| `documents().download()` | `documents().get_signed()` | `getSignedDocument` | `GET /v1/documents/{kind}/{id}` |
| `documents().split_report()` | `documents().get_split()` | `getSplitDocument` | `GET /v1/documents/split` |
| `documents().statement()` | `documents().get_statement()` | `getStatementDocument` | `GET /v1/documents/statement` |
| `documents().wallet_statement()` | `documents().get_wallet_statement()` | `getWalletStatementDocument` | `GET /v1/documents/wallet/statement` |
| `payment_links().create()` | `payment_links().create()` | `createPaymentLink` | `POST /v1/payment/link` |
| `payment_links().info()` | `payment_links().get()` | `getPaymentLink` | `POST /v1/payment/link/info` |
| `payment_links().list()` | `payment_links().list()` | `listPaymentLinks` | `POST /v1/payment/link/list` |
| `payment_links().toggle()` | `payment_links().toggle()` | `togglePaymentLink` | `POST /v1/payment/link/toggle` |
| `payments().cancel()` | `payments().cancel()` | `cancelPayment` | `POST /v1/payment/cancel` |
| `payments().create()` | `payments().create()` | `createPayment` | `POST /v1/payment` |
| — (new) | `payments().get_aml_links()` | `getPaymentAmlLinks` | `POST /v1/payment/aml-links` |
| — (new) | `payments().get_checkout_config()` | `getCheckoutConfig` | `POST /v1/checkout-config/get` |
| `payments().info()` | `payments().get_info()` | `getPaymentInfo` | `POST /v1/payment/info` |
| `payments().qr()` | `payments().get_qr()` | `getPaymentQr` | `POST /v1/payment/qr` |
| `payments().history()` | `payments().list_history()` | `listPaymentHistory` | `POST /v1/payment/history` |
| `payments().services()` | `payments().list_services()` | `listPaymentServices` | `POST /v1/payment/services` |
| `refunds().resolve()` | `payments().resolve()` | `resolvePayment` | `POST /v1/payment/resolve` |
| `payments().send_email()` | `payments().send_email()` | `sendPaymentEmail` | `POST /v1/payment/send-email` |
| — (new) | `payments().set_checkout_config()` | `setCheckoutConfig` | `POST /v1/checkout-config/set` |
| `payout_links().cancel()` | `payout_links().cancel()` | `cancelPayoutLink` | `POST /v1/payout/link/cancel` |
| `payout_links().claim()` | `payout_links().claim_payout()` | `claimPayout` | `POST /v1/claim/{token}` |
| `payout_links().create()` | `payout_links().create()` | `createPayoutLink` | `POST /v1/payout/link` |
| `payout_links().batch()` | `payout_links().create_batch()` | `createPayoutLinkBatch` | `POST /v1/payout/link/batch` |
| `payout_links().info()` | `payout_links().get()` | `getPayoutLink` | `POST /v1/payout/link/info` |
| `payout_links().claim_preview()` | `payout_links().get_payout_claim()` | `getPayoutClaim` | `GET /v1/claim/{token}` |
| `payout_links().list()` | `payout_links().list()` | `listPayoutLinks` | `POST /v1/payout/link/list` |
| `payouts().approve()` | `payouts().approve()` | `approvePayout` | `POST /v1/payout/approve` |
| `payouts().calculate()` | `payouts().calculate()` | `calculatePayout` | `POST /v1/payout/calculate` |
| `payouts().cancel()` | `payouts().cancel()` | `cancelPayout` | `POST /v1/payout/cancel` |
| `payouts().create()` | `payouts().create()` | `createPayout` | `POST /v1/payout` |
| `payouts().mass()` | `payouts().create_mass()` | `createMassPayout` | `POST /v1/payout/mass` |
| `transfers().batch()` | `payouts().create_transfer_batch()` | `createTransferBatch` | `POST /v1/transfer/batch` |
| `payouts().info()` | `payouts().get_info()` | `getPayoutInfo` | `POST /v1/payout/info` |
| `payouts().history()` | `payouts().list_history()` | `listPayoutHistory` | `POST /v1/payout/history` |
| `payouts().services()` | `payouts().list_services()` | `listPayoutServices` | `POST /v1/payout/services` |
| `transfers().to_personal()` | `payouts().transfer_to_personal()` | `transferToPersonal` | `POST /v1/transfer/to-personal` |
| `transfers().to_user()` | `payouts().transfer_to_user()` | `transferToUser` | `POST /v1/transfer/to-user` |
| `payouts().validate()` | `payouts().validate()` | `validatePayout` | `POST /v1/payout/validate` |
| `account().referral()` | `referrals().get_info()` | `getReferralInfo` | `POST /v1/referral/info` |
| `wallets().refund_blocked_deposit()` | `refunds().blocked_wallet()` | `refundBlockedWallet` | `POST /v1/wallet/blocked-address-refund` |
| `refunds().create()` | `refunds().payment()` | `refundPayment` | `POST /v1/payment/refund` |
| `sandbox().faucet()` | `sandbox().faucet()` | `sandboxFaucet` | `POST /v1/sandbox/faucet` |
| `sandbox().webhooks()` | `sandbox().list_webhooks()` | `sandboxListWebhooks` | `GET /v1/sandbox/webhooks` |
| `merchants().create_sandbox()` | `sandbox().onboard_store()` | `onboardSandboxStore` | `POST /v1/merchants/{id}/sandbox` |
| `sandbox().replay()` | `sandbox().replay_webhook()` | `sandboxReplayWebhook` | `POST /v1/sandbox/webhooks/replay` |
| `sandbox().reset()` | `sandbox().reset()` | `sandboxReset` | `POST /v1/sandbox/reset` |
| `sandbox().deposit()` | `sandbox().simulate_deposit()` | `sandboxSimulateDeposit` | `POST /v1/sandbox/deposit` |
| `account().vrcs()`, `account().set_vrcs()` | `settings().configure_vrcs()` | `configureVrcs` | `POST /v1/vrcs` |
| `settings().delete_auto_withdraw()` | `settings().delete_auto_withdraw_rule()` | `deleteAutoWithdrawRule` | `POST /v1/auto-withdraw/delete` |
| `settings().get_accuracy()` | `settings().get_accuracy()` | `getAccuracy` | `POST /v1/payment/accuracy/get` |
| — (new) | `settings().get_auto_convert()` | `getAutoConvert` | `POST /v1/payment/autoconvert/get` |
| `settings().get_auto_refund()` | `settings().get_auto_refund()` | `getAutoRefund` | `POST /v1/payment/autorefund/get` |
| `settings().get_payment_fee_config()` | `settings().get_payment_fee_config()` | `getPaymentFeeConfig` | `POST /v1/payment/fee-config/get` |
| `payouts().get_fee_config()` | `settings().get_payout_fee_config()` | `getPayoutFeeConfig` | `POST /v1/payout/fee-config/get` |
| `payouts().get_refund_fee_config()` | `settings().get_refund_fee_config()` | `getRefundFeeConfig` | `POST /v1/payout/refund-fee-config/get` |
| `settings().list_accepted()` | `settings().list_accepted_currencies()` | `listAcceptedCurrencies` | `POST /v1/payment/accepted/list` |
| — (new) | `settings().list_api_log()` | `listApiLog` | `POST /v1/payment/api-log` |
| `settings().list_auto_withdraw()` | `settings().list_auto_withdraw_rules()` | `listAutoWithdrawRules` | `POST /v1/auto-withdraw/list` |
| `settings().list_discounts()` | `settings().list_discounts()` | `listDiscounts` | `POST /v1/payment/discount/list` |
| `settings().set_accepted()` | `settings().set_accepted_currencies()` | `setAcceptedCurrencies` | `POST /v1/payment/accepted/set` |
| `settings().set_accuracy()` | `settings().set_accuracy()` | `setAccuracy` | `POST /v1/payment/accuracy/set` |
| — (new) | `settings().set_auto_convert()` | `setAutoConvert` | `POST /v1/payment/autoconvert/set` |
| `settings().set_auto_refund()` | `settings().set_auto_refund()` | `setAutoRefund` | `POST /v1/payment/autorefund/set` |
| `settings().set_auto_withdraw()` | `settings().set_auto_withdraw_rule()` | `setAutoWithdrawRule` | `POST /v1/auto-withdraw/set` |
| `settings().set_discount()` | `settings().set_discount()` | `setDiscount` | `POST /v1/payment/discount/set` |
| `settings().set_payment_fee_config()` | `settings().set_payment_fee_config()` | `setPaymentFeeConfig` | `POST /v1/payment/fee-config/set` |
| `payouts().set_fee_config()` | `settings().set_payout_fee_config()` | `setPayoutFeeConfig` | `POST /v1/payout/fee-config/set` |
| `payouts().set_refund_fee_config()` | `settings().set_refund_fee_config()` | `setRefundFeeConfig` | `POST /v1/payout/refund-fee-config/set` |
| `splits().create_rule()` | `splits().create_rule()` | `createSplitRule` | `POST /v1/split/rule` |
| `splits().delete_rule()` | `splits().delete_rule()` | `deleteSplitRule` | `POST /v1/split/rule/delete` |
| `splits().get_config()` | `splits().get_config()` | `getSplitConfig` | `POST /v1/split/config/get` |
| `splits().get_opt_in()` | `splits().get_recipient_opt_in()` | `getSplitRecipientOptIn` | `POST /v1/split/recipient/optin/get` |
| `splits().list_rules()` | `splits().list_rules()` | `listSplitRules` | `POST /v1/split/rule/list` |
| `splits().set_config()` | `splits().set_config()` | `setSplitConfig` | `POST /v1/split/config/set` |
| `splits().set_opt_in()` | `splits().set_recipient_opt_in()` | `setSplitRecipientOptIn` | `POST /v1/split/recipient/optin` |
| `wallets().block()` | `wallets().block()` | `blockWallet` | `POST /v1/wallet/block` |
| `wallets().create()` | `wallets().create()` | `createWallet` | `POST /v1/wallet` |
| `wallets().qr()` | `wallets().get_qr()` | `getWalletQr` | `POST /v1/wallet/qr` |
| `webhooks().deliveries()` | `webhooks().list_deliveries()` | `listWebhookDeliveries` | `POST /v1/webhooks/deliveries` |
| `webhooks().register()` | `webhooks().register()` | `registerWebhook` | `POST /v1/webhooks` |
| — (new) | `webhooks().requeue_delivery()` | `requeueWebhookDelivery` | `POST /v1/webhooks/deliveries/requeue` |
| `payments().resend()` | `webhooks().resend_payment()` | `resendPaymentWebhook` | `POST /v1/payment/resend` |
| `webhooks().rotate_secret()` | `webhooks().rotate_secret()` | `rotateWebhookSecret` | `POST /v1/webhooks/rotate-secret` |
| `webhooks().test_legacy()` | `webhooks().send_legacy_test()` | `sendLegacyTestWebhook` | `POST /v1/payment/testing-webhook` |
| — (new) | `webhooks().send_test_conversion()` | `sendTestConversionWebhook` | `POST /v1/test-webhook/conversion` |
| `webhooks().test(WebhookKind::Payment, ..)` | `webhooks().send_test_payment()` | `sendTestPaymentWebhook` | `POST /v1/test-webhook/payment` |
| `webhooks().test(WebhookKind::Payout, ..)` | `webhooks().send_test_payout()` | `sendTestPayoutWebhook` | `POST /v1/test-webhook/payout` |
| `webhooks().test(WebhookKind::Wallet, ..)` | `webhooks().send_test_wallet()` | `sendTestWalletWebhook` | `POST /v1/test-webhook/wallet` |
| — (new) | `webhooks().set_active()` | `setWebhookActive` | `POST /v1/webhooks/active` |
