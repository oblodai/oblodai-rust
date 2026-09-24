//! Long-running operations (batches, document jobs) and how to follow them — a decision of this
//! SDK, not of the API: the generator knows nothing of this table (Ruling 3).
//!
//! A create call listed in [`LRO`] can be sent with [`Request::job`] instead of `.await`: it
//! returns a [`Job`] around the create answer, and [`Job::wait`] polls the operation named here
//! until the status is terminal ([`TERMINAL_STATUSES`]).
//!
//! ```no_run
//! # async fn demo(client: &oblodai::Client) -> oblodai::Result<()> {
//! use oblodai::models::{DocumentJobRequest, PayoutBatchRequest};
//!
//! let job = client.batches().create_payout(PayoutBatchRequest::default()).job().await?;
//! let info = job.wait().await?; // BatchInfoResponse, status completed or stopped
//! println!("{} ok, {} failed", info.succeeded, info.failed);
//!
//! let job = client.documents().create_job(DocumentJobRequest::new("statement")).job().await?;
//! if job.wait().await?.status.as_str() == "done" {
//!     job.download().await?.write_to("statement.pdf").ok();
//! }
//! # Ok(()) }
//! ```

use std::time::Duration;

use serde::de::DeserializeOwned;
use serde_json::json;

use crate::core::engine::CallOptions;
use crate::core::route::RouteSpec;
use crate::core::transport::Transport;
use crate::error::{Error, Result};
use crate::generated::models::{
    BatchInfoResponse, BatchSubmitResponse, DocumentJobAccepted, DocumentJobView,
};
use crate::generated::routes;
use crate::resources::base::{Decode, FileResult, Request};

/// `create operationId -> poll operationId`.
pub const LRO: &[(&str, &str)] = &[
    ("createPaymentBatch", "getBatchInfo"),
    ("createPayoutBatch", "getBatchInfo"),
    ("createRefundBatch", "getBatchInfo"),
    ("createTransferBatch", "getBatchInfo"),
    ("createDocumentJob", "getDocumentJob"),
];

/// Statuses after which a job no longer changes: a batch ends `completed` or `stopped`
/// (`on_error=stop`), a document job `done`, `failed` or `expired`.
pub const TERMINAL_STATUSES: &[&str] = &["completed", "stopped", "done", "failed", "expired"];

/// The poll operation of a long-running create operation, if it is one.
pub fn poll_of(operation_id: &str) -> Option<&'static str> {
    LRO.iter()
        .find(|(create, _)| *create == operation_id)
        .map(|(_, poll)| *poll)
}

/// The answer of a create call that starts a job.
pub trait JobAck: Decode + Send + 'static {
    /// What a poll answers with.
    type Status: DeserializeOwned + JobStatus + Send + 'static;
    /// The route that reports the job's progress.
    fn poll_route() -> &'static RouteSpec;
    /// The job's id field, in the create answer and in the poll request.
    fn id_field() -> &'static str;
    /// The job's id.
    fn job_id(&self) -> &str;
    /// The route that returns the finished job's file, if the job makes one.
    fn download_route() -> Option<&'static RouteSpec> {
        None
    }
}

/// A poll answer: where the job is.
pub trait JobStatus {
    /// The status literal (`pending`, `completed`, `done`, …).
    fn status(&self) -> &str;

    /// Whether the job will not change any more.
    fn is_terminal(&self) -> bool {
        TERMINAL_STATUSES.contains(&self.status())
    }
}

impl JobAck for BatchSubmitResponse {
    type Status = BatchInfoResponse;
    fn poll_route() -> &'static RouteSpec {
        &routes::GET_BATCH_INFO
    }
    fn id_field() -> &'static str {
        "batch_id"
    }
    fn job_id(&self) -> &str {
        &self.batch_id
    }
}

impl JobStatus for BatchInfoResponse {
    fn status(&self) -> &str {
        self.status.as_str()
    }
}

impl JobAck for DocumentJobAccepted {
    type Status = DocumentJobView;
    fn poll_route() -> &'static RouteSpec {
        &routes::GET_DOCUMENT_JOB
    }
    fn id_field() -> &'static str {
        "job_id"
    }
    fn job_id(&self) -> &str {
        &self.job_id
    }
    fn download_route() -> Option<&'static RouteSpec> {
        Some(&routes::DOWNLOAD_DOCUMENT_JOB_FILE)
    }
}

impl JobStatus for DocumentJobView {
    fn status(&self) -> &str {
        self.status.as_str()
    }
}

/// How long [`Job::wait`] polls by default.
pub const DEFAULT_WAIT_TIMEOUT: Duration = Duration::from_secs(300);
/// How often [`Job::wait`] polls by default.
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(2);

/// A long-running operation: its [`id`](Self::id), the create answer ([`result`](Self::result))
/// and [`wait`](Self::wait).
#[derive(Clone, Debug)]
pub struct Job<Tr, A> {
    id: String,
    result: A,
    transport: Tr,
    /// The create call's options for the polls: same timeout, retries and headers; no
    /// idempotency key (it belongs to the create) and a fresh request id per poll.
    follow: CallOptions,
}

