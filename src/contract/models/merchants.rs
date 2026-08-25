//! Merchant provisioning: the onboarding call and the sandbox (dev store) it can mint.

/// An API key pair as minted by onboarding. The secret is shown once.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ApiKeyPair {
    pub public_id: String,
    pub secret: String,
    /// `api` — the unified key kind current merchants receive.
    pub kind: String,
}

/// `POST /v1/merchants` — a freshly provisioned merchant and its keys.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MerchantOnboarded {
    pub merchant_id: String,
    pub project_id: String,
    /// The unified key (same as `payment_key`/`payout_key` for merchants created now).
    pub api_key: ApiKeyPair,
    pub payment_key: ApiKeyPair,
    pub payout_key: ApiKeyPair,
}

/// `POST /v1/merchants/{id}/sandbox` — the merchant's dev store and its `test_` key.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SandboxStore {
    pub merchant_id: String,
    pub project_id: String,
    /// The unified key (same as `payment_key`/`payout_key` for merchants created now).
    pub api_key: ApiKeyPair,
    pub payment_key: ApiKeyPair,
    pub payout_key: ApiKeyPair,
    /// False when the dev store already existed (the call is idempotent).
    pub created: bool,
}
