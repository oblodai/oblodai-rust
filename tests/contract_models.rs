//! Wire models versus the golden bodies the core recorded.
//!
//! Each row names a route, where inside its `result` the object sits, and the model it must decode
//! into. The check is a round trip: decode the recorded body, encode it again, and require the two
//! to be identical. A field the core stopped sending fails here (it is missing on the way in), and
//! so does a field it started sending that the model does not carry (it vanishes on the way out).

mod support;

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use oblodai::contract::models::*;
use oblodai::{
    Page, PlainList, DELIVERY_STATUSES, ERROR_CODES, EVENT_TYPES, PAYMENT_STATUSES,
    PAYOUT_LINK_STATUSES, PAYOUT_STATUSES,
};
use support::{load_contract, load_error_samples, load_fixtures, load_webhook_samples, pick};

type Check = fn(&Value) -> Result<Value, String>;

fn round_trip<T: DeserializeOwned + Serialize>(value: &Value) -> Result<Value, String> {
    let model: T = serde_json::from_value(value.clone()).map_err(|e| format!("decode: {e}"))?;
    serde_json::to_value(&model).map_err(|e| format!("encode: {e}"))
}

macro_rules! rows {
    ($( $route:literal @ $path:literal => $ty:ty ),* $(,)?) => {
        vec![ $( ($route, $path, round_trip::<$ty> as Check) ),* ]
    };
}

/// Routes the API guarantees to refuse for API keys, so no success body exists to model.
const NOT_MODELLED: [&str; 1] = [
    // API-key payouts auto-approve; approve serves the cabinet's maker-checker flow.
    "POST /v1/payout/approve",
];

