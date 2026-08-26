// GENERATED FILE - do not edit. Source: contract/contract.json (core bfca971cce71).
// Regenerate with: python3 scripts/codegen.py

use super::types::{ListKind, Method, RouteAuth, RouteSpec};

/// `POST /v1/api-allowlist/add`
pub static POST_V1_API_ALLOWLIST_ADD: RouteSpec = RouteSpec {
    key: "POST /v1/api-allowlist/add",
    method: Method::Post,
    path: "/v1/api-allowlist/add",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: false,
    bare: false,
    list: Some(ListKind::Plain),
};

/// `POST /v1/api-allowlist/enable`
pub static POST_V1_API_ALLOWLIST_ENABLE: RouteSpec = RouteSpec {
    key: "POST /v1/api-allowlist/enable",
    method: Method::Post,
    path: "/v1/api-allowlist/enable",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: false,
    bare: false,
    list: Some(ListKind::Plain),
};

/// `POST /v1/api-allowlist/list`
pub static POST_V1_API_ALLOWLIST_LIST: RouteSpec = RouteSpec {
    key: "POST /v1/api-allowlist/list",
    method: Method::Post,
    path: "/v1/api-allowlist/list",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: true,
    bare: false,
    list: Some(ListKind::Plain),
};

/// `POST /v1/api-allowlist/remove`
pub static POST_V1_API_ALLOWLIST_REMOVE: RouteSpec = RouteSpec {
    key: "POST /v1/api-allowlist/remove",
    method: Method::Post,
    path: "/v1/api-allowlist/remove",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: false,
    bare: false,
    list: Some(ListKind::Plain),
};

