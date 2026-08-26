//! Progress of asynchronous batches, and the internal transfers between platform balances.

use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::time::Duration;

use super::base::{Call, RequestBuilder};
use crate::contract::models::{BatchInfo, BatchSubmitted, TransferToPersonal, TransferToUser};
use crate::contract::requests::{
    BatchInfoRequest, TransferBatchRequest, TransferToPersonalRequest, TransferToUserRequest,
};
use crate::contract::routes;
use crate::core::engine::CallOptions;
use crate::core::transport::Transport;
use crate::error::{ErrorKind, Result};

/// The `batches` namespace.
#[derive(Clone, Debug)]
pub struct Batches<Tr> {
    transport: Tr,
}

impl<Tr: Clone> Batches<Tr> {
    pub(crate) fn new(transport: Tr) -> Self {
        Self { transport }
    }

    /// `POST /v1/batch/info` — status, counters and per-row outcomes.
    ///
    /// The route accepts either key kind, but the core requires the kind that created the batch,
    /// so a `merchant.wrong_key_kind` refusal is retried once with the payout key when one is
    /// configured.
    pub fn info(&self, params: BatchInfoRequest) -> BatchInfoCall<Tr> {
        BatchInfoCall {
            transport: self.transport.clone(),
            opts: Call::new().body(&params).done(),
        }
    }
}

/// The prepared `batch/info` call, with its key-kind fallback.
#[must_use = "nothing is sent until this builder is awaited (or `.send()` is called)"]
pub struct BatchInfoCall<Tr> {
    transport: Tr,
    opts: CallOptions,
}

impl<Tr> std::fmt::Debug for BatchInfoCall<Tr> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BatchInfoCall").finish_non_exhaustive()
    }
}

impl<Tr> BatchInfoCall<Tr> {
    /// Per-attempt timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.opts.timeout = Some(timeout);
        self
    }

    /// Overall budget, retries included.
    pub fn deadline(mut self, deadline: Duration) -> Self {
        self.opts.deadline = Some(deadline);
        self
    }

    /// Sign with the payout key straight away (and skip the fallback).
    pub fn prefer_payout_key(mut self, prefer: bool) -> Self {
        self.opts.prefer_payout_key = prefer;
        self
    }

    /// An extra header on this call only, merged over the client-wide ones. Names the SDK owns
    /// (the signing headers, `Accept`, `Content-Type`, `User-Agent`, `X-Admin-Token`) are never
    /// overridden; a CR/LF or non-ASCII value is a `sdk.bad_header` config error.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.opts.headers.push((name.into(), value.into()));
        self
    }

    /// Is the answer a "you used the wrong key kind" refusal we should retry with the other key?
    fn is_wrong_key_kind(already_payout: bool, err: &crate::error::Error) -> bool {
        !already_payout
            && err.kind() == ErrorKind::Permission
            && err.code() == "merchant.wrong_key_kind"
    }
}

impl BatchInfoCall<Transport> {
    /// Send, falling back to the payout key on `merchant.wrong_key_kind`.
    pub async fn send(self) -> Result<BatchInfo> {
        let already_payout = self.opts.prefer_payout_key;
        let mut retry = self.opts.clone();
        retry.prefer_payout_key = true;
        let first = RequestBuilder::<Transport, BatchInfo>::new(
            self.transport.clone(),
            &routes::POST_V1_BATCH_INFO,
            self.opts,
        )
        .send()
        .await;
        match first {
            Err(err) if Self::is_wrong_key_kind(already_payout, &err) => {
                RequestBuilder::<Transport, BatchInfo>::new(
                    self.transport,
                    &routes::POST_V1_BATCH_INFO,
                    retry,
                )
                .send()
                .await
            }
            other => other,
        }
    }
}

impl IntoFuture for BatchInfoCall<Transport> {
    type Output = Result<BatchInfo>;
    type IntoFuture = Pin<Box<dyn Future<Output = Result<BatchInfo>> + Send>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.send())
    }
}

#[cfg(feature = "blocking")]
impl BatchInfoCall<crate::core::transport::BlockingTransport> {
    /// Send, falling back to the payout key on `merchant.wrong_key_kind`.
    pub fn send(self) -> Result<BatchInfo> {
        type Tr = crate::core::transport::BlockingTransport;
        let already_payout = self.opts.prefer_payout_key;
        let mut retry = self.opts.clone();
        retry.prefer_payout_key = true;
        let first = RequestBuilder::<Tr, BatchInfo>::new(
            self.transport.clone(),
            &routes::POST_V1_BATCH_INFO,
            self.opts,
        )
        .send();
        match first {
            Err(err) if Self::is_wrong_key_kind(already_payout, &err) => {
                RequestBuilder::<Tr, BatchInfo>::new(
                    self.transport,
                    &routes::POST_V1_BATCH_INFO,
                    retry,
                )
                .send()
            }
            other => other,
        }
    }
}

/// Internal, instant, fee-free moves between platform balances. Payout key.
#[derive(Clone, Debug)]
pub struct Transfers<Tr> {
    transport: Tr,
}

impl<Tr: Clone> Transfers<Tr> {
    pub(crate) fn new(transport: Tr) -> Self {
        Self { transport }
    }

    /// `POST /v1/transfer/to-personal` — business balance → the owner's personal wallet (needs an
    /// owner link). **Payout key.**
    ///
    /// Codes to branch on: `transfer.bad_amount`, `merchant.no_owner`,
    /// `merchant.no_personal_wallet`, `payout.insufficient_funds` (retryable),
    /// `payout.funds_maturing` (retryable), `merchant.wrong_key_kind`.
    pub fn to_personal(
        &self,
        params: TransferToPersonalRequest,
    ) -> RequestBuilder<Tr, TransferToPersonal> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_TRANSFER_TO_PERSONAL,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/transfer/to-user` — business balance → another platform user's personal wallet.
    /// `amount` and `currency` are required. **Payout key.**
    ///
    /// Codes to branch on: `transfer.bad_amount`, `transfer.no_recipient`,
    /// `transfer.recipient_not_found`, `transfer.bad_recipient` (the recipient is yourself),
    /// `payout.insufficient_funds` (retryable), `merchant.wrong_key_kind`.
    pub fn to_user(&self, params: TransferToUserRequest) -> RequestBuilder<Tr, TransferToUser> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_TRANSFER_TO_USER,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/transfer/batch` — ASYNCHRONOUS batch (**≤ 5000**) of `to_user` transfers; poll
    /// `batches().info()`. `order_id` is required on every item. **Payout key.**
    ///
    /// Codes to branch on: `batch.too_large`, `batch.empty`, `batch.order_id_required`,
    /// `batch.duplicate_order_id`, `batch.bad_recipient`, `merchant.wrong_key_kind`.
    pub fn batch(&self, params: TransferBatchRequest) -> RequestBuilder<Tr, BatchSubmitted> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_TRANSFER_BATCH,
            Call::new().body(&params).done(),
        )
    }
}