fn model_rows() -> Vec<(&'static str, &'static str, Check)> {
    rows![
        "POST /v1/payment" @ "" => Payment,
        "POST /v1/payment/info" @ "" => Payment,
        "POST /v1/payment/cancel" @ "" => Payment,
        "POST /v1/payment/history" @ "" => Page<Payment>,
        "GET /v1/pay/{id}" @ "" => PublicPayment,
        "POST /v1/pay/{id}/select" @ "" => PublicPayment,
        "POST /v1/link/{id}/checkout" @ "" => PublicPayment,
        "POST /v1/payment/qr" @ "" => QrCode,
        "GET /v1/pay/{id}/qr" @ "" => QrCode,
        "POST /v1/payment/services" @ "" => Page<ServiceMethod>,
        "POST /v1/payout/services" @ "" => Page<ServiceMethod>,
        "POST /v1/payment/batch" @ "" => BatchSubmitted,
        "POST /v1/payout/batch" @ "" => BatchSubmitted,
        "POST /v1/refund/batch" @ "" => BatchSubmitted,
        "POST /v1/transfer/batch" @ "" => BatchSubmitted,
        "POST /v1/batch/info" @ "" => BatchInfo,
        "POST /v1/payout" @ "" => Payout,
        "POST /v1/payout/info" @ "" => Payout,
        "POST /v1/payout/cancel" @ "" => Payout,
        "POST /v1/payout/history" @ "" => Page<Payout>,
        "POST /v1/payout/mass" @ "" => PlainList<BatchElement<Payout>>,
        "POST /v1/payment/refund" @ "" => Payout,
        "POST /v1/payout/calculate" @ "" => PayoutCalculation,
        "POST /v1/payout/validate" @ "" => PayoutValidation,
        "POST /v1/payout/link" @ "" => PayoutLink,
        "POST /v1/payout/link/info" @ "" => PayoutLink,
        "POST /v1/payout/link/list" @ "" => Page<PayoutLink>,
        "POST /v1/payout/link/cancel" @ "" => PayoutLink,
        "POST /v1/payout/link/batch" @ "" => PlainList<BatchElement<PayoutLink>>,
        "GET /v1/claim/{token}" @ "" => ClaimPreview,
        "POST /v1/claim/{token}" @ "" => ClaimResult,
        "POST /v1/payment/link" @ "" => PaymentLinkCreated,
        "POST /v1/payment/link/info" @ "" => PaymentLink,
        "POST /v1/payment/link/list" @ "" => Page<PaymentLink>,
        "GET /v1/link/{id}" @ "" => PublicPaymentLink,
        "POST /v1/balance" @ "" => Balance,
        "POST /v1/referral/info" @ "" => ReferralInfo,
        "POST /v1/auto-withdraw/list" @ "" => PlainList<AutoWithdrawRule>,
        "POST /v1/auto-withdraw/set" @ "" => PlainList<AutoWithdrawRule>,
        "POST /v1/auto-withdraw/delete" @ "" => PlainList<AutoWithdrawRule>,
        "POST /v1/api-allowlist/list" @ "" => ApiAllowlist,
        "POST /v1/api-allowlist/add" @ "" => ApiAllowlist,
        "POST /v1/api-allowlist/remove" @ "" => ApiAllowlist,
        "POST /v1/api-allowlist/enable" @ "" => ApiAllowlist,
        "POST /v1/payment/discount/list" @ "" => Page<DiscountRule>,
        "POST /v1/payment/discount/set" @ "" => DiscountRule,
        "POST /v1/split/rule" @ "" => SplitRule,
        "POST /v1/split/rule/list" @ "" => Page<SplitRule>,
        "POST /v1/split/rule/delete" @ "" => OkResult,
        "POST /v1/split/config/get" @ "" => SplitConfig,
        "POST /v1/split/config/set" @ "" => SplitConfig,
        "POST /v1/split/recipient/optin" @ "" => SplitOptIn,
        "POST /v1/split/recipient/optin/get" @ "" => SplitOptIn,
        "GET /v1/currencies" @ "" => Currencies,
        "POST /v1/exchange-rate/list" @ "" => Page<ExchangeRate>,
        "POST /v1/webhooks" @ "" => WebhookEndpoint,
        "POST /v1/webhooks/rotate-secret" @ "" => WebhookSecretRotated,
        "POST /v1/webhooks/deliveries" @ "" => Page<WebhookDelivery>,
        "GET /v1/sandbox/webhooks" @ "" => Page<WebhookDelivery>,
        "POST /v1/payment/resolve" @ "" => Resolution,
        "POST /v1/payment/send-email" @ "" => EmailSent,
        "POST /v1/payment/resend" @ "" => OkResult,
        "POST /v1/payment/accepted/set" @ "" => OkResult,
        "POST /v1/payment/accepted/list" @ "" => Page<AcceptedMethod>,
        "POST /v1/payment/accuracy/get" @ "" => AccuracyConfig,
        "POST /v1/payment/accuracy/set" @ "" => AccuracyConfig,
        "POST /v1/payment/autorefund/get" @ "" => AutoRefundConfig,
        "POST /v1/payment/autorefund/set" @ "" => AutoRefundConfig,
        "POST /v1/payment/fee-config/get" @ "" => PaymentFeeConfig,
        "POST /v1/payment/fee-config/set" @ "" => PaymentFeeConfig,
        "POST /v1/payout/fee-config/get" @ "" => PayoutFeeConfig,
        "POST /v1/payout/fee-config/set" @ "" => PayoutFeeConfig,
        "POST /v1/payout/refund-fee-config/get" @ "" => RefundFeeConfig,
        "POST /v1/payout/refund-fee-config/set" @ "" => RefundFeeConfig,
        "POST /v1/payment/link/toggle" @ "" => PaymentLinkToggled,
        "POST /v1/vrcs" @ "" => VrcsStatus,
        "POST /v1/wallet" @ "" => Wallet,
        "POST /v1/wallet/block" @ "" => WalletBlocked,
        "POST /v1/wallet/qr" @ "" => WalletQr,
        "POST /v1/wallet/blocked-address-refund" @ "" => Payout,
        "POST /v1/transfer/to-personal" @ "" => TransferToPersonal,
        "POST /v1/transfer/to-user" @ "" => TransferToUser,
        "POST /v1/documents/jobs" @ "" => DocumentJob,
        "POST /v1/documents/jobs/info" @ "" => DocumentJob,
        "POST /v1/test-webhook/payment" @ "" => WebhookTestResult,
        "POST /v1/test-webhook/payout" @ "" => WebhookTestResult,
        "POST /v1/test-webhook/wallet" @ "" => WebhookTestResult,
        "POST /v1/payment/testing-webhook" @ "" => WebhookTestResult,
        "POST /v1/sandbox/faucet" @ "" => FaucetResult,
        "POST /v1/sandbox/deposit" @ "" => SandboxDeposit,
        "POST /v1/sandbox/reset" @ "" => SandboxReset,
        "POST /v1/sandbox/webhooks/replay" @ "" => SandboxReplay,
        "POST /v1/merchants" @ "" => MerchantOnboarded,
        "POST /v1/merchants/{id}/sandbox" @ "" => SandboxStore,
        // A few nested shapes, named explicitly so a change inside them reads clearly.
        "GET /v1/currencies" @ "currencies.0" => CurrencyInfo,
        "GET /v1/currencies" @ "currencies.0.networks.0" => CurrencyNetwork,
        "POST /v1/documents/jobs/info" @ "file" => DocumentFile,
        "POST /v1/payment/history" @ "items.0" => Payment,
        "POST /v1/batch/info" @ "items.0" => BatchInfoItem,
        "POST /v1/merchants" @ "api_key" => ApiKeyPair,
        "POST /v1/payout/mass" @ "items.0.result" => Payout,
    ]
}

