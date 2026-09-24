//! Pagination: one page when awaited, every page when streamed, nothing before either.

// A `Client` only exists with an HTTP backend feature on.
#![cfg(feature = "reqwest-client")]

mod support;

use std::sync::Arc;

use futures_util::StreamExt;
use oblodai::models::HistoryRequest;
use oblodai::{Client, HttpBackend};
use serde_json::{json, Value};
use support::{api_error, ok, MockBackend, Scripted};

fn page(items: Vec<Value>, offset: i64, total: i64, per_page: i64) -> Scripted {
    let count = items.len() as i64;
    ok(json!({
        "items": items,
        "paginate": {
            "total": total,
            "per_page": per_page,
            "offset": offset,
            "has_pages": offset + count < total
        }
    }))
}

/// A real recorded invoice with a fresh uuid — the models are strict, and so is this test.
fn invoice(uuid: &str) -> Value {
    let mut item = support::sample("payment");
    item["uuid"] = json!(uuid);
    item
}

/// A real recorded payout with a fresh uuid.
fn payout(uuid: &str) -> Value {
    let mut item = support::sample("payout");
    item["uuid"] = json!(uuid);
    item
}

fn harness(script: Vec<Scripted>) -> (Client, Arc<MockBackend>) {
    let mock = MockBackend::new(script);
    let client = Client::builder()
        .public_id("pk")
        .secret("s")
        .base_url("https://api.test")
        .env(Vec::<(String, String)>::new())
        .http_backend(mock.clone() as Arc<dyn HttpBackend>)
        .build()
        .unwrap();
    (client, mock)
}

#[tokio::test]
async fn awaiting_a_pager_fetches_exactly_one_page() {
    let (client, mock) = harness(vec![page(vec![invoice("a"), invoice("b")], 0, 5, 2)]);
    let first = client
        .payments()
        .list_history(HistoryRequest::default())
        .limit(2)
        .await
        .unwrap();
    assert_eq!(first.items.len(), 2);
    assert_eq!(first.paginate.total, 5);
    assert!(first.paginate.has_pages);
    assert_eq!(mock.call_count(), 1);
    assert_eq!(mock.first().json_body(), json!({ "limit": 2, "offset": 0 }));
}

#[tokio::test]
async fn nothing_is_requested_until_the_pager_is_consumed() {
    let (client, mock) = harness(vec![page(vec![], 0, 0, 50)]);
    let pager = client.payments().list_history(HistoryRequest::default());
    assert_eq!(
        mock.call_count(),
        0,
        "building a pager must not touch the network"
    );
    let _stream = pager.stream();
    assert_eq!(mock.call_count(), 0, "nor must turning it into a stream");
}

#[tokio::test]
async fn streaming_walks_every_page_lazily() {
    let (client, mock) = harness(vec![
        page(vec![invoice("1"), invoice("2")], 0, 5, 2),
        page(vec![invoice("3"), invoice("4")], 2, 5, 2),
        page(vec![invoice("5")], 4, 5, 2),
    ]);
    let mut seen = Vec::new();
    let mut stream = client
        .payments()
        .list_history(HistoryRequest::default())
        .limit(2)
        .stream();
    while let Some(item) = stream.next().await {
        seen.push(item.unwrap().uuid);
    }
    assert_eq!(seen, ["1", "2", "3", "4", "5"]);
    assert_eq!(mock.call_count(), 3);
    // The offset advances by what actually arrived, not by the requested page size.
    assert_eq!(
        mock.calls()[1].json_body(),
        json!({ "limit": 2, "offset": 2 })
    );
    assert_eq!(
        mock.calls()[2].json_body(),
        json!({ "limit": 2, "offset": 4 })
    );
}

#[tokio::test]
async fn a_failed_page_ends_the_stream_with_that_error() {
    let (client, _) = harness(vec![
        page(vec![invoice("1")], 0, 5, 1),
        api_error(
            404,
            json!({ "code": "payment.not_found", "retryable": false }),
        ),
    ]);
    let mut stream = client
        .payments()
        .list_history(HistoryRequest::default())
        .limit(1)
        .stream();
    assert_eq!(stream.next().await.unwrap().unwrap().uuid, "1");
    let err = stream.next().await.unwrap().unwrap_err();
    assert_eq!(err.code(), "payment.not_found");
    assert!(
        stream.next().await.is_none(),
        "the stream is over, not stuck"
    );
}

#[tokio::test]
async fn a_pager_that_is_never_consumed_cannot_fail_anything() {
    let (client, mock) = harness(vec![]);
    // Dropping an unconsumed pager must not panic and must not have sent anything.
    drop(client.payouts().list_history(HistoryRequest::default()));
    assert_eq!(mock.call_count(), 0);
}

