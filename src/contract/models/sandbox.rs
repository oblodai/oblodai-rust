//! Sandbox-only helpers: the faucet, simulated deposits, resets and webhook replays.

use crate::contract::models::common::Money;

/// `/v1/sandbox/faucet`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FaucetResult {
    pub asset: String,
    pub amount: Money,
    pub journal_id: String,
}

/// `/v1/sandbox/deposit`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SandboxDeposit {
    pub invoice_id: String,
    pub amount: Money,
    pub confirmations: i64,
    pub txid: String,
}

/// `/v1/sandbox/reset`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SandboxReset {
    pub invoices_cancelled: i64,
    pub balances_zeroed: i64,
}

/// `/v1/sandbox/webhooks/replay`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SandboxReplay {
    pub ok: bool,
    pub delivery_id: String,
}