fn key_set(value: &Value) -> Vec<String> {
    match value {
        Value::Object(map) => map.keys().cloned().collect(),
        _ => Vec::new(),
    }
}

/// Rows whose recorded body cannot be compared in this snapshot, each with the reason. Anything
/// else that fails to produce a comparison is a hole in the gate, not a waiver — the count below
/// is exact so a row cannot quietly stop being checked.
const UNCOMPARABLE: [&str; 0] = [];

#[test]
fn every_model_round_trips_its_golden_body() {
    let fixtures = load_fixtures();
    let mut checked = 0;
    let mut skipped: Vec<String> = Vec::new();
    for (route, path, check) in model_rows() {
        let Some(fx) = fixtures.iter().find(|f| f.route == route) else {
            panic!("{route}: no fixture recorded");
        };
        if !fx.is_success() {
            // Recorded as a refusal in this environment: nothing to compare.
            skipped.push(format!("{route} @ \"{path}\" (recorded refusal)"));
            continue;
        }
        let result = fx.result();
        let Some(target) = pick(&result, path) else {
            panic!("{route}: nothing at \"{path}\" in the recorded result");
        };
        if target.is_null() {
            skipped.push(format!("{route} @ \"{path}\" (null in the recorded body)"));
            continue;
        }
        let encoded = check(target).unwrap_or_else(|e| panic!("{route} @ \"{path}\": {e}"));
        if &encoded != target {
            let on_wire = key_set(target);
            let in_model = key_set(&encoded);
            let missing: Vec<_> = on_wire.iter().filter(|k| !in_model.contains(k)).collect();
            let invented: Vec<_> = in_model.iter().filter(|k| !on_wire.contains(k)).collect();
            panic!(
                "{route} @ \"{path}\": the model drifted from the wire\n  \
                 fields the model does not carry: {missing:?}\n  \
                 fields the model invents: {invented:?}\n  \
                 wire:  {target}\n  model: {encoded}"
            );
        }
        checked += 1;
    }
    // Exact, not a floor: `checked + skipped == rows`, and every skip is named.
    assert_eq!(
        checked + skipped.len(),
        model_rows().len(),
        "a row neither compared nor skipped"
    );
    assert_eq!(
        skipped.len(),
        UNCOMPARABLE.len(),
        "rows that stopped being verified: {skipped:?}"
    );
    assert_eq!(
        checked,
        model_rows().len() - UNCOMPARABLE.len(),
        "only {checked} of {} models were checked",
        model_rows().len()
    );
}

