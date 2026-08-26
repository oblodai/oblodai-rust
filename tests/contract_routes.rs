//! Every route the core declares has exactly one SDK method, wired to the right method, path,
//! credential and idempotency wrapper. The table below is the SDK's coverage ledger: a route the
//! core adds fails this test until a method is added for it.

// A `Client` only exists with an HTTP backend feature on.
#![cfg(feature = "reqwest-client")]
#![allow(deprecated)]

mod support;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use oblodai::contract::requests::*;
use oblodai::contract::types::{Method, RouteAuth};
use oblodai::resources::{DocumentQuery, FormatQuery, PageParams, PeriodQuery, SignedLinkQuery};
use oblodai::{Client, HttpBackend, ROUTES};
use serde_json::json;
use support::{ok, MockBackend, Scripted};

type Case = Box<dyn Fn(Client) -> Pin<Box<dyn Future<Output = ()>>>>;

fn case<Fut: Future<Output = ()> + 'static>(f: impl Fn(Client) -> Fut + 'static) -> Case {
    Box::new(move |client| Box::pin(f(client)))
}

/// An answer generic enough that no method chokes before the request is recorded.
fn any_answer(bare: bool) -> Scripted {
    if bare {
        Scripted {
            status: 200,
            body: "%PDF".into(),
            headers: vec![("content-type".into(), "application/pdf".into())],
            ..Default::default()
        }
    } else {
        ok(json!({
            "items": [],
            "paginate": { "total": 0, "per_page": 1, "offset": 0, "has_pages": false },
            "enabled": true
        }))
    }
}