/// `POST /v1/auto-withdraw/delete`
pub static POST_V1_AUTO_WITHDRAW_DELETE: RouteSpec = RouteSpec {
    key: "POST /v1/auto-withdraw/delete",
    method: Method::Post,
    path: "/v1/auto-withdraw/delete",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/auto-withdraw/list`
pub static POST_V1_AUTO_WITHDRAW_LIST: RouteSpec = RouteSpec {
    key: "POST /v1/auto-withdraw/list",
    method: Method::Post,
    path: "/v1/auto-withdraw/list",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: true,
    bare: false,
    list: Some(ListKind::Plain),
};

/// `POST /v1/auto-withdraw/set`
pub static POST_V1_AUTO_WITHDRAW_SET: RouteSpec = RouteSpec {
    key: "POST /v1/auto-withdraw/set",
    method: Method::Post,
    path: "/v1/auto-withdraw/set",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/balance`
pub static POST_V1_BALANCE: RouteSpec = RouteSpec {
    key: "POST /v1/balance",
    method: Method::Post,
    path: "/v1/balance",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `POST /v1/batch/info`
pub static POST_V1_BATCH_INFO: RouteSpec = RouteSpec {
    key: "POST /v1/batch/info",
    method: Method::Post,
    path: "/v1/batch/info",
    auth: RouteAuth::Any,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `GET /v1/claim/{token}`
pub static GET_V1_CLAIM_TOKEN: RouteSpec = RouteSpec {
    key: "GET /v1/claim/{token}",
    method: Method::Get,
    path: "/v1/claim/{token}",
    auth: RouteAuth::Public,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `POST /v1/claim/{token}`
pub static POST_V1_CLAIM_TOKEN: RouteSpec = RouteSpec {
    key: "POST /v1/claim/{token}",
    method: Method::Post,
    path: "/v1/claim/{token}",
    auth: RouteAuth::Public,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `GET /v1/currencies`
pub static GET_V1_CURRENCIES: RouteSpec = RouteSpec {
    key: "GET /v1/currencies",
    method: Method::Get,
    path: "/v1/currencies",
    auth: RouteAuth::Public,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `GET /v1/documents/balance`
pub static GET_V1_DOCUMENTS_BALANCE: RouteSpec = RouteSpec {
    key: "GET /v1/documents/balance",
    method: Method::Get,
    path: "/v1/documents/balance",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: true,
    list: None,
};

/// `GET /v1/documents/batch`
pub static GET_V1_DOCUMENTS_BATCH: RouteSpec = RouteSpec {
    key: "GET /v1/documents/batch",
    method: Method::Get,
    path: "/v1/documents/batch",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: true,
    list: None,
};

/// `GET /v1/documents/fees`
pub static GET_V1_DOCUMENTS_FEES: RouteSpec = RouteSpec {
    key: "GET /v1/documents/fees",
    method: Method::Get,
    path: "/v1/documents/fees",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: true,
    list: None,
};

/// `POST /v1/documents/jobs`
pub static POST_V1_DOCUMENTS_JOBS: RouteSpec = RouteSpec {
    key: "POST /v1/documents/jobs",
    method: Method::Post,
    path: "/v1/documents/jobs",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `GET /v1/documents/jobs/file`
pub static GET_V1_DOCUMENTS_JOBS_FILE: RouteSpec = RouteSpec {
    key: "GET /v1/documents/jobs/file",
    method: Method::Get,
    path: "/v1/documents/jobs/file",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: true,
    list: None,
};

/// `POST /v1/documents/jobs/info`
pub static POST_V1_DOCUMENTS_JOBS_INFO: RouteSpec = RouteSpec {
    key: "POST /v1/documents/jobs/info",
    method: Method::Post,
    path: "/v1/documents/jobs/info",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `GET /v1/documents/ledger`
pub static GET_V1_DOCUMENTS_LEDGER: RouteSpec = RouteSpec {
    key: "GET /v1/documents/ledger",
    method: Method::Get,
    path: "/v1/documents/ledger",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: true,
    list: None,
};

/// `GET /v1/documents/link`
pub static GET_V1_DOCUMENTS_LINK: RouteSpec = RouteSpec {
    key: "GET /v1/documents/link",
    method: Method::Get,
    path: "/v1/documents/link",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: true,
    list: None,
};

/// `GET /v1/documents/referrals`
pub static GET_V1_DOCUMENTS_REFERRALS: RouteSpec = RouteSpec {
    key: "GET /v1/documents/referrals",
    method: Method::Get,
    path: "/v1/documents/referrals",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: true,
    list: None,
};

/// `GET /v1/documents/split`
pub static GET_V1_DOCUMENTS_SPLIT: RouteSpec = RouteSpec {
    key: "GET /v1/documents/split",
    method: Method::Get,
    path: "/v1/documents/split",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: true,
    list: None,
};

/// `GET /v1/documents/statement`
pub static GET_V1_DOCUMENTS_STATEMENT: RouteSpec = RouteSpec {
    key: "GET /v1/documents/statement",
    method: Method::Get,
    path: "/v1/documents/statement",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: true,
    list: None,
};

/// `GET /v1/documents/wallet/statement`
pub static GET_V1_DOCUMENTS_WALLET_STATEMENT: RouteSpec = RouteSpec {
    key: "GET /v1/documents/wallet/statement",
    method: Method::Get,
    path: "/v1/documents/wallet/statement",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: true,
    list: None,
};

/// `GET /v1/documents/{kind}/{id}`
pub static GET_V1_DOCUMENTS_KIND_ID: RouteSpec = RouteSpec {
    key: "GET /v1/documents/{kind}/{id}",
    method: Method::Get,
    path: "/v1/documents/{kind}/{id}",
    auth: RouteAuth::Public,
    idempotent: false,
    safe: true,
    bare: true,
    list: None,
};

/// `POST /v1/exchange-rate/list`
pub static POST_V1_EXCHANGE_RATE_LIST: RouteSpec = RouteSpec {
    key: "POST /v1/exchange-rate/list",
    method: Method::Post,
    path: "/v1/exchange-rate/list",
    auth: RouteAuth::Public,
    idempotent: false,
    safe: true,
    bare: false,
    list: Some(ListKind::Paged),
};

/// `GET /v1/link/{id}`
pub static GET_V1_LINK_ID: RouteSpec = RouteSpec {
    key: "GET /v1/link/{id}",
    method: Method::Get,
    path: "/v1/link/{id}",
    auth: RouteAuth::Public,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `POST /v1/link/{id}/checkout`
pub static POST_V1_LINK_ID_CHECKOUT: RouteSpec = RouteSpec {
    key: "POST /v1/link/{id}/checkout",
    method: Method::Post,
    path: "/v1/link/{id}/checkout",
    auth: RouteAuth::Public,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/merchants`
pub static POST_V1_MERCHANTS: RouteSpec = RouteSpec {
    key: "POST /v1/merchants",
    method: Method::Post,
    path: "/v1/merchants",
    auth: RouteAuth::Onboard,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/merchants/{id}/sandbox`
pub static POST_V1_MERCHANTS_ID_SANDBOX: RouteSpec = RouteSpec {
    key: "POST /v1/merchants/{id}/sandbox",
    method: Method::Post,
    path: "/v1/merchants/{id}/sandbox",
    auth: RouteAuth::Onboard,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `GET /v1/pay/{id}`
pub static GET_V1_PAY_ID: RouteSpec = RouteSpec {
    key: "GET /v1/pay/{id}",
    method: Method::Get,
    path: "/v1/pay/{id}",
    auth: RouteAuth::Public,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `GET /v1/pay/{id}/qr`
pub static GET_V1_PAY_ID_QR: RouteSpec = RouteSpec {
    key: "GET /v1/pay/{id}/qr",
    method: Method::Get,
    path: "/v1/pay/{id}/qr",
    auth: RouteAuth::Public,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `POST /v1/pay/{id}/select`
pub static POST_V1_PAY_ID_SELECT: RouteSpec = RouteSpec {
    key: "POST /v1/pay/{id}/select",
    method: Method::Post,
    path: "/v1/pay/{id}/select",
    auth: RouteAuth::Public,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payment`
pub static POST_V1_PAYMENT: RouteSpec = RouteSpec {
    key: "POST /v1/payment",
    method: Method::Post,
    path: "/v1/payment",
    auth: RouteAuth::Payment,
    idempotent: true,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payment/accepted/list`
pub static POST_V1_PAYMENT_ACCEPTED_LIST: RouteSpec = RouteSpec {
    key: "POST /v1/payment/accepted/list",
    method: Method::Post,
    path: "/v1/payment/accepted/list",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: false,
    list: Some(ListKind::Paged),
};

/// `POST /v1/payment/accepted/set`
pub static POST_V1_PAYMENT_ACCEPTED_SET: RouteSpec = RouteSpec {
    key: "POST /v1/payment/accepted/set",
    method: Method::Post,
    path: "/v1/payment/accepted/set",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payment/accuracy/get`
pub static POST_V1_PAYMENT_ACCURACY_GET: RouteSpec = RouteSpec {
    key: "POST /v1/payment/accuracy/get",
    method: Method::Post,
    path: "/v1/payment/accuracy/get",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `POST /v1/payment/accuracy/set`
pub static POST_V1_PAYMENT_ACCURACY_SET: RouteSpec = RouteSpec {
    key: "POST /v1/payment/accuracy/set",
    method: Method::Post,
    path: "/v1/payment/accuracy/set",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payment/autorefund/get`
pub static POST_V1_PAYMENT_AUTOREFUND_GET: RouteSpec = RouteSpec {
    key: "POST /v1/payment/autorefund/get",
    method: Method::Post,
    path: "/v1/payment/autorefund/get",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `POST /v1/payment/autorefund/set`
pub static POST_V1_PAYMENT_AUTOREFUND_SET: RouteSpec = RouteSpec {
    key: "POST /v1/payment/autorefund/set",
    method: Method::Post,
    path: "/v1/payment/autorefund/set",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payment/batch`
pub static POST_V1_PAYMENT_BATCH: RouteSpec = RouteSpec {
    key: "POST /v1/payment/batch",
    method: Method::Post,
    path: "/v1/payment/batch",
    auth: RouteAuth::Payment,
    idempotent: true,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payment/cancel`
pub static POST_V1_PAYMENT_CANCEL: RouteSpec = RouteSpec {
    key: "POST /v1/payment/cancel",
    method: Method::Post,
    path: "/v1/payment/cancel",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payment/discount/list`
pub static POST_V1_PAYMENT_DISCOUNT_LIST: RouteSpec = RouteSpec {
    key: "POST /v1/payment/discount/list",
    method: Method::Post,
    path: "/v1/payment/discount/list",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: false,
    list: Some(ListKind::Paged),
};

/// `POST /v1/payment/discount/set`
pub static POST_V1_PAYMENT_DISCOUNT_SET: RouteSpec = RouteSpec {
    key: "POST /v1/payment/discount/set",
    method: Method::Post,
    path: "/v1/payment/discount/set",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payment/fee-config/get`
pub static POST_V1_PAYMENT_FEE_CONFIG_GET: RouteSpec = RouteSpec {
    key: "POST /v1/payment/fee-config/get",
    method: Method::Post,
    path: "/v1/payment/fee-config/get",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `POST /v1/payment/fee-config/set`
pub static POST_V1_PAYMENT_FEE_CONFIG_SET: RouteSpec = RouteSpec {
    key: "POST /v1/payment/fee-config/set",
    method: Method::Post,
    path: "/v1/payment/fee-config/set",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payment/history`
pub static POST_V1_PAYMENT_HISTORY: RouteSpec = RouteSpec {
    key: "POST /v1/payment/history",
    method: Method::Post,
    path: "/v1/payment/history",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: false,
    list: Some(ListKind::Paged),
};

/// `POST /v1/payment/info`
pub static POST_V1_PAYMENT_INFO: RouteSpec = RouteSpec {
    key: "POST /v1/payment/info",
    method: Method::Post,
    path: "/v1/payment/info",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `POST /v1/payment/link`
pub static POST_V1_PAYMENT_LINK: RouteSpec = RouteSpec {
    key: "POST /v1/payment/link",
    method: Method::Post,
    path: "/v1/payment/link",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payment/link/info`
pub static POST_V1_PAYMENT_LINK_INFO: RouteSpec = RouteSpec {
    key: "POST /v1/payment/link/info",
    method: Method::Post,
    path: "/v1/payment/link/info",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `POST /v1/payment/link/list`
pub static POST_V1_PAYMENT_LINK_LIST: RouteSpec = RouteSpec {
    key: "POST /v1/payment/link/list",
    method: Method::Post,
    path: "/v1/payment/link/list",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: false,
    list: Some(ListKind::Paged),
};

/// `POST /v1/payment/link/toggle`
pub static POST_V1_PAYMENT_LINK_TOGGLE: RouteSpec = RouteSpec {
    key: "POST /v1/payment/link/toggle",
    method: Method::Post,
    path: "/v1/payment/link/toggle",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payment/qr`
pub static POST_V1_PAYMENT_QR: RouteSpec = RouteSpec {
    key: "POST /v1/payment/qr",
    method: Method::Post,
    path: "/v1/payment/qr",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `POST /v1/payment/refund`
pub static POST_V1_PAYMENT_REFUND: RouteSpec = RouteSpec {
    key: "POST /v1/payment/refund",
    method: Method::Post,
    path: "/v1/payment/refund",
    auth: RouteAuth::Payout,
    idempotent: true,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payment/resend`
pub static POST_V1_PAYMENT_RESEND: RouteSpec = RouteSpec {
    key: "POST /v1/payment/resend",
    method: Method::Post,
    path: "/v1/payment/resend",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payment/resolve`
pub static POST_V1_PAYMENT_RESOLVE: RouteSpec = RouteSpec {
    key: "POST /v1/payment/resolve",
    method: Method::Post,
    path: "/v1/payment/resolve",
    auth: RouteAuth::Payout,
    idempotent: true,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payment/send-email`
pub static POST_V1_PAYMENT_SEND_EMAIL: RouteSpec = RouteSpec {
    key: "POST /v1/payment/send-email",
    method: Method::Post,
    path: "/v1/payment/send-email",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payment/services`
pub static POST_V1_PAYMENT_SERVICES: RouteSpec = RouteSpec {
    key: "POST /v1/payment/services",
    method: Method::Post,
    path: "/v1/payment/services",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: false,
    list: Some(ListKind::Paged),
};

/// `POST /v1/payment/testing-webhook`
pub static POST_V1_PAYMENT_TESTING_WEBHOOK: RouteSpec = RouteSpec {
    key: "POST /v1/payment/testing-webhook",
    method: Method::Post,
    path: "/v1/payment/testing-webhook",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payout`
pub static POST_V1_PAYOUT: RouteSpec = RouteSpec {
    key: "POST /v1/payout",
    method: Method::Post,
    path: "/v1/payout",
    auth: RouteAuth::Payout,
    idempotent: true,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payout/approve`
pub static POST_V1_PAYOUT_APPROVE: RouteSpec = RouteSpec {
    key: "POST /v1/payout/approve",
    method: Method::Post,
    path: "/v1/payout/approve",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payout/batch`
pub static POST_V1_PAYOUT_BATCH: RouteSpec = RouteSpec {
    key: "POST /v1/payout/batch",
    method: Method::Post,
    path: "/v1/payout/batch",
    auth: RouteAuth::Payout,
    idempotent: true,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payout/calculate`
pub static POST_V1_PAYOUT_CALCULATE: RouteSpec = RouteSpec {
    key: "POST /v1/payout/calculate",
    method: Method::Post,
    path: "/v1/payout/calculate",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `POST /v1/payout/cancel`
pub static POST_V1_PAYOUT_CANCEL: RouteSpec = RouteSpec {
    key: "POST /v1/payout/cancel",
    method: Method::Post,
    path: "/v1/payout/cancel",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payout/fee-config/get`
pub static POST_V1_PAYOUT_FEE_CONFIG_GET: RouteSpec = RouteSpec {
    key: "POST /v1/payout/fee-config/get",
    method: Method::Post,
    path: "/v1/payout/fee-config/get",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `POST /v1/payout/fee-config/set`
pub static POST_V1_PAYOUT_FEE_CONFIG_SET: RouteSpec = RouteSpec {
    key: "POST /v1/payout/fee-config/set",
    method: Method::Post,
    path: "/v1/payout/fee-config/set",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payout/history`
pub static POST_V1_PAYOUT_HISTORY: RouteSpec = RouteSpec {
    key: "POST /v1/payout/history",
    method: Method::Post,
    path: "/v1/payout/history",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: true,
    bare: false,
    list: Some(ListKind::Paged),
};

/// `POST /v1/payout/info`
pub static POST_V1_PAYOUT_INFO: RouteSpec = RouteSpec {
    key: "POST /v1/payout/info",
    method: Method::Post,
    path: "/v1/payout/info",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `POST /v1/payout/link`
pub static POST_V1_PAYOUT_LINK: RouteSpec = RouteSpec {
    key: "POST /v1/payout/link",
    method: Method::Post,
    path: "/v1/payout/link",
    auth: RouteAuth::Payout,
    idempotent: true,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payout/link/batch`
pub static POST_V1_PAYOUT_LINK_BATCH: RouteSpec = RouteSpec {
    key: "POST /v1/payout/link/batch",
    method: Method::Post,
    path: "/v1/payout/link/batch",
    auth: RouteAuth::Payout,
    idempotent: true,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payout/link/cancel`
pub static POST_V1_PAYOUT_LINK_CANCEL: RouteSpec = RouteSpec {
    key: "POST /v1/payout/link/cancel",
    method: Method::Post,
    path: "/v1/payout/link/cancel",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payout/link/cheque`
pub static POST_V1_PAYOUT_LINK_CHEQUE: RouteSpec = RouteSpec {
    key: "POST /v1/payout/link/cheque",
    method: Method::Post,
    path: "/v1/payout/link/cheque",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: false,
    bare: true,
    list: None,
};

/// `POST /v1/payout/link/info`
pub static POST_V1_PAYOUT_LINK_INFO: RouteSpec = RouteSpec {
    key: "POST /v1/payout/link/info",
    method: Method::Post,
    path: "/v1/payout/link/info",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `POST /v1/payout/link/list`
pub static POST_V1_PAYOUT_LINK_LIST: RouteSpec = RouteSpec {
    key: "POST /v1/payout/link/list",
    method: Method::Post,
    path: "/v1/payout/link/list",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: true,
    bare: false,
    list: Some(ListKind::Paged),
};

/// `POST /v1/payout/mass`
pub static POST_V1_PAYOUT_MASS: RouteSpec = RouteSpec {
    key: "POST /v1/payout/mass",
    method: Method::Post,
    path: "/v1/payout/mass",
    auth: RouteAuth::Payout,
    idempotent: true,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payout/refund-fee-config/get`
pub static POST_V1_PAYOUT_REFUND_FEE_CONFIG_GET: RouteSpec = RouteSpec {
    key: "POST /v1/payout/refund-fee-config/get",
    method: Method::Post,
    path: "/v1/payout/refund-fee-config/get",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `POST /v1/payout/refund-fee-config/set`
pub static POST_V1_PAYOUT_REFUND_FEE_CONFIG_SET: RouteSpec = RouteSpec {
    key: "POST /v1/payout/refund-fee-config/set",
    method: Method::Post,
    path: "/v1/payout/refund-fee-config/set",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/payout/services`
pub static POST_V1_PAYOUT_SERVICES: RouteSpec = RouteSpec {
    key: "POST /v1/payout/services",
    method: Method::Post,
    path: "/v1/payout/services",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: true,
    bare: false,
    list: Some(ListKind::Paged),
};

/// `POST /v1/payout/validate`
pub static POST_V1_PAYOUT_VALIDATE: RouteSpec = RouteSpec {
    key: "POST /v1/payout/validate",
    method: Method::Post,
    path: "/v1/payout/validate",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `POST /v1/referral/info`
pub static POST_V1_REFERRAL_INFO: RouteSpec = RouteSpec {
    key: "POST /v1/referral/info",
    method: Method::Post,
    path: "/v1/referral/info",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `POST /v1/refund/batch`
pub static POST_V1_REFUND_BATCH: RouteSpec = RouteSpec {
    key: "POST /v1/refund/batch",
    method: Method::Post,
    path: "/v1/refund/batch",
    auth: RouteAuth::Payout,
    idempotent: true,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/sandbox/deposit`
pub static POST_V1_SANDBOX_DEPOSIT: RouteSpec = RouteSpec {
    key: "POST /v1/sandbox/deposit",
    method: Method::Post,
    path: "/v1/sandbox/deposit",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/sandbox/faucet`
pub static POST_V1_SANDBOX_FAUCET: RouteSpec = RouteSpec {
    key: "POST /v1/sandbox/faucet",
    method: Method::Post,
    path: "/v1/sandbox/faucet",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/sandbox/reset`
pub static POST_V1_SANDBOX_RESET: RouteSpec = RouteSpec {
    key: "POST /v1/sandbox/reset",
    method: Method::Post,
    path: "/v1/sandbox/reset",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `GET /v1/sandbox/webhooks`
pub static GET_V1_SANDBOX_WEBHOOKS: RouteSpec = RouteSpec {
    key: "GET /v1/sandbox/webhooks",
    method: Method::Get,
    path: "/v1/sandbox/webhooks",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: false,
    list: Some(ListKind::Paged),
};

/// `POST /v1/sandbox/webhooks/replay`
pub static POST_V1_SANDBOX_WEBHOOKS_REPLAY: RouteSpec = RouteSpec {
    key: "POST /v1/sandbox/webhooks/replay",
    method: Method::Post,
    path: "/v1/sandbox/webhooks/replay",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/split/config/get`
pub static POST_V1_SPLIT_CONFIG_GET: RouteSpec = RouteSpec {
    key: "POST /v1/split/config/get",
    method: Method::Post,
    path: "/v1/split/config/get",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `POST /v1/split/config/set`
pub static POST_V1_SPLIT_CONFIG_SET: RouteSpec = RouteSpec {
    key: "POST /v1/split/config/set",
    method: Method::Post,
    path: "/v1/split/config/set",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/split/recipient/optin`
pub static POST_V1_SPLIT_RECIPIENT_OPTIN: RouteSpec = RouteSpec {
    key: "POST /v1/split/recipient/optin",
    method: Method::Post,
    path: "/v1/split/recipient/optin",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/split/recipient/optin/get`
pub static POST_V1_SPLIT_RECIPIENT_OPTIN_GET: RouteSpec = RouteSpec {
    key: "POST /v1/split/recipient/optin/get",
    method: Method::Post,
    path: "/v1/split/recipient/optin/get",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `POST /v1/split/rule`
pub static POST_V1_SPLIT_RULE: RouteSpec = RouteSpec {
    key: "POST /v1/split/rule",
    method: Method::Post,
    path: "/v1/split/rule",
    auth: RouteAuth::Payout,
    idempotent: true,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/split/rule/delete`
pub static POST_V1_SPLIT_RULE_DELETE: RouteSpec = RouteSpec {
    key: "POST /v1/split/rule/delete",
    method: Method::Post,
    path: "/v1/split/rule/delete",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/split/rule/list`
pub static POST_V1_SPLIT_RULE_LIST: RouteSpec = RouteSpec {
    key: "POST /v1/split/rule/list",
    method: Method::Post,
    path: "/v1/split/rule/list",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: true,
    bare: false,
    list: Some(ListKind::Paged),
};

/// `POST /v1/test-webhook/payment`
pub static POST_V1_TEST_WEBHOOK_PAYMENT: RouteSpec = RouteSpec {
    key: "POST /v1/test-webhook/payment",
    method: Method::Post,
    path: "/v1/test-webhook/payment",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/test-webhook/payout`
pub static POST_V1_TEST_WEBHOOK_PAYOUT: RouteSpec = RouteSpec {
    key: "POST /v1/test-webhook/payout",
    method: Method::Post,
    path: "/v1/test-webhook/payout",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/test-webhook/wallet`
pub static POST_V1_TEST_WEBHOOK_WALLET: RouteSpec = RouteSpec {
    key: "POST /v1/test-webhook/wallet",
    method: Method::Post,
    path: "/v1/test-webhook/wallet",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/transfer/batch`
pub static POST_V1_TRANSFER_BATCH: RouteSpec = RouteSpec {
    key: "POST /v1/transfer/batch",
    method: Method::Post,
    path: "/v1/transfer/batch",
    auth: RouteAuth::Payout,
    idempotent: true,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/transfer/to-personal`
pub static POST_V1_TRANSFER_TO_PERSONAL: RouteSpec = RouteSpec {
    key: "POST /v1/transfer/to-personal",
    method: Method::Post,
    path: "/v1/transfer/to-personal",
    auth: RouteAuth::Payout,
    idempotent: true,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/transfer/to-user`
pub static POST_V1_TRANSFER_TO_USER: RouteSpec = RouteSpec {
    key: "POST /v1/transfer/to-user",
    method: Method::Post,
    path: "/v1/transfer/to-user",
    auth: RouteAuth::Payout,
    idempotent: true,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/vrcs`
pub static POST_V1_VRCS: RouteSpec = RouteSpec {
    key: "POST /v1/vrcs",
    method: Method::Post,
    path: "/v1/vrcs",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/wallet`
pub static POST_V1_WALLET: RouteSpec = RouteSpec {
    key: "POST /v1/wallet",
    method: Method::Post,
    path: "/v1/wallet",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/wallet/block`
pub static POST_V1_WALLET_BLOCK: RouteSpec = RouteSpec {
    key: "POST /v1/wallet/block",
    method: Method::Post,
    path: "/v1/wallet/block",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/wallet/blocked-address-refund`
pub static POST_V1_WALLET_BLOCKED_ADDRESS_REFUND: RouteSpec = RouteSpec {
    key: "POST /v1/wallet/blocked-address-refund",
    method: Method::Post,
    path: "/v1/wallet/blocked-address-refund",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/wallet/qr`
pub static POST_V1_WALLET_QR: RouteSpec = RouteSpec {
    key: "POST /v1/wallet/qr",
    method: Method::Post,
    path: "/v1/wallet/qr",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: false,
    list: None,
};

/// `POST /v1/webhooks`
pub static POST_V1_WEBHOOKS: RouteSpec = RouteSpec {
    key: "POST /v1/webhooks",
    method: Method::Post,
    path: "/v1/webhooks",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// `POST /v1/webhooks/deliveries`
pub static POST_V1_WEBHOOKS_DELIVERIES: RouteSpec = RouteSpec {
    key: "POST /v1/webhooks/deliveries",
    method: Method::Post,
    path: "/v1/webhooks/deliveries",
    auth: RouteAuth::Payment,
    idempotent: false,
    safe: true,
    bare: false,
    list: Some(ListKind::Paged),
};

/// `POST /v1/webhooks/rotate-secret`
pub static POST_V1_WEBHOOKS_ROTATE_SECRET: RouteSpec = RouteSpec {
    key: "POST /v1/webhooks/rotate-secret",
    method: Method::Post,
    path: "/v1/webhooks/rotate-secret",
    auth: RouteAuth::Payout,
    idempotent: false,
    safe: false,
    bare: false,
    list: None,
};

/// Every merchant-facing route the core declares, in the order its conformance table
/// declares them. The SDK can call exactly these and nothing else.
pub static ROUTES: &[&RouteSpec] = &[
    &POST_V1_API_ALLOWLIST_ADD,
    &POST_V1_API_ALLOWLIST_ENABLE,
    &POST_V1_API_ALLOWLIST_LIST,
    &POST_V1_API_ALLOWLIST_REMOVE,
    &POST_V1_AUTO_WITHDRAW_DELETE,
    &POST_V1_AUTO_WITHDRAW_LIST,
    &POST_V1_AUTO_WITHDRAW_SET,
    &POST_V1_BALANCE,
    &POST_V1_BATCH_INFO,
    &GET_V1_CLAIM_TOKEN,
    &POST_V1_CLAIM_TOKEN,
    &GET_V1_CURRENCIES,
    &GET_V1_DOCUMENTS_BALANCE,
    &GET_V1_DOCUMENTS_BATCH,
    &GET_V1_DOCUMENTS_FEES,
    &POST_V1_DOCUMENTS_JOBS,
    &GET_V1_DOCUMENTS_JOBS_FILE,
    &POST_V1_DOCUMENTS_JOBS_INFO,
    &GET_V1_DOCUMENTS_LEDGER,
    &GET_V1_DOCUMENTS_LINK,
    &GET_V1_DOCUMENTS_REFERRALS,
    &GET_V1_DOCUMENTS_SPLIT,
    &GET_V1_DOCUMENTS_STATEMENT,
    &GET_V1_DOCUMENTS_WALLET_STATEMENT,
    &GET_V1_DOCUMENTS_KIND_ID,
    &POST_V1_EXCHANGE_RATE_LIST,
    &GET_V1_LINK_ID,
    &POST_V1_LINK_ID_CHECKOUT,
    &POST_V1_MERCHANTS,
    &POST_V1_MERCHANTS_ID_SANDBOX,
    &GET_V1_PAY_ID,
    &GET_V1_PAY_ID_QR,
    &POST_V1_PAY_ID_SELECT,
    &POST_V1_PAYMENT,
    &POST_V1_PAYMENT_ACCEPTED_LIST,
    &POST_V1_PAYMENT_ACCEPTED_SET,
    &POST_V1_PAYMENT_ACCURACY_GET,
    &POST_V1_PAYMENT_ACCURACY_SET,
    &POST_V1_PAYMENT_AUTOREFUND_GET,
    &POST_V1_PAYMENT_AUTOREFUND_SET,
    &POST_V1_PAYMENT_BATCH,
    &POST_V1_PAYMENT_CANCEL,
    &POST_V1_PAYMENT_DISCOUNT_LIST,
    &POST_V1_PAYMENT_DISCOUNT_SET,
    &POST_V1_PAYMENT_FEE_CONFIG_GET,
    &POST_V1_PAYMENT_FEE_CONFIG_SET,
    &POST_V1_PAYMENT_HISTORY,
    &POST_V1_PAYMENT_INFO,
    &POST_V1_PAYMENT_LINK,
    &POST_V1_PAYMENT_LINK_INFO,
    &POST_V1_PAYMENT_LINK_LIST,
    &POST_V1_PAYMENT_LINK_TOGGLE,
    &POST_V1_PAYMENT_QR,
    &POST_V1_PAYMENT_REFUND,
    &POST_V1_PAYMENT_RESEND,
    &POST_V1_PAYMENT_RESOLVE,
    &POST_V1_PAYMENT_SEND_EMAIL,
    &POST_V1_PAYMENT_SERVICES,
    &POST_V1_PAYMENT_TESTING_WEBHOOK,
    &POST_V1_PAYOUT,
    &POST_V1_PAYOUT_APPROVE,
    &POST_V1_PAYOUT_BATCH,
    &POST_V1_PAYOUT_CALCULATE,
    &POST_V1_PAYOUT_CANCEL,
    &POST_V1_PAYOUT_FEE_CONFIG_GET,
    &POST_V1_PAYOUT_FEE_CONFIG_SET,
    &POST_V1_PAYOUT_HISTORY,
    &POST_V1_PAYOUT_INFO,
    &POST_V1_PAYOUT_LINK,
    &POST_V1_PAYOUT_LINK_BATCH,
    &POST_V1_PAYOUT_LINK_CANCEL,
    &POST_V1_PAYOUT_LINK_CHEQUE,
    &POST_V1_PAYOUT_LINK_INFO,
    &POST_V1_PAYOUT_LINK_LIST,
    &POST_V1_PAYOUT_MASS,
    &POST_V1_PAYOUT_REFUND_FEE_CONFIG_GET,
    &POST_V1_PAYOUT_REFUND_FEE_CONFIG_SET,
    &POST_V1_PAYOUT_SERVICES,
    &POST_V1_PAYOUT_VALIDATE,
    &POST_V1_REFERRAL_INFO,
    &POST_V1_REFUND_BATCH,
    &POST_V1_SANDBOX_DEPOSIT,
    &POST_V1_SANDBOX_FAUCET,
    &POST_V1_SANDBOX_RESET,
    &GET_V1_SANDBOX_WEBHOOKS,
    &POST_V1_SANDBOX_WEBHOOKS_REPLAY,
    &POST_V1_SPLIT_CONFIG_GET,
    &POST_V1_SPLIT_CONFIG_SET,
    &POST_V1_SPLIT_RECIPIENT_OPTIN,
    &POST_V1_SPLIT_RECIPIENT_OPTIN_GET,
    &POST_V1_SPLIT_RULE,
    &POST_V1_SPLIT_RULE_DELETE,
    &POST_V1_SPLIT_RULE_LIST,
    &POST_V1_TEST_WEBHOOK_PAYMENT,
    &POST_V1_TEST_WEBHOOK_PAYOUT,
    &POST_V1_TEST_WEBHOOK_WALLET,
    &POST_V1_TRANSFER_BATCH,
    &POST_V1_TRANSFER_TO_PERSONAL,
    &POST_V1_TRANSFER_TO_USER,
    &POST_V1_VRCS,
    &POST_V1_WALLET,
    &POST_V1_WALLET_BLOCK,
    &POST_V1_WALLET_BLOCKED_ADDRESS_REFUND,
    &POST_V1_WALLET_QR,
    &POST_V1_WEBHOOKS,
    &POST_V1_WEBHOOKS_DELIVERIES,
    &POST_V1_WEBHOOKS_ROTATE_SECRET,
];