impl<Tr, A: JobAck> Job<Tr, A> {
    pub(crate) fn new(transport: Tr, result: A, create: &CallOptions) -> Result<Self> {
        let id = result.job_id().to_string();
        if id.is_empty() {
            return Err(Error::contract(
                format!("long-running call answered without {}", A::id_field()),
                200,
                None,
            ));
        }
        let follow = CallOptions {
            headers: create.headers.clone(),
            timeout: create.timeout,
            deadline: create.deadline,
            max_retries: create.max_retries,
            ..Default::default()
        };
        Ok(Self {
            id,
            result,
            transport,
            follow,
        })
    }

    /// The job's id (`batch_id`, `job_id`).
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The create call's answer.
    pub fn result(&self) -> &A {
        &self.result
    }

    fn poll_options(&self) -> CallOptions {
        CallOptions {
            body: Some(json!({ A::id_field(): self.id })),
            ..self.follow.clone()
        }
    }

    fn download_options(&self) -> Result<(&'static RouteSpec, CallOptions)> {
        let route = A::download_route().ok_or_else(|| {
            Error::config(
                "sdk.no_download",
                format!("job {} produces no file to download", self.id),
                None,
            )
        })?;
        let opts = CallOptions {
            query: vec![(A::id_field().to_string(), self.id.clone())],
            ..self.follow.clone()
        };
        Ok((route, opts))
    }

    fn timed_out(&self, timeout: Duration, status: &str) -> Error {
        Error::job_timeout(format!(
            "job {} is still {} after {timeout:?}",
            self.id,
            if status.is_empty() {
                "unfinished"
            } else {
                status
            }
        ))
    }
}

impl<A: JobAck> Job<Transport, A> {
    /// Where the job is now (one poll).
    pub async fn poll(&self) -> Result<A::Status> {
        self.transport
            .call(A::poll_route(), self.poll_options())
            .await
    }

    /// Poll every 2 s until the status is terminal, for at most 5 minutes; return that answer.
    /// A terminal status is returned, not raised: a `failed` job is inspected like a finished one.
    pub async fn wait(&self) -> Result<A::Status> {
        self.wait_with(DEFAULT_WAIT_TIMEOUT, DEFAULT_POLL_INTERVAL)
            .await
    }

    /// [`wait`](Self::wait) with your own budget and interval; `sdk.job_timeout` when the budget
    /// runs out first (the job keeps running — wait again).
    pub async fn wait_with(&self, timeout: Duration, interval: Duration) -> Result<A::Status> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let answer = self.poll().await?;
            if answer.is_terminal() {
                return Ok(answer);
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(self.timed_out(timeout, answer.status()));
            }
            tokio::time::sleep(interval.min(remaining)).await;
        }
    }

    /// The finished job's file (document jobs only; `sdk.no_download` otherwise).
    pub async fn download(&self) -> Result<FileResult> {
        let (route, opts) = self.download_options()?;
        FileResult::decode(route, self.transport.execute(route, opts).await?)
    }
}

#[cfg(feature = "blocking")]
impl<A: JobAck> Job<crate::core::transport::BlockingTransport, A> {
    /// Where the job is now (one poll).
    pub fn poll(&self) -> Result<A::Status> {
        self.transport.call(A::poll_route(), self.poll_options())
    }

    /// Poll every 2 s until the status is terminal, for at most 5 minutes; return that answer.
    pub fn wait(&self) -> Result<A::Status> {
        self.wait_with(DEFAULT_WAIT_TIMEOUT, DEFAULT_POLL_INTERVAL)
    }

    /// [`wait`](Self::wait) with your own budget and interval.
    pub fn wait_with(&self, timeout: Duration, interval: Duration) -> Result<A::Status> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            let answer = self.poll()?;
            if answer.is_terminal() {
                return Ok(answer);
            }
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(self.timed_out(timeout, answer.status()));
            }
            std::thread::sleep(interval.min(remaining));
        }
    }

    /// The finished job's file (document jobs only).
    pub fn download(&self) -> Result<FileResult> {
        let (route, opts) = self.download_options()?;
        FileResult::decode(route, self.transport.execute(route, opts)?)
    }
}

fn check_lro<A: JobAck>(route: &'static RouteSpec) -> Result<()> {
    if poll_of(route.operation_id) == Some(A::poll_route().operation_id) {
        return Ok(());
    }
    Err(Error::config(
        "sdk.not_long_running",
        format!("{route} does not start a long-running operation"),
        None,
    ))
}

impl<A: JobAck> Request<Transport, A> {
    /// Send the create call and follow the job it starts: a [`Job`] whose
    /// [`wait`](Job::wait) polls until the job is done.
    pub async fn job(self) -> Result<Job<Transport, A>> {
        let (transport, route, opts) = self.parts();
        check_lro::<A>(route)?;
        let answer = transport.execute(route, opts.clone()).await?;
        Job::new(transport, A::decode(route, answer)?, &opts)
    }
}

#[cfg(feature = "blocking")]
impl<A: JobAck> Request<crate::core::transport::BlockingTransport, A> {
    /// Send the create call and follow the job it starts.
    pub fn job(self) -> Result<Job<crate::core::transport::BlockingTransport, A>> {
        let (transport, route, opts) = self.parts();
        check_lro::<A>(route)?;
        let answer = transport.execute(route, opts.clone())?;
        Job::new(transport, A::decode(route, answer)?, &opts)
    }
}
