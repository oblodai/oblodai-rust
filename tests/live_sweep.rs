//! Every namespace against a REAL core.
//!
//! Ignored by default:
//!
//! ```sh
//! OBLODAI_LIVE_URL=http://127.0.0.1:8095 cargo test --test live_sweep -- --ignored --nocapture
//! ```
//!
//! The point is not the business outcome but that the bodies the SDK sends are accepted (no 400
//! from our own shapes) and the bodies that come back decode (no contract error). Routes that need
//! a subsystem the stand may lack (documents, email) are probed and skipped when the core reports
//! them disabled.

// A `Client` only exists with an HTTP backend feature on.
#![cfg(feature = "reqwest-client")]

mod support;

use std::time::{SystemTime, UNIX_EPOCH};

use futures_util::StreamExt;
use oblodai::contract::requests::*;
use oblodai::resources::{PageParams, SignedLinkQuery};
use oblodai::{Client, ErrorKind, Result};

const ADDRESS: &str = "TQrY8bkbpXKPt2LZbU8jqfnpFbUSF15sbx";

fn base_url() -> String {
    std::env::var("OBLODAI_LIVE_URL").expect("set OBLODAI_LIVE_URL to run the live tests")
}

fn hook_url() -> String {
    std::env::var("OBLODAI_LIVE_HOOK_URL").unwrap_or_else(|_| "http://127.0.0.1:8096/hook".into())
}

fn stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

/// Fails the test on SDK-side shape problems; tolerates business refusals (409/403/404).
fn accept<T>(result: Result<T>, what: &str) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(err) if matches!(err.kind(), ErrorKind::Contract | ErrorKind::Validation) => {
            panic!("{what}: the SDK's own shape was rejected — {err}")
        }
        Err(err) => {
            eprintln!("  {what}: refused by the core ({}) — tolerated", err.code());
            None
        }
    }
}

fn anonymous() -> Client {
    Client::builder()
        .base_url(base_url())
        .allow_insecure_base_url(true)
        .env(Vec::<(String, String)>::new())
        .build()
        .unwrap()
}

async fn onboard() -> Client {
    let public = anonymous();
    let merchant = public
        .merchants()
        .create(MerchantsRequest {
            email: format!("sweep-{}@example.com", stamp()),
            name: Some("Sweep".into()),
        })
        .await
        .unwrap();
    let sandbox = public
        .merchants()
        .create_sandbox(&merchant.merchant_id)
        .await
        .unwrap();
    Client::builder()
        .public_id(sandbox.api_key.public_id)
        .secret(sandbox.api_key.secret)
        .base_url(base_url())
        .allow_insecure_base_url(true)
        .env(Vec::<(String, String)>::new())
        .build()
        .unwrap()
}