#[tokio::test]
async fn all_collects_across_pages_with_a_cap() {
    let (client, mock) = harness(vec![
        page(vec![payout("1"), payout("2")], 0, 3, 2),
        page(vec![payout("3")], 2, 3, 2),
    ]);
    let items = client
        .payouts()
        .list_history(HistoryRequest::default())
        .limit(2)
        .all(None)
        .await
        .unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(mock.call_count(), 2);

    let (client, mock) = harness(vec![page(vec![invoice("1"), invoice("2")], 0, 9, 2)]);
    let capped = client
        .payments()
        .list_history(HistoryRequest::default())
        .limit(2)
        .all(Some(2))
        .await
        .unwrap();
    assert_eq!(capped.len(), 2);
    assert_eq!(
        mock.call_count(),
        1,
        "the cap stops the walk instead of fetching more"
    );
}

#[tokio::test]
async fn a_short_page_ends_the_walk_even_when_has_pages_lies() {
    let (client, mock) = harness(vec![ok(json!({
        "items": [],
        "paginate": { "total": 99, "per_page": 50, "offset": 0, "has_pages": true }
    }))]);
    let items = client
        .payments()
        .list_history(HistoryRequest::default())
        .all(None)
        .await
        .unwrap();
    assert!(items.is_empty());
    assert_eq!(mock.call_count(), 1, "an empty page stops the walk");
}

#[tokio::test]
async fn caller_filters_travel_with_every_page() {
    let (client, mock) = harness(vec![
        page(vec![payout("1")], 0, 2, 1),
        page(vec![payout("2")], 1, 2, 1),
    ]);
    let _ = client
        .payouts()
        .list_history(HistoryRequest {
            kind: Some(oblodai::enums::PayoutKind::Refund),
            ..Default::default()
        })
        .limit(1)
        .all(None)
        .await
        .unwrap();
    for call in mock.calls() {
        assert_eq!(call.json_body()["kind"], "refund");
    }
}

#[tokio::test]
async fn a_get_list_route_pages_over_the_query_string() {
    let (client, mock) = harness(vec![page(vec![], 0, 0, 5)]);
    let _ = client
        .sandbox()
        .list_webhooks(oblodai::generated::resources::SandboxListWebhooksQuery::default())
        .limit(5)
        .await
        .unwrap();
    let call = mock.first();
    assert_eq!(call.query("limit").as_deref(), Some("5"));
    assert_eq!(call.query("offset").as_deref(), Some("0"));
    assert!(call.body.is_none());
}

#[tokio::test]
async fn list_pages_never_carry_an_idempotency_key() {
    let (client, mock) = harness(vec![page(vec![], 0, 0, 50)]);
    let _ = client
        .payouts()
        .list_history(HistoryRequest::default())
        .await
        .unwrap();
    assert_eq!(mock.first().header("idempotency-key"), None);
}

#[tokio::test]
async fn by_page_yields_one_page_per_request() {
    let (client, mock) = harness(vec![
        page(vec![invoice("a"), invoice("b")], 0, 3, 2),
        page(vec![invoice("c")], 2, 3, 2),
    ]);
    let mut pages = client
        .payments()
        .list_history(HistoryRequest::default())
        .limit(2)
        .by_page();
    assert_eq!(mock.call_count(), 0, "nothing before the first poll");
    let first = pages.next().await.unwrap().unwrap();
    assert_eq!(first.items.len(), 2);
    assert_eq!(first.paginate.total, 3);
    assert_eq!(mock.call_count(), 1);
    let second = pages.next().await.unwrap().unwrap();
    assert_eq!(second.items[0].uuid, "c");
    assert!(
        pages.next().await.is_none(),
        "has_pages=false ends the walk"
    );
    assert_eq!(mock.call_count(), 2);
    let offsets: Vec<_> = mock
        .calls()
        .iter()
        .map(|c| c.json_body()["offset"].clone())
        .collect();
    assert_eq!(offsets, [json!(0), json!(2)]);
}

#[tokio::test]
async fn limit_and_offset_in_the_params_start_the_walk() {
    let (client, mock) = harness(vec![page(vec![], 40, 40, 20)]);
    let _ = client
        .payments()
        .list_history(HistoryRequest {
            limit: Some(20),
            offset: Some(40),
            ..Default::default()
        })
        .await
        .unwrap();
    let body = mock.first().json_body();
    assert_eq!(body["limit"], 20);
    assert_eq!(body["offset"], 40);
}
