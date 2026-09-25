//! Spec §3 item 8: long-running operations (batches, document jobs) are followed with a waiter.

#![cfg(feature = "reqwest-client")]

mod support;

use std::sync::Arc;
use std::time::Duration;

use oblodai::core::signing::HEADER_IDEMPOTENCY_KEY;
use oblodai::models::{DocumentJobRequest, PaymentBatchRequest, PaymentRequest};
use oblodai::{Client, HttpBackend, JobStatus};
use serde_json::{json, Value};
use support::{ok, MockBackend, Scripted};

fn client_on(mock: &Arc<MockBackend>) -> Client {
    Client::builder()
        .public_id("pk")
        .secret("s")
        .base_url("https://api.test")
        .env(Vec::<(String, String)>::new())
        .http_backend(mock.clone() as Arc<dyn HttpBackend>)
        .build()
        .unwrap()
}

fn batch(status: &str) -> Value {
    json!({
        "batch_id": "b-1", "created_at": "t", "updated_at": "t", "failed": 0, "succeeded": 2,
        "total": 2, "items": [], "kind": "payment", "on_error": "continue", "status": status
    })
}

fn job(status: &str) -> Value {
    json!({
        "job_id": "j-1", "created_at": "t", "updated_at": "t", "format": "pdf", "kind": "statement",
        "lang": "en", "period": {"from": "2026-09-01", "to": "2026-09-24"}, "status": status
    })
}

#[test]
fn the_lro_table_names_the_batches_and_document_jobs() {
    assert_eq!(
        oblodai::lro::poll_of("createPayoutBatch"),
        Some("getBatchInfo")
    );
    assert_eq!(
        oblodai::lro::poll_of("createTransferBatch"),
        Some("getBatchInfo")
    );
    assert_eq!(
        oblodai::lro::poll_of("createDocumentJob"),
        Some("getDocumentJob")
    );
    assert_eq!(oblodai::lro::poll_of("createPayment"), None);
    // Every operation in the table exists in the generated registry.
    for (create, poll) in oblodai::lro::LRO {
        assert!(oblodai::routes::route(create).is_some(), "{create}");
        assert!(oblodai::routes::route(poll).is_some(), "{poll}");
    }
}

#[tokio::test(start_paused = true)]
async fn a_batch_job_polls_until_a_terminal_status() {
    let mock = MockBackend::new(vec![
        ok(json!({"batch_id": "b-1", "count": 2, "kind": "payment", "status": "pending"})),
        ok(batch("processing")),
        ok(batch("completed")),
    ]);
    let client = client_on(&mock);
    let job = client
        .batches()
        .create_payment(PaymentBatchRequest::default())
        .extra_header("X-Tenant", "eu")
        .job()
        .await
        .unwrap();
    assert_eq!(job.id(), "b-1");
    assert_eq!(job.result().count, 2);
    let done = job.wait().await.unwrap();
    assert_eq!(done.status(), "completed");
    assert_eq!(done.succeeded, 2);

    let calls = mock.calls();
    assert_eq!(calls.len(), 3);
    assert!(
        calls[0].header(HEADER_IDEMPOTENCY_KEY).is_some(),
        "the create is keyed"
    );
    for poll in &calls[1..] {
        assert_eq!(poll.path(), "/v1/batch/info");
        assert_eq!(poll.json_body(), json!({"batch_id": "b-1"}));
        assert_eq!(
            poll.header(HEADER_IDEMPOTENCY_KEY),
            None,
            "polls carry no key"
        );
        assert_eq!(
            poll.header("x-tenant"),
            Some("eu"),
            "the create's headers follow"
        );
        assert_ne!(poll.header("x-request-id"), calls[0].header("x-request-id"));
    }
}

#[tokio::test(start_paused = true)]
async fn a_stopped_batch_is_returned_not_raised_and_a_slow_one_times_out() {
    let mock = MockBackend::new(vec![
        ok(json!({"batch_id": "b-1", "count": 2, "kind": "payment", "status": "pending"})),
        ok(batch("stopped")),
    ]);
    let client = client_on(&mock);
    let job = client
        .batches()
        .create_payment(PaymentBatchRequest::default())
        .job()
        .await
        .unwrap();
    assert!(job.wait().await.unwrap().is_terminal());

    let mut script: Vec<Scripted> = vec![ok(
        json!({"batch_id": "b-2", "count": 1, "kind": "payment", "status": "pending"}),
    )];
    script.extend((0..10).map(|_| ok(batch("processing"))));
    let mock = MockBackend::new(script);
    let client = client_on(&mock);
    let job = client
        .batches()
        .create_payment(PaymentBatchRequest::default())
        .job()
        .await
        .unwrap();
    let err = job
        .wait_with(Duration::from_secs(5), Duration::from_secs(2))
        .await
        .unwrap_err();
    assert_eq!(err.code(), "sdk.job_timeout");
    assert!(
        err.message().contains("b-2") && err.message().contains("processing"),
        "{err}"
    );
}

#[tokio::test(start_paused = true)]
async fn a_document_job_waits_and_downloads_its_file() {
    let mock = MockBackend::new(vec![
        ok(job("queued")),
        ok(job("done")),
        Scripted {
            status: 200,
            body: "%PDF-1.7".into(),
            headers: vec![
                ("content-type".into(), "application/pdf".into()),
                (
                    "content-disposition".into(),
                    "attachment; filename=\"statement.pdf\"".into(),
                ),
            ],
            ..Default::default()
        },
    ]);
    let client = client_on(&mock);
    let job = client
        .documents()
        .create_job(DocumentJobRequest::new("statement"))
        .job()
        .await
        .unwrap();
    assert_eq!(job.wait().await.unwrap().status(), "done");
    let file = job.download().await.unwrap();
    assert_eq!(file.bytes, b"%PDF-1.7");
    assert_eq!(file.filename.as_deref(), Some("statement.pdf"));
    let download = mock.last();
    assert_eq!(download.method, "GET");
    assert_eq!(download.path(), "/v1/documents/jobs/file");
    assert_eq!(download.query("job_id").as_deref(), Some("j-1"));
}

#[tokio::test]
async fn a_batch_job_has_nothing_to_download() {
    let mock = MockBackend::new(vec![ok(
        json!({"batch_id": "b-1", "count": 2, "kind": "payment", "status": "pending"}),
    )]);
    let client = client_on(&mock);
    let job = client
        .batches()
        .create_payment(PaymentBatchRequest::default())
        .job()
        .await
        .unwrap();
    assert_eq!(job.download().await.unwrap_err().code(), "sdk.no_download");
}

#[test]
fn only_long_running_operations_have_a_waiter() {
    // `job()` exists only where the answer type starts a job: a payment does not compile with it.
    // (`client.payments().create(p).job()` — no method `job` for `Request<_, PaymentView>`.)
    let _ = PaymentRequest::new("1", "USDT");
    fn has_job<A: oblodai::JobAck>() {}
    has_job::<oblodai::models::BatchSubmitResponse>();
    has_job::<oblodai::models::DocumentJobAccepted>();
}