fn coverage() -> Vec<(&'static str, Case)> {
    vec![
        (
            "POST /v1/api-allowlist/add",
            case(|c| async move {
                let _ = c.settings().add_api_allowlist("10.0.0.0/8").await;
            }),
        ),
        (
            "POST /v1/api-allowlist/enable",
            case(|c| async move {
                let _ = c.settings().enable_api_allowlist(true).await;
            }),
        ),
        (
            "POST /v1/api-allowlist/list",
            case(|c| async move {
                let _ = c.settings().list_api_allowlist().await;
            }),
        ),
        (
            "POST /v1/api-allowlist/remove",
            case(|c| async move {
                let _ = c.settings().remove_api_allowlist("10.0.0.0/8").await;
            }),
        ),
        (
            "POST /v1/auto-withdraw/delete",
            case(|c| async move {
                let _ = c.settings().delete_auto_withdraw("USDT").await;
            }),
        ),
        (
            "POST /v1/auto-withdraw/list",
            case(|c| async move {
                let _ = c.settings().list_auto_withdraw().await;
            }),
        ),
        (
            "POST /v1/auto-withdraw/set",
            case(|c| async move {
                let _ = c
                    .settings()
                    .set_auto_withdraw(AutoWithdrawSetRequest {
                        currency: "USDT".into(),
                        network: "tron".into(),
                        address: "T".into(),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/balance",
            case(|c| async move {
                let _ = c.account().balance().await;
            }),
        ),
        (
            "POST /v1/batch/info",
            case(|c| async move {
                let _ = c
                    .batches()
                    .info(BatchInfoRequest {
                        batch_id: "b1".into(),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "GET /v1/claim/{token}",
            case(|c| async move {
                let _ = c.payout_links().claim_preview("tok").await;
            }),
        ),
        (
            "POST /v1/claim/{token}",
            case(|c| async move {
                let _ = c
                    .payout_links()
                    .claim(
                        "tok",
                        ClaimRequest {
                            address: "T".into(),
                            ..Default::default()
                        },
                    )
                    .await;
            }),
        ),
        (
            "GET /v1/currencies",
            case(|c| async move {
                let _ = c.catalog().currencies().await;
            }),
        ),
        (
            "GET /v1/documents/balance",
            case(|c| async move {
                let _ = c
                    .documents()
                    .balance_certificate(DocumentQuery::default())
                    .await;
            }),
        ),
        (
            "GET /v1/documents/batch",
            case(|c| async move {
                let _ = c
                    .documents()
                    .batch_report("b1", FormatQuery::default())
                    .await;
            }),
        ),
        (
            "GET /v1/documents/fees",
            case(|c| async move {
                let _ = c.documents().fee_schedule(DocumentQuery::default()).await;
            }),
        ),
        (
            "POST /v1/documents/jobs",
            case(|c| async move {
                let _ = c
                    .documents()
                    .create_job(DocumentsJobsRequest {
                        kind: "statement".into(),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "GET /v1/documents/jobs/file",
            case(|c| async move {
                let _ = c.documents().job_file("j1").await;
            }),
        ),
        (
            "POST /v1/documents/jobs/info",
            case(|c| async move {
                let _ = c.documents().job_info("j1").await;
            }),
        ),
        (
            "GET /v1/documents/ledger",
            case(|c| async move {
                let _ = c.documents().ledger(PeriodQuery::default()).await;
            }),
        ),
        (
            "GET /v1/documents/link",
            case(|c| async move {
                let _ = c
                    .documents()
                    .link_report("l1", FormatQuery::default())
                    .await;
            }),
        ),
        (
            "GET /v1/documents/referrals",
            case(|c| async move {
                let _ = c.documents().referrals_report(PeriodQuery::default()).await;
            }),
        ),
        (
            "GET /v1/documents/split",
            case(|c| async move {
                let _ = c
                    .documents()
                    .split_report("i1", DocumentQuery::default())
                    .await;
            }),
        ),
        (
            "GET /v1/documents/statement",
            case(|c| async move {
                let _ = c
                    .documents()
                    .statement(PeriodQuery {
                        from: Some("2026-01-01".into()),
                        to: Some("2026-02-01".into()),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "GET /v1/documents/wallet/statement",
            case(|c| async move {
                let _ = c
                    .documents()
                    .wallet_statement("w1", PeriodQuery::default())
                    .await;
            }),
        ),
        (
            "GET /v1/documents/{kind}/{id}",
            case(|c| async move {
                let _ = c
                    .documents()
                    .download("invoice", "i1", SignedLinkQuery::new(1, "s"))
                    .await;
            }),
        ),
        (
            "POST /v1/exchange-rate/list",
            case(|c| async move {
                let _ = c
                    .catalog()
                    .exchange_rates(ExchangeRateListRequest::default())
                    .await;
            }),
        ),
        (
            "GET /v1/link/{id}",
            case(|c| async move {
                let _ = c.payment_links().public_view("l1").await;
            }),
        ),
        (
            "POST /v1/link/{id}/checkout",
            case(|c| async move {
                let _ = c
                    .payment_links()
                    .checkout("l1", LinkCheckoutRequest::default())
                    .await;
            }),
        ),
        (
            "POST /v1/merchants",
            case(|c| async move {
                let _ = c
                    .merchants()
                    .create(MerchantsRequest {
                        email: "a@b.c".into(),
                        name: Some("A".into()),
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/merchants/{id}/sandbox",
            case(|c| async move {
                let _ = c.merchants().create_sandbox("m1").await;
            }),
        ),
        (
            "GET /v1/pay/{id}",
            case(|c| async move {
                let _ = c.payments().public_view("i1").await;
            }),
        ),
        (
            "GET /v1/pay/{id}/qr",
            case(|c| async move {
                let _ = c.payments().public_qr("i1").await;
            }),
        ),
        (
            "POST /v1/pay/{id}/select",
            case(|c| async move {
                let _ = c
                    .payments()
                    .select(
                        "i1",
                        PaySelectRequest {
                            currency: "USDT".into(),
                            network: "tron".into(),
                        },
                    )
                    .await;
            }),
        ),
        (
            "POST /v1/payment",
            case(|c| async move {
                let _ = c
                    .payments()
                    .create(PaymentRequest {
                        amount: "1".into(),
                        currency: "USDT".into(),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/payment/accepted/list",
            case(|c| async move {
                let _ = c
                    .settings()
                    .list_accepted(PaymentAcceptedListRequest::default())
                    .await;
            }),
        ),
        (
            "POST /v1/payment/accepted/set",
            case(|c| async move {
                let _ = c
                    .settings()
                    .set_accepted(PaymentAcceptedSetRequest { accepted: vec![] })
                    .await;
            }),
        ),
        (
            "POST /v1/payment/accuracy/get",
            case(|c| async move {
                let _ = c.settings().get_accuracy().await;
            }),
        ),
        (
            "POST /v1/payment/accuracy/set",
            case(|c| async move {
                let _ = c
                    .settings()
                    .set_accuracy(PaymentAccuracySetRequest {
                        enabled: true,
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/payment/autorefund/get",
            case(|c| async move {
                let _ = c.settings().get_auto_refund().await;
            }),
        ),
        (
            "POST /v1/payment/autorefund/set",
            case(|c| async move {
                let _ = c
                    .settings()
                    .set_auto_refund(PaymentAutorefundSetRequest {
                        overpay: true,
                        underpay: false,
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/payment/batch",
            case(|c| async move {
                let _ = c
                    .payments()
                    .batch(PaymentBatchRequest {
                        payments: vec![],
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/payment/cancel",
            case(|c| async move {
                let _ = c.payments().cancel("i1").await;
            }),
        ),
        (
            "POST /v1/payment/discount/list",
            case(|c| async move {
                let _ = c
                    .settings()
                    .list_discounts(PaymentDiscountListRequest::default())
                    .await;
            }),
        ),
        (
            "POST /v1/payment/discount/set",
            case(|c| async move {
                let _ = c
                    .settings()
                    .set_discount(PaymentDiscountSetRequest {
                        discount_percent: 1,
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/payment/fee-config/get",
            case(|c| async move {
                let _ = c.settings().get_payment_fee_config().await;
            }),
        ),
        (
            "POST /v1/payment/fee-config/set",
            case(|c| async move {
                let _ = c
                    .settings()
                    .set_payment_fee_config(PaymentFeeConfigSetRequest {
                        payer_pays_percent: 50,
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/payment/history",
            case(|c| async move {
                let _ = c.payments().history(PaymentHistoryRequest::default()).await;
            }),
        ),
        (
            "POST /v1/payment/info",
            case(|c| async move {
                let _ = c.payments().info("i1").await;
            }),
        ),
        (
            "POST /v1/payment/link",
            case(|c| async move {
                let _ = c
                    .payment_links()
                    .create(PaymentLinkRequest {
                        amount_mode: "open".into(),
                        currency: "USDT".into(),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/payment/link/info",
            case(|c| async move {
                let _ = c.payment_links().info("l1", PageParams::default()).await;
            }),
        ),
        (
            "POST /v1/payment/link/list",
            case(|c| async move {
                let _ = c
                    .payment_links()
                    .list(PaymentLinkListRequest::default())
                    .await;
            }),
        ),
        (
            "POST /v1/payment/link/toggle",
            case(|c| async move {
                let _ = c.payment_links().toggle("l1", false).await;
            }),
        ),
        (
            "POST /v1/payment/qr",
            case(|c| async move {
                let _ = c.payments().qr("i1").await;
            }),
        ),
        (
            "POST /v1/payment/refund",
            case(|c| async move {
                let _ = c
                    .refunds()
                    .create(PaymentRefundRequest {
                        uuid: Some("i1".into()),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/payment/resend",
            case(|c| async move {
                let _ = c.payments().resend("i1").await;
            }),
        ),
        (
            "POST /v1/payment/resolve",
            case(|c| async move {
                let _ = c
                    .refunds()
                    .resolve(PaymentResolveRequest {
                        action: "accept".into(),
                        uuid: Some("i1".into()),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/payment/send-email",
            case(|c| async move {
                let _ = c
                    .payments()
                    .send_email(PaymentSendEmailRequest {
                        uuid: Some("i1".into()),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/payment/services",
            case(|c| async move {
                let _ = c
                    .payments()
                    .services(PaymentServicesRequest::default())
                    .await;
            }),
        ),
        (
            "POST /v1/payment/testing-webhook",
            case(|c| async move {
                let _ = c
                    .webhooks()
                    .test_legacy(PaymentTestingWebhookRequest {
                        url: Some("https://x".into()),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/payout",
            case(|c| async move {
                let _ = c
                    .payouts()
                    .create(PayoutRequest {
                        amount: "1".into(),
                        currency: "USDT".into(),
                        address: "T".into(),
                        order_id: "o".into(),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/payout/approve",
            case(|c| async move {
                let _ = c.payouts().approve("p1").await;
            }),
        ),
        (
            "POST /v1/payout/batch",
            case(|c| async move {
                let _ = c
                    .payouts()
                    .batch(PayoutBatchRequest {
                        payouts: vec![],
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/payout/calculate",
            case(|c| async move {
                let _ = c
                    .payouts()
                    .calculate(PayoutCalculateRequest {
                        amount: "1".into(),
                        currency: "USDT".into(),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/payout/cancel",
            case(|c| async move {
                let _ = c.payouts().cancel("p1").await;
            }),
        ),
        (
            "POST /v1/payout/fee-config/get",
            case(|c| async move {
                let _ = c.payouts().get_fee_config().await;
            }),
        ),
        (
            "POST /v1/payout/fee-config/set",
            case(|c| async move {
                let _ = c
                    .payouts()
                    .set_fee_config(PayoutFeeConfigSetRequest {
                        fee_on_recipient: true,
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/payout/history",
            case(|c| async move {
                let _ = c.payouts().history(PayoutHistoryRequest::default()).await;
            }),
        ),
        (
            "POST /v1/payout/info",
            case(|c| async move {
                let _ = c.payouts().info("p1").await;
            }),
        ),
        (
            "POST /v1/payout/link",
            case(|c| async move {
                let _ = c
                    .payout_links()
                    .create(PayoutLinkRequest {
                        amount: "1".into(),
                        currency: "USDT".into(),
                        network: "tron".into(),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/payout/link/batch",
            case(|c| async move {
                let _ = c
                    .payout_links()
                    .batch(PayoutLinkBatchRequest { items: vec![] })
                    .await;
            }),
        ),
        (
            "POST /v1/payout/link/cancel",
            case(|c| async move {
                let _ = c.payout_links().cancel("l1").await;
            }),
        ),
        (
            "POST /v1/payout/link/cheque",
            case(|c| async move {
                let _ = c
                    .payout_links()
                    .cheque(PayoutLinkChequeRequest {
                        claim_token: "t".into(),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/payout/link/info",
            case(|c| async move {
                let _ = c.payout_links().info("l1").await;
            }),
        ),
        (
            "POST /v1/payout/link/list",
            case(|c| async move {
                let _ = c
                    .payout_links()
                    .list(PayoutLinkListRequest::default())
                    .await;
            }),
        ),
        (
            "POST /v1/payout/mass",
            case(|c| async move {
                let _ = c
                    .payouts()
                    .mass(PayoutMassRequest {
                        payouts: vec![],
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/payout/refund-fee-config/get",
            case(|c| async move {
                let _ = c.payouts().get_refund_fee_config().await;
            }),
        ),
        (
            "POST /v1/payout/refund-fee-config/set",
            case(|c| async move {
                let _ = c
                    .payouts()
                    .set_refund_fee_config(PayoutRefundFeeConfigSetRequest {
                        fee_on_customer: true,
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/payout/services",
            case(|c| async move {
                let _ = c.payouts().services(PayoutServicesRequest::default()).await;
            }),
        ),
        (
            "POST /v1/payout/validate",
            case(|c| async move {
                let _ = c
                    .payouts()
                    .validate(PayoutValidateRequest {
                        amount: "1".into(),
                        currency: "USDT".into(),
                        address: "T".into(),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/referral/info",
            case(|c| async move {
                let _ = c.account().referral().await;
            }),
        ),
        (
            "POST /v1/refund/batch",
            case(|c| async move {
                let _ = c
                    .refunds()
                    .batch(RefundBatchRequest {
                        refunds: vec![],
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/sandbox/deposit",
            case(|c| async move {
                let _ = c
                    .sandbox()
                    .deposit(SandboxDepositRequest {
                        invoice_id: "i1".into(),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/sandbox/faucet",
            case(|c| async move {
                let _ = c
                    .sandbox()
                    .faucet(SandboxFaucetRequest {
                        asset: "USDT".into(),
                        amount: "1".into(),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/sandbox/reset",
            case(|c| async move {
                let _ = c.sandbox().reset().await;
            }),
        ),
        (
            "GET /v1/sandbox/webhooks",
            case(|c| async move {
                let _ = c.sandbox().webhooks(PageParams::default()).await;
            }),
        ),
        (
            "POST /v1/sandbox/webhooks/replay",
            case(|c| async move {
                let _ = c.sandbox().replay("d1").await;
            }),
        ),
        (
            "POST /v1/split/config/get",
            case(|c| async move {
                let _ = c.splits().get_config().await;
            }),
        ),
        (
            "POST /v1/split/config/set",
            case(|c| async move {
                let _ = c
                    .splits()
                    .set_config(SplitConfigSetRequest {
                        refund_hold_seconds: 60,
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/split/recipient/optin",
            case(|c| async move {
                let _ = c.splits().set_opt_in(true).await;
            }),
        ),
        (
            "POST /v1/split/recipient/optin/get",
            case(|c| async move {
                let _ = c.splits().get_opt_in().await;
            }),
        ),
        (
            "POST /v1/split/rule",
            case(|c| async move {
                let _ = c
                    .splits()
                    .create_rule(SplitRuleRequest {
                        percent: "10".into(),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/split/rule/delete",
            case(|c| async move {
                let _ = c.splits().delete_rule("r1").await;
            }),
        ),
        (
            "POST /v1/split/rule/list",
            case(|c| async move {
                let _ = c.splits().list_rules(SplitRuleListRequest::default()).await;
            }),
        ),
        (
            "POST /v1/test-webhook/payment",
            case(|c| async move {
                let _ = c
                    .webhooks()
                    .test(
                        oblodai::WebhookKind::Payment,
                        TestWebhookPaymentRequest {
                            url_callback: "https://x".into(),
                            ..Default::default()
                        },
                    )
                    .await;
            }),
        ),
        (
            "POST /v1/test-webhook/payout",
            case(|c| async move {
                let _ = c
                    .webhooks()
                    .test(
                        oblodai::WebhookKind::Payout,
                        TestWebhookPaymentRequest {
                            url_callback: "https://x".into(),
                            ..Default::default()
                        },
                    )
                    .await;
            }),
        ),
        (
            "POST /v1/test-webhook/wallet",
            case(|c| async move {
                let _ = c
                    .webhooks()
                    .test(
                        oblodai::WebhookKind::Wallet,
                        TestWebhookPaymentRequest {
                            url_callback: "https://x".into(),
                            ..Default::default()
                        },
                    )
                    .await;
            }),
        ),
        (
            "POST /v1/transfer/batch",
            case(|c| async move {
                let _ = c.transfers().batch(TransferBatchRequest::default()).await;
            }),
        ),
        (
            "POST /v1/transfer/to-personal",
            case(|c| async move {
                let _ = c
                    .transfers()
                    .to_personal(TransferToPersonalRequest {
                        amount: "1".into(),
                        currency: "USDT".into(),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/transfer/to-user",
            case(|c| async move {
                let _ = c
                    .transfers()
                    .to_user(TransferToUserRequest {
                        amount: "1".into(),
                        currency: "USDT".into(),
                        to_user_id: "u".into(),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/vrcs",
            case(|c| async move {
                let _ = c.account().vrcs().await;
            }),
        ),
        (
            "POST /v1/wallet",
            case(|c| async move {
                let _ = c
                    .wallets()
                    .create(WalletRequest {
                        currency: "USDT".into(),
                        network: "tron".into(),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/wallet/block",
            case(|c| async move {
                let _ = c
                    .wallets()
                    .block(WalletBlockRequest {
                        address: "T".into(),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/wallet/blocked-address-refund",
            case(|c| async move {
                let _ = c
                    .wallets()
                    .refund_blocked_deposit(WalletBlockedAddressRefundRequest {
                        uuid: "w1".into(),
                        address: "T".into(),
                        ..Default::default()
                    })
                    .await;
            }),
        ),
        (
            "POST /v1/wallet/qr",
            case(|c| async move {
                let _ = c.wallets().qr("T").await;
            }),
        ),
        (
            "POST /v1/webhooks",
            case(|c| async move {
                let _ = c.webhooks().register("https://x").await;
            }),
        ),
        (
            "POST /v1/webhooks/deliveries",
            case(|c| async move {
                let _ = c
                    .webhooks()
                    .deliveries(WebhooksDeliveriesRequest::default())
                    .await;
            }),
        ),
        (
            "POST /v1/webhooks/rotate-secret",
            case(|c| async move {
                let _ = c.webhooks().rotate_secret().await;
            }),
        ),
    ]
}

#[test]
fn the_table_is_the_core_surface_nothing_more_and_nothing_less() {
    let declared: std::collections::BTreeSet<&str> = ROUTES.iter().map(|r| r.key).collect();
    let covered: std::collections::BTreeSet<&str> = coverage().iter().map(|(k, _)| *k).collect();
    let missing: Vec<_> = declared.difference(&covered).collect();
    let extra: Vec<_> = covered.difference(&declared).collect();
    assert!(missing.is_empty(), "routes with no SDK method: {missing:?}");
    assert!(
        extra.is_empty(),
        "table names routes the core does not declare: {extra:?}"
    );
    assert_eq!(
        declared.len(),
        107,
        "the snapshot declares 107 merchant routes"
    );
}

/// Every flag of every generated `RouteSpec` equals `contract/contract.json`, field by field.
///
/// `safe` is the one that authorises re-sending a request after a transport failure, so it is the
/// core's own hand-classified statement and never a guess from the path. `list` decides whether a
/// method pages. A flipped flag here is a money bug, so the comparison is exhaustive rather than a
/// key-set check — see `a_flipped_flag_is_caught` for the proof that it bites.
#[test]
fn every_route_flag_equals_the_contract() {
    let contract = support::load_contract();
    let declared = contract["routes"].as_array().expect("routes array");
    let mut seen = 0usize;
    for route in declared {
        let key = format!(
            "{} {}",
            route["method"].as_str().unwrap(),
            route["path"].as_str().unwrap()
        );
        let Some(spec) = ROUTES.iter().find(|r| r.key == key) else {
            continue; // /healthz, /docs and friends are not merchant routes
        };
        seen += 1;
        assert_eq!(
            spec.method.as_str(),
            route["method"].as_str().unwrap(),
            "{key}: method"
        );
        assert_eq!(spec.path, route["path"].as_str().unwrap(), "{key}: path");
        assert_eq!(
            spec.auth.as_str(),
            route["auth"].as_str().unwrap(),
            "{key}: auth"
        );
        assert_eq!(
            spec.idempotent,
            route["idempotent"].as_bool().unwrap(),
            "{key}: idempotent"
        );
        assert_eq!(
            spec.safe,
            route["safe"]
                .as_bool()
                .unwrap_or_else(|| panic!("{key}: the contract carries no boolean `safe`")),
            "{key}: safe — the flag that decides whether a failed request may be re-sent"
        );
        assert_eq!(spec.bare, route["bare"].as_bool().unwrap(), "{key}: bare");
        let list = route.get("list").and_then(|v| v.as_str());
        assert_eq!(
            spec.list.map(|l| match l {
                oblodai::ListKind::Paged => "paged",
                oblodai::ListKind::Plain => "plain",
                other => panic!("{key}: unknown list kind {other:?}"),
            }),
            list,
            "{key}: list"
        );
    }
    assert_eq!(
        seen,
        ROUTES.len(),
        "every generated route came from the contract"
    );
    assert_eq!(seen, 107, "the snapshot declares 107 merchant routes");
}

/// The comparison above is only worth anything if it fails when a flag is wrong. Flip each flag of
/// a real route on a copy of the contract and require the same comparison to reject it.
#[test]
fn a_flipped_flag_is_caught() {
    let contract = support::load_contract();
    let route = contract["routes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["path"] == "/v1/payout" && r["method"] == "POST")
        .expect("POST /v1/payout is in the contract")
        .clone();
    let spec = ROUTES.iter().find(|r| r.key == "POST /v1/payout").unwrap();

    // As shipped, everything agrees.
    assert_eq!(spec.safe, route["safe"].as_bool().unwrap());
    assert_eq!(spec.idempotent, route["idempotent"].as_bool().unwrap());
    assert_eq!(spec.bare, route["bare"].as_bool().unwrap());

    // Flip each of them in turn: the generated spec must disagree with the tampered contract.
    for field in ["safe", "idempotent", "bare"] {
        let mut tampered = route.clone();
        let flipped = !tampered[field].as_bool().unwrap();
        tampered[field] = serde_json::Value::Bool(flipped);
        let generated = match field {
            "safe" => spec.safe,
            "idempotent" => spec.idempotent,
            _ => spec.bare,
        };
        assert_ne!(
            generated,
            tampered[field].as_bool().unwrap(),
            "flipping `{field}` must make the per-route comparison fail"
        );
    }
    // And `safe: true` on a create route would be the dangerous direction specifically.
    assert!(
        !spec.safe,
        "POST /v1/payout must never be marked safe to repeat"
    );
}

#[tokio::test]
async fn every_route_is_wired_to_the_right_method_path_and_credential() {
    for (key, call) in coverage() {
        let spec = ROUTES
            .iter()
            .find(|r| r.key == key)
            .unwrap_or_else(|| panic!("{key} is not a declared route"));
        let mock = MockBackend::new(vec![any_answer(spec.bare)]);
        let client = Client::builder()
            .public_id("pk")
            .secret("s")
            .admin_token("adm")
            .base_url("https://api.test")
            .env(Vec::<(String, String)>::new())
            .http_backend(mock.clone() as Arc<dyn HttpBackend>)
            .build()
            .unwrap();

        call(client).await;

        assert_eq!(mock.call_count(), 1, "{key}: expected exactly one request");
        let sent = mock.first();
        assert_eq!(
            sent.method,
            spec.method.as_str(),
            "{key}: wrong HTTP method"
        );

        // The path may carry filled-in parameters, but its shape must be the route's.
        let pattern = spec
            .path
            .split('/')
            .map(|seg| if seg.starts_with('{') { "[^/]+" } else { seg })
            .collect::<Vec<_>>()
            .join("/");
        let path = sent.path();
        let mut expected = pattern.split("[^/]+");
        let mut rest = path.as_str();
        for (i, part) in expected.by_ref().enumerate() {
            if i == 0 {
                assert!(
                    rest.starts_with(part),
                    "{key}: path {path} does not match {pattern}"
                );
                rest = &rest[part.len()..];
            } else if part.is_empty() {
                rest = "";
            } else {
                let at = rest
                    .find(part)
                    .unwrap_or_else(|| panic!("{key}: {path} vs {pattern}"));
                assert!(at > 0, "{key}: an empty path parameter in {path}");
                rest = &rest[at + part.len()..];
            }
        }

        match spec.auth {
            RouteAuth::Public => {
                assert!(
                    sent.header("x-signature").is_none(),
                    "{key}: a public route was signed"
                );
                assert!(sent.header("x-public-id").is_none(), "{key}");
            }
            RouteAuth::Onboard => {
                assert!(
                    sent.header("x-signature").is_none(),
                    "{key}: onboarding is not signed"
                );
                assert_eq!(sent.header("x-admin-token"), Some("adm"), "{key}");
            }
            RouteAuth::Key => {
                assert_eq!(
                    sent.header("x-public-id"),
                    Some("pk"),
                    "{key}: signed with the merchant API key"
                );
                assert_eq!(sent.header("x-signature").unwrap().len(), 64, "{key}");
                assert!(
                    sent.header("x-admin-token").is_none(),
                    "{key}: the admin token belongs to onboard routes only"
                );
            }
            // `RouteAuth` is `#[non_exhaustive]`: a gate the core adds must be classified here
            // before its routes can be trusted, not silently accepted.
            other => panic!("{key}: unhandled auth gate {other:?}"),
        }

        if spec.idempotent {
            assert!(
                sent.header("idempotency-key").is_some(),
                "{key}: a deduplicated route must carry a key"
            );
        } else {
            assert!(
                sent.header("idempotency-key").is_none(),
                "{key}: the core ignores the header here, so sending one would be a lie"
            );
        }

        if spec.method == Method::Get {
            assert!(sent.body.is_none(), "{key}: a GET must not carry a body");
        } else {
            assert!(sent.body.is_some(), "{key}: a POST always carries a body");
            assert_eq!(
                sent.header("content-type"),
                Some("application/json"),
                "{key}"
            );
        }
        assert_eq!(sent.header("accept"), Some("application/json"), "{key}");
        assert!(
            sent.header("user-agent")
                .unwrap()
                .starts_with("oblodai-rust/"),
            "{key}: the user agent identifies the SDK"
        );
    }
}
