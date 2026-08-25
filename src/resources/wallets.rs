//! Static deposit wallets: one permanent address per customer, deposits reported as `wallet.paid`.

use serde_json::json;

use super::base::{Call, RequestBuilder};
use crate::contract::models::{Payout, Wallet, WalletBlocked, WalletQr};
use crate::contract::requests::{
    WalletBlockRequest, WalletBlockedAddressRefundRequest, WalletRequest,
};
use crate::contract::routes;

/// The `wallets` namespace.
#[derive(Clone, Debug)]
pub struct Wallets<Tr> {
    transport: Tr,
}

impl<Tr: Clone> Wallets<Tr> {
    pub(crate) fn new(transport: Tr) -> Self {
        Self { transport }
    }

    /// `POST /v1/wallet` — idempotent by `order_id`.
    pub fn create(&self, params: WalletRequest) -> RequestBuilder<Tr, Wallet> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_WALLET,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/wallet/qr`.
    pub fn qr(&self, address: impl Into<String>) -> RequestBuilder<Tr, WalletQr> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_WALLET_QR,
            Call::new()
                .json(json!({ "address": address.into() }))
                .done(),
        )
    }

    /// `POST /v1/wallet/block` — stop crediting an address; later deposits wait for a refund
    /// decision.
    pub fn block(&self, params: WalletBlockRequest) -> RequestBuilder<Tr, WalletBlocked> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_WALLET_BLOCK,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/wallet/blocked-address-refund` — send funds that landed on a blocked address
    /// back. Payout key.
    pub fn refund_blocked_deposit(
        &self,
        params: WalletBlockedAddressRefundRequest,
    ) -> RequestBuilder<Tr, Payout> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_WALLET_BLOCKED_ADDRESS_REFUND,
            Call::new().body(&params).done(),
        )
    }
}