#[tokio::test]
#[ignore = "needs a running gateway at OBLODAI_LIVE_URL"]
async fn live_sweep() {
    let client = onboard().await;
    let public = anonymous();
    let hook = hook_url();

    client
        .sandbox()
        .faucet(SandboxFaucetRequest {
            asset: "USDT".into(),
            amount: "1000".into(),
            ..Default::default()
        })
        .await
        .unwrap();
    // A per-invoice url_callback needs a registered endpoint: the core signs with its secret.
    client.webhooks().register(&hook).await.unwrap();

    let invoice = client
        .payments()
        .create(PaymentRequest {
            amount: "25".into(),
            currency: "USDT".into(),
            network: Some("tron".into()),
            order_id: Some(format!("sw-{}", stamp())),
            payer_email: Some("buyer@example.com".into()),
            url_callback: Some(hook.clone()),
            ..Default::default()
        })
        .await
        .unwrap();

    let documents_enabled = match client
        .documents()
        .balance_certificate(Default::default())
        .await
    {
        Ok(_) => true,
        Err(err) if err.code() == "document.disabled" => false,
        Err(err) if err.kind() == ErrorKind::NotFound => false,
        Err(err) => panic!("documents probe: {err}"),
    };

    // --- catalogue & account ---
    assert!(!public
        .catalog()
        .currencies()
        .await
        .unwrap()
        .currencies
        .is_empty());
    let rates = public
        .catalog()
        .exchange_rates(ExchangeRateListRequest {
            currency_from: Some("BTC".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(rates.paginate.per_page > 0);
    client.account().balance().await.unwrap();
    assert!(!client.account().referral().await.unwrap().code.is_empty());
    client.account().vrcs().await.unwrap();
    client.account().set_vrcs(false).await.unwrap();

    // --- payments ---
    assert_eq!(
        client.payments().get(&invoice.uuid).await.unwrap().uuid,
        invoice.uuid
    );
    client.payments().qr(&invoice.uuid).await.unwrap();
    assert!(!client
        .payments()
        .services(Default::default())
        .limit(5)
        .await
        .unwrap()
        .items
        .is_empty());
    assert_eq!(
        public
            .payments()
            .public_view(&invoice.uuid)
            .await
            .unwrap()
            .status,
        oblodai::PaymentStatus::Created
    );
    public.payments().public_qr(&invoice.uuid).await.unwrap();

    let multi = client
        .payments()
        .create(PaymentRequest {
            amount: "10".into(),
            currency: "USDT".into(),
            order_id: Some(format!("sw-multi-{}", stamp())),
            ..Default::default()
        })
        .await
        .unwrap();
    let selected = public
        .payments()
        .select(
            &multi.uuid,
            PaySelectRequest {
                currency: "USDT".into(),
                network: "tron".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(selected.network, oblodai::Network::Tron);

    accept(
        client.payments().resend(&invoice.uuid).await,
        "payments.resend",
    );
    accept(
        client
            .payments()
            .send_email(PaymentSendEmailRequest {
                uuid: Some(invoice.uuid.clone()),
                ..Default::default()
            })
            .await,
        "payments.send_email",
    );

    let batch = client
        .payments()
        .batch(PaymentBatchRequest {
            on_error: Some(oblodai::BatchOnError::Continue),
            payments: vec![PaymentBatchPaymentsItem {
                amount: "3".into(),
                currency: "USDT".into(),
                network: Some("tron".into()),
                order_id: format!("sw-b-{}", stamp()),
                ..Default::default()
            }],
        })
        .await
        .unwrap();
    assert!(!batch.batch_id.is_empty());
    let info = client
        .batches()
        .info(BatchInfoRequest {
            batch_id: batch.batch_id.clone(),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(info.batch_id, batch.batch_id);

    let to_cancel = client
        .payments()
        .create(PaymentRequest {
            amount: "1".into(),
            currency: "USDT".into(),
            network: Some("tron".into()),
            order_id: Some(format!("sw-c-{}", stamp())),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(
        client
            .payments()
            .cancel(&to_cancel.uuid)
            .await
            .unwrap()
            .status,
        oblodai::PaymentStatus::Cancelled
    );

    let mut history = client
        .payments()
        .history(Default::default())
        .limit(2)
        .stream();
    let mut seen = 0;
    while let Some(item) = history.next().await {
        assert!(!item.unwrap().uuid.is_empty());
        seen += 1;
        if seen >= 5 {
            break;
        }
    }
    assert!(seen > 0, "the history walks");

    // --- deposit, refund, resolve ---
    client
        .sandbox()
        .deposit(SandboxDepositRequest {
            invoice_id: invoice.uuid.clone(),
            amount: Some("25".into()),
            confirmations: Some(20),
            txid: Some(format!("sw-tx-{}", stamp())),
        })
        .await
        .unwrap();
    let paid = client.payments().get(&invoice.uuid).await.unwrap();
    assert!(
        matches!(
            paid.status,
            oblodai::PaymentStatus::Paid | oblodai::PaymentStatus::ConfirmCheck
        ),
        "status {}",
        paid.status
    );
    accept(
        client
            .refunds()
            .create(PaymentRefundRequest {
                uuid: Some(invoice.uuid.clone()),
                address: Some(ADDRESS.into()),
                amount: Some("5".into()),
                reference: Some(format!("sw-r-{}", stamp())),
                ..Default::default()
            })
            .await,
        "refunds.create",
    );
    accept(
        client
            .refunds()
            .resolve(PaymentResolveRequest {
                action: oblodai::ResolveAction::Accept,
                uuid: Some(invoice.uuid.clone()),
                ..Default::default()
            })
            .await,
        "refunds.resolve",
    );
    accept(
        client
            .refunds()
            .batch(RefundBatchRequest {
                refunds: vec![RefundBatchRefundsItem {
                    uuid: Some(invoice.uuid.clone()),
                    address: Some(ADDRESS.into()),
                    amount: Some("1".into()),
                    reference: format!("sw-rb-{}", stamp()),
                    ..Default::default()
                }],
                ..Default::default()
            })
            .await,
        "refunds.batch",
    );

    // --- payouts ---
    assert_eq!(
        client
            .payouts()
            .calculate(PayoutCalculateRequest {
                amount: "10".into(),
                currency: "USDT".into(),
                network: Some("tron".into()),
                ..Default::default()
            })
            .await
            .unwrap()
            .currency,
        "USDT"
    );
    assert!(
        client
            .payouts()
            .validate(PayoutValidateRequest {
                amount: "10".into(),
                currency: "USDT".into(),
                network: Some("tron".into()),
                address: ADDRESS.into(),
                ..Default::default()
            })
            .await
            .unwrap()
            .valid
    );
    let payout_order = format!("sw-po-{}", stamp());
    let payout = client
        .payouts()
        .create(PayoutRequest {
            amount: "10".into(),
            currency: "USDT".into(),
            network: Some("tron".into()),
            address: ADDRESS.into(),
            order_id: payout_order.clone(),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(
        client
            .payouts()
            .get(oblodai::Lookup::order_id(&payout_order))
            .await
            .unwrap()
            .uuid,
        payout.uuid
    );
    accept(
        client.payouts().cancel(&payout.uuid).await,
        "payouts.cancel",
    );
    accept(
        client.payouts().approve(&payout.uuid).await,
        "payouts.approve",
    );

    let mass = client
        .payouts()
        .mass(PayoutMassRequest {
            payouts: vec![PayoutMassPayoutsItem {
                amount: "1".into(),
                currency: "USDT".into(),
                network: Some("tron".into()),
                address: ADDRESS.into(),
                order_id: format!("sw-m-{}", stamp()),
                ..Default::default()
            }],
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(mass.items[0].idx, 0);
    assert!(!client
        .payouts()
        .batch(PayoutBatchRequest {
            payouts: vec![PayoutBatchPayoutsItem {
                amount: "1".into(),
                currency: "USDT".into(),
                network: Some("tron".into()),
                address: ADDRESS.into(),
                order_id: format!("sw-pb-{}", stamp()),
                ..Default::default()
            }],
            ..Default::default()
        })
        .await
        .unwrap()
        .batch_id
        .is_empty());
    assert!(!client
        .payouts()
        .services(Default::default())
        .await
        .unwrap()
        .items
        .is_empty());
    client
        .payouts()
        .set_fee_config(PayoutFeeConfigSetRequest {
            fee_on_recipient: true,
        })
        .await
        .unwrap();
    client.payouts().get_fee_config().await.unwrap();
    client
        .payouts()
        .set_refund_fee_config(PayoutRefundFeeConfigSetRequest {
            fee_on_customer: true,
        })
        .await
        .unwrap();
    client.payouts().get_refund_fee_config().await.unwrap();
    client
        .payouts()
        .history(PayoutHistoryRequest {
            kind: Some(oblodai::PayoutKind::Refund),
            ..Default::default()
        })
        .limit(5)
        .await
        .unwrap();

    // --- payout links ---
    let link = client
        .payout_links()
        .create(PayoutLinkRequest {
            amount: "5".into(),
            currency: "USDT".into(),
            network: "tron".into(),
            reference: Some(format!("sw-pl-{}", stamp())),
            title: Some("Bonus".into()),
            expires_in_seconds: Some(3600),
            ..Default::default()
        })
        .await
        .unwrap();
    let claim_token = link
        .claim_token
        .clone()
        .expect("a fresh link hands out its token once");
    assert_eq!(
        client
            .payout_links()
            .get(&link.link_id)
            .await
            .unwrap()
            .status,
        oblodai::PayoutLinkStatus::Funded
    );
    assert!(!client
        .payout_links()
        .list(Default::default())
        .limit(5)
        .await
        .unwrap()
        .items
        .is_empty());
    assert!(
        public
            .payout_links()
            .claim_preview(&claim_token)
            .await
            .unwrap()
            .claimable
    );
    let claimed = public
        .payout_links()
        .claim(
            &claim_token,
            ClaimRequest {
                address: ADDRESS.into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(!claimed.payout_id.is_empty());

    let second = client
        .payout_links()
        .create(PayoutLinkRequest {
            amount: "1".into(),
            currency: "USDT".into(),
            network: "tron".into(),
            reference: Some(format!("sw-pl2-{}", stamp())),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(
        client
            .payout_links()
            .cancel(&second.link_id)
            .await
            .unwrap()
            .status,
        oblodai::PayoutLinkStatus::Cancelled
    );
    let link_batch = client
        .payout_links()
        .batch(PayoutLinkBatchRequest {
            items: vec![PayoutLinkBatchItemsItem {
                amount: "1".into(),
                currency: "USDT".into(),
                network: "tron".into(),
                reference: format!("sw-plb-{}", stamp()),
                ..Default::default()
            }],
        })
        .await
        .unwrap();
    assert!(link_batch.items[0].ok);

    // --- payment links ---
    let created = client
        .payment_links()
        .create(PaymentLinkRequest {
            title: Some("Tip".into()),
            amount_mode: oblodai::AmountMode::Fixed,
            currency: "USDT".into(),
            amount_fixed: Some("10".into()),
            pinned_network: Some("tron".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(
        client
            .payment_links()
            .get(&created.link_id, PageParams::default().limit(5).offset(0))
            .await
            .unwrap()
            .active
    );
    assert!(!client
        .payment_links()
        .list(Default::default())
        .await
        .unwrap()
        .items
        .is_empty());
    assert_eq!(
        public
            .payment_links()
            .public_view(&created.link_id)
            .await
            .unwrap()
            .amount_mode,
        oblodai::AmountMode::Fixed
    );
    assert!(!public
        .payment_links()
        .checkout(
            &created.link_id,
            LinkCheckoutRequest {
                currency: Some("USDT".into()),
                network: Some("tron".into()),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .uuid
        .is_empty());
    assert!(
        !client
            .payment_links()
            .toggle(&created.link_id, false)
            .await
            .unwrap()
            .active
    );

    // --- splits & settings ---
    let rule = client
        .splits()
        .create_rule(SplitRuleRequest {
            percent: "10".into(),
            address: Some(ADDRESS.into()),
            network: Some("tron".into()),
            note: Some("partner".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(client
        .splits()
        .list_rules(Default::default())
        .await
        .unwrap()
        .items
        .iter()
        .any(|r| r.rule_id == rule.rule_id));
    assert_eq!(
        client
            .splits()
            .set_config(SplitConfigSetRequest {
                refund_hold_seconds: 3600
            })
            .await
            .unwrap()
            .refund_hold_seconds,
        3600
    );
    assert_eq!(
        client
            .splits()
            .get_config()
            .await
            .unwrap()
            .refund_hold_seconds,
        3600
    );
    assert!(client.splits().set_opt_in(true).await.unwrap().enabled);
    assert!(client.splits().get_opt_in().await.unwrap().enabled);
    assert!(client.splits().delete_rule(&rule.rule_id).await.unwrap().ok);

    assert_eq!(
        client
            .settings()
            .set_discount(PaymentDiscountSetRequest {
                currency: Some("USDT".into()),
                network: Some("tron".into()),
                discount_percent: 2,
            })
            .await
            .unwrap()
            .discount_percent,
        2
    );
    assert!(!client
        .settings()
        .list_discounts(Default::default())
        .await
        .unwrap()
        .items
        .is_empty());
    assert!(
        client
            .settings()
            .set_accuracy(PaymentAccuracySetRequest {
                enabled: true,
                accuracy_percent: Some(2)
            })
            .await
            .unwrap()
            .enabled
    );
    assert!(client.settings().get_accuracy().await.unwrap().enabled);
    assert!(
        client
            .settings()
            .set_auto_refund(PaymentAutorefundSetRequest {
                overpay: true,
                underpay: false
            })
            .await
            .unwrap()
            .overpay
    );
    client.settings().get_auto_refund().await.unwrap();
    assert!(
        client
            .settings()
            .set_accepted(PaymentAcceptedSetRequest {
                accepted: vec![PaymentAcceptedSetAcceptedItem {
                    currency: "USDT".into(),
                    network: "tron".into(),
                }],
            })
            .await
            .unwrap()
            .ok
    );
    client
        .settings()
        .list_accepted(Default::default())
        .await
        .unwrap();
    assert_eq!(
        client
            .settings()
            .set_payment_fee_config(PaymentFeeConfigSetRequest {
                payer_pays_percent: 50
            })
            .await
            .unwrap()
            .payer_pays_percent,
        50
    );
    assert_eq!(
        client
            .settings()
            .get_payment_fee_config()
            .await
            .unwrap()
            .payer_pays_percent,
        50
    );
    assert!(!client
        .settings()
        .set_auto_withdraw(AutoWithdrawSetRequest {
            currency: "USDT".into(),
            network: "tron".into(),
            address: ADDRESS.into(),
            min_amount: Some("100".into()),
        })
        .items()
        .await
        .unwrap()
        .is_empty());
    client
        .settings()
        .list_auto_withdraw()
        .items()
        .await
        .unwrap();
    client
        .settings()
        .delete_auto_withdraw("USDT")
        .items()
        .await
        .unwrap();
    assert!(client
        .settings()
        .add_api_allowlist("203.0.113.0/24")
        .await
        .unwrap()
        .items
        .contains(&"203.0.113.0/24".to_string()));
    assert!(client
        .settings()
        .list_api_allowlist()
        .await
        .unwrap()
        .items
        .contains(&"203.0.113.0/24".to_string()));
    assert!(
        !client
            .settings()
            .enable_api_allowlist(false)
            .await
            .unwrap()
            .enabled
    );
    assert!(!client
        .settings()
        .remove_api_allowlist("203.0.113.0/24")
        .await
        .unwrap()
        .items
        .contains(&"203.0.113.0/24".to_string()));

    // --- webhooks & the sandbox inspector ---
    let endpoint = client.webhooks().register(&hook).await.unwrap();
    assert!(!endpoint.endpoint_id.is_empty());
    assert!(!client
        .webhooks()
        .rotate_secret()
        .await
        .unwrap()
        .secret
        .is_empty());
    client
        .webhooks()
        .deliveries(Default::default())
        .limit(5)
        .await
        .unwrap();
    accept(
        client
            .webhooks()
            .test(
                oblodai::WebhookKind::Payment,
                TestWebhookPaymentRequest {
                    url_callback: hook.clone(),
                    currency: Some("USDT".into()),
                    network: Some("tron".into()),
                    status: Some(oblodai::PaymentStatus::Paid),
                    ..Default::default()
                },
            )
            .await,
        "webhooks.test",
    );
    #[allow(deprecated)]
    accept(
        client
            .webhooks()
            .test_legacy(PaymentTestingWebhookRequest {
                url: Some(hook.clone()),
                status: Some(oblodai::PaymentStatus::Paid),
            })
            .await,
        "webhooks.test_legacy",
    );
    let inspector = client
        .sandbox()
        .webhooks(PageParams::default())
        .limit(5)
        .await
        .unwrap();
    if let Some(terminal) = inspector.items.iter().find(|d| {
        matches!(
            d.status,
            oblodai::DeliveryStatus::Delivered | oblodai::DeliveryStatus::Dead
        )
    }) {
        accept(
            client.sandbox().replay(&terminal.id).await,
            "sandbox.replay",
        );
    }

    // --- wallets and transfers (a dev store refuses these by design) ---
    accept(
        client
            .wallets()
            .create(WalletRequest {
                currency: "USDT".into(),
                network: "tron".into(),
                order_id: Some(format!("sw-w-{}", stamp())),
            })
            .await,
        "wallets.create",
    );
    accept(client.wallets().qr(ADDRESS).await, "wallets.qr");
    accept(
        client
            .wallets()
            .block(WalletBlockRequest {
                address: ADDRESS.into(),
                ..Default::default()
            })
            .await,
        "wallets.block",
    );
    accept(
        client
            .transfers()
            .to_personal(TransferToPersonalRequest {
                amount: "1".into(),
                currency: "USDT".into(),
                ..Default::default()
            })
            .await,
        "transfers.to_personal",
    );

    // --- documents, when the stand has a renderer ---
    if documents_enabled {
        let statement = client
            .documents()
            .statement(oblodai::resources::PeriodQuery {
                from: Some("2026-01-01".into()),
                to: Some("2026-12-31".into()),
                lang: Some("en".into()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert!(
            statement.content_type.contains("pdf"),
            "{}",
            statement.content_type
        );
        assert!(!statement.bytes.is_empty());
        assert!(!client
            .documents()
            .fee_schedule(Default::default())
            .await
            .unwrap()
            .bytes
            .is_empty());
        client
            .documents()
            .ledger(oblodai::resources::PeriodQuery {
                format: Some(oblodai::resources::DocumentFormat::Csv),
                ..Default::default()
            })
            .await
            .unwrap();
        accept(
            client
                .payout_links()
                .cheque(PayoutLinkChequeRequest {
                    claim_token: claim_token.clone(),
                    lang: Some("en".into()),
                })
                .await,
            "payout_links.cheque",
        );
        let job = client
            .documents()
            .create_job(DocumentsJobsRequest {
                kind: "statement".into(),
                format: Some("csv".into()),
                lang: Some("en".into()),
                from: Some("2026-01-01".into()),
                to: Some("2026-08-25".into()),
            })
            .await
            .unwrap();
        assert_eq!(
            client
                .documents()
                .job_info(&job.job_id)
                .await
                .unwrap()
                .job_id,
            job.job_id
        );
        accept(
            client.documents().job_file(&job.job_id).await,
            "documents.job_file",
        );

        // A signed public document link, fetched the way a customer's browser would.
        let with_url = client.payments().get(&invoice.uuid).await.unwrap();
        let parsed = url::Url::parse(&with_url.document_url).unwrap();
        let segments: Vec<_> = parsed.path_segments().unwrap().collect();
        let exp: i64 = parsed
            .query_pairs()
            .find(|(k, _)| k == "exp")
            .and_then(|(_, v)| v.parse().ok())
            .unwrap_or(0);
        let sig = parsed
            .query_pairs()
            .find(|(k, _)| k == "sig")
            .map(|(_, v)| v.into_owned())
            .unwrap_or_default();
        let document = public
            .documents()
            .download(segments[2], segments[3], SignedLinkQuery::new(exp, sig))
            .await
            .unwrap();
        assert!(document.content_type.contains("pdf"));
    } else {
        eprintln!("documents are disabled on this stand — skipped");
    }

    // --- and put the sandbox back where it started ---
    client.sandbox().reset().await.unwrap();
}