#[test]
fn every_recorded_success_body_is_covered_by_a_model_row() {
    let rows = model_rows();
    for fx in load_fixtures() {
        if !fx.is_success() || NOT_MODELLED.contains(&fx.route.as_str()) || !fx.is_json() {
            continue;
        }
        assert!(
            rows.iter().any(|(route, _, _)| *route == fx.route),
            "{}: a recorded success body with no model row",
            fx.route
        );
    }
}

#[test]
fn every_fixture_belongs_to_a_declared_route() {
    for fx in load_fixtures() {
        assert!(
            oblodai::ROUTES.iter().any(|r| r.key == fx.route),
            "{}: recorded but not declared",
            fx.route
        );
    }
}

#[test]
fn statuses_on_the_wire_are_in_the_vocabulary() {
    let payments = support::result_of("POST /v1/payment/history");
    for item in payments["items"].as_array().unwrap() {
        let status = item["status"].as_str().unwrap();
        assert!(
            PAYMENT_STATUSES.contains(&status),
            "unknown payment status {status}"
        );
    }
    let payouts = support::result_of("POST /v1/payout/history");
    for item in payouts["items"].as_array().unwrap() {
        let status = item["status"].as_str().unwrap();
        assert!(
            PAYOUT_STATUSES.contains(&status),
            "unknown payout status {status}"
        );
    }
    for item in support::result_of("POST /v1/payout/link/list")["items"]
        .as_array()
        .unwrap()
    {
        let status = item["status"].as_str().unwrap();
        assert!(
            PAYOUT_LINK_STATUSES.contains(&status),
            "unknown link status {status}"
        );
    }
    for item in support::result_of("POST /v1/webhooks/deliveries")["items"]
        .as_array()
        .unwrap()
    {
        let status = item["status"].as_str().unwrap();
        assert!(
            DELIVERY_STATUSES.contains(&status),
            "unknown delivery status {status}"
        );
    }
}

#[test]
fn webhook_samples_carry_known_event_types() {
    let samples = load_webhook_samples();
    assert!(!samples.is_empty());
    for sample in samples {
        let event_type = sample.header("X-Webhook-Event").unwrap();
        assert!(
            EVENT_TYPES.contains(&event_type),
            "unknown event type {event_type}"
        );
    }
}

#[test]
fn every_recorded_error_is_a_known_code_with_the_documented_envelope() {
    let samples = load_error_samples();
    assert!(!samples.is_empty());
    for (code, fx) in samples {
        assert!(
            ERROR_CODES.contains(&code.as_str()),
            "{code} is not in the contract vocabulary"
        );
        let error = &fx.response["error"];
        assert_eq!(error["code"].as_str(), Some(code.as_str()));
        assert!(
            error["retryable"].is_boolean(),
            "{code}: retryable must be a boolean"
        );
        assert!(
            error["request_id"].is_string(),
            "{code}: no request id to quote to support"
        );
        if fx.status == 429 {
            assert!(
                error["retry_after"].as_i64().unwrap_or(0) > 0,
                "{code}: 429 needs a hint"
            );
        }
        // And the SDK must classify it the way the envelope describes it.
        let decoded = oblodai::core::envelope::decode_envelope(
            fx.status,
            fx.response.to_string().as_bytes(),
            None,
            None,
        )
        .unwrap_err();
        assert_eq!(decoded.code(), code);
        assert_eq!(decoded.retryable(), error["retryable"].as_bool().unwrap());
        assert!(!decoded.synthetic());
        assert_eq!(decoded.http_status(), fx.status);
    }
}

#[test]
fn recorded_request_bodies_only_use_documented_fields() {
    let contract = load_contract();
    for fx in load_fixtures() {
        let Some(route) = contract["routes"].as_array().unwrap().iter().find(|r| {
            format!(
                "{} {}",
                r["method"].as_str().unwrap(),
                r["path"].as_str().unwrap()
            ) == fx.route
        }) else {
            continue;
        };
        let Some(properties) = route
            .get("request_schema")
            .and_then(|s| s.get("properties"))
        else {
            continue;
        };
        let Value::Object(sent) = &fx.request else {
            continue;
        };
        for field in sent.keys() {
            assert!(
                properties.get(field).is_some(),
                "{}: the recorded journey sent an undocumented field \"{field}\"",
                fx.route
            );
        }
    }
}

#[test]
fn the_contract_snapshot_the_code_was_generated_from_is_the_one_that_ships() {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(support::contract_dir().join("contract.json")).unwrap();
    let digest = format!("{:x}", Sha256::digest(&bytes));
    assert_eq!(
        digest,
        oblodai::CONTRACT_HASH,
        "contract/contract.json changed — run python3 scripts/codegen.py"
    );
    let contract = load_contract();
    assert_eq!(
        contract["core_commit"].as_str().unwrap(),
        oblodai::CONTRACT_CORE_COMMIT
    );
}

/// Every `family.reason` token a method's rustdoc names must be a real code in the contract's
/// catalogue.
///
/// A doc that invites `match err.code()` on a code the gateway never emits sends the caller down a
/// branch that can never run — the reference SDK documented `wallet.blocked`, which does not exist.
/// The scan is deliberately blunt: anything shaped like a code inside backticks counts, unless it is
/// an SDK-raised code, a transport code, a webhook code or one of the named non-code identifiers
/// below.
#[test]
fn every_error_code_named_in_a_doc_comment_exists_in_the_catalogue() {
    /// Dotted identifiers that appear in docs and are NOT error codes.
    const NOT_A_CODE: [&str; 12] = [
        "wallet.paid",      // an event type
        "invoice.paid",     // an event type prefix
        "payout.confirmed", // an event type
        "webhooks.test",    // an SDK method
        "payouts.info",     // an SDK method
        "family.reason",    // the shape of a code, not a code
        "paginate.has_pages",
        "paginate.total",
        "payment.status",          // a model field
        "payout.status",           // a model field
        "payout.history",          // a route
        "api_conformance_test.go", // a file in the core
    ];
    let catalogue: std::collections::BTreeSet<&str> = ERROR_CODES.iter().copied().collect();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs(&root, &mut files);
    assert!(files.len() > 20, "the source tree was not walked");

    let mut unknown: Vec<String> = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).unwrap();
        for line in text.lines() {
            let line = line.trim_start();
            if !line.starts_with("///") && !line.starts_with("//!") {
                continue;
            }
            for token in backticked(line) {
                if !is_code_shaped(&token)
                    || token.starts_with("sdk.")
                    || token.starts_with("transport.")
                    || token.starts_with("webhook.")
                    || NOT_A_CODE.contains(&token.as_str())
                {
                    continue;
                }
                if !catalogue.contains(token.as_str()) {
                    unknown.push(format!("{}: {token}", file.display()));
                }
            }
        }
    }
    assert!(
        unknown.is_empty(),
        "documented codes that are not in contract/contract.json: {unknown:#?}"
    );

    // Proof the scan bites: this is exactly the code the reference SDK documented by mistake, and
    // the rule above both recognises its shape and rejects it.
    assert!(is_code_shaped("wallet.blocked"));
    assert!(
        !catalogue.contains("wallet.blocked"),
        "`blocked` is a wallet MODEL field, never an error code"
    );
    assert!(
        is_code_shaped("payout.insufficient_funds")
            && catalogue.contains("payout.insufficient_funds")
    );
}

fn collect_rs(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn backticked(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(open) = rest.find('`') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find('`') else { break };
        out.push(rest[..close].to_string());
        rest = &rest[close + 1..];
    }
    out
}

fn is_code_shaped(token: &str) -> bool {
    let Some((family, reason)) = token.split_once('.') else {
        return false;
    };
    if reason.contains('.') || family.is_empty() || reason.is_empty() {
        return false;
    }
    let ok = |s: &str| s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_');
    ok(family) && ok(reason)
}
