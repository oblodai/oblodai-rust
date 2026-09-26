//! `Client::invoke*`: any operation by its `operationId`, through the same transport as a typed
//! method — signing, idempotency, retries, error classification. Operation ids come from the
//! generated route table, never spelled here.

// A `Client` only exists with an HTTP backend feature on.
#![cfg(feature = "reqwest-client")]
#![allow(clippy::result_large_err)]

mod support;

use std::collections::BTreeMap;
use std::sync::Arc;

use oblodai::core::signing::{HEADER_IDEMPOTENCY_KEY, HEADER_PUBLIC_ID, HEADER_SIGNATURE};
use oblodai::routes;
use oblodai::{Client, ErrorKind, HttpBackend, InvokeInput, RouteSpec};
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

/// A client without an API key — the public operations still work.
fn keyless_client_on(mock: &Arc<MockBackend>) -> Client {
    Client::builder()
        .base_url("https://api.test")
        .env(Vec::<(String, String)>::new())
        .http_backend(mock.clone() as Arc<dyn HttpBackend>)
        .build()
        .unwrap()
}

/// `route.path` with every `{name}` replaced by `value`.
fn filled(route: &RouteSpec, value: &str) -> String {
    let mut out = String::new();
    let mut rest = route.path;
    while let Some(start) = rest.find('{') {
        let end = start + rest[start..].find('}').unwrap();
        out.push_str(&rest[..start]);
        out.push_str(value);
        rest = &rest[end + 1..];
    }
    out + rest
}

/// The `{name}` segments of a route's path.
fn path_names(route: &RouteSpec) -> Vec<String> {
    route
        .path
        .split('/')
        .filter_map(|s| s.strip_prefix('{').and_then(|s| s.strip_suffix('}')))
        .map(str::to_string)
        .collect()
}

fn with_path(route: &RouteSpec, value: &str) -> InvokeInput {
    InvokeInput {
        path: path_names(route)
            .into_iter()
            .map(|n| (n, value.to_string()))
            .collect(),
        ..Default::default()
    }
}

fn file(bytes: &[u8]) -> Scripted {
    Scripted {
        status: 200,
        body: String::from_utf8(bytes.to_vec()).unwrap(),
        headers: vec![
            ("content-type".into(), "application/pdf".into()),
            (
                "content-disposition".into(),
                "attachment; filename=\"statement.pdf\"".into(),
            ),
        ],
        ..Default::default()
    }
}

#[tokio::test]
async fn invoke_sends_the_body_signed_and_answers_with_the_raw_result() {
    let route = &routes::CREATE_PAYMENT;
    let mock = MockBackend::new(vec![ok(json!({"uuid": "u-1", "future_field": 7}))]);
    let client = client_on(&mock);
    let body = json!({"amount": "25", "currency": "USDT", "order_id": "o-1"});
    let result: Value = client
        .invoke(
            route.operation_id,
            InvokeInput {
                body: Some(body.clone()),
                ..Default::default()
            },
        )
        .unwrap()
        .await
        .unwrap();
    assert_eq!(result, json!({"uuid": "u-1", "future_field": 7}));
    let sent = mock.first();
    assert_eq!(sent.method, route.method.to_string());
    assert_eq!(sent.path(), route.path);
    assert_eq!(sent.json_body(), body);
    assert_eq!(sent.header(HEADER_PUBLIC_ID), Some("pk"));
    assert!(
        sent.header(HEADER_SIGNATURE).is_some(),
        "the call is signed"
    );
}

#[tokio::test]
async fn an_idempotent_route_gets_a_generated_key_and_a_caller_key_wins() {
    let route = &routes::CREATE_PAYMENT;
    assert!(route.idempotent);
    let mock = MockBackend::new(vec![ok(json!({})), ok(json!({}))]);
    let client = client_on(&mock);
    let _ = client
        .invoke(route.operation_id, InvokeInput::default())
        .unwrap()
        .await
        .unwrap();
    let generated = mock
        .first()
        .header(HEADER_IDEMPOTENCY_KEY)
        .map(str::to_string);
    assert!(
        generated.is_some_and(|k| !k.is_empty()),
        "a key is generated"
    );
    let _ = client
        .invoke(route.operation_id, InvokeInput::default())
        .unwrap()
        .idempotency_key("order-1001")
        .await
        .unwrap();
    assert_eq!(
        mock.last().header(HEADER_IDEMPOTENCY_KEY),
        Some("order-1001")
    );
}

#[tokio::test]
async fn a_body_idempotency_key_route_takes_the_key_in_its_body() {
    let route = routes::ROUTES
        .iter()
        .find(|r| r.body_idempotency_key)
        .expect("the contract has a route with the key in its body");
    let mock = MockBackend::new(vec![ok(json!({}))]);
    let client = client_on(&mock);
    let _ = client
        .invoke(
            route.operation_id,
            InvokeInput {
                body: Some(json!({"amount": "10"})),
                ..Default::default()
            },
        )
        .unwrap()
        .idempotency_key("tap-1")
        .await
        .unwrap();
    let sent = mock.first();
    assert_eq!(sent.header(HEADER_IDEMPOTENCY_KEY), None);
    assert_eq!(
        sent.json_body(),
        json!({"amount": "10", "idempotency_key": "tap-1"})
    );
}

#[tokio::test]
async fn a_public_operation_works_without_a_key_and_fills_its_path() {
    let route = &routes::GET_PAYOUT_CLAIM;
    assert!(!path_names(route).is_empty());
    let mock = MockBackend::new(vec![ok(json!({"state": "open"}))]);
    let client = keyless_client_on(&mock);
    let result = client
        .invoke(route.operation_id, with_path(route, "tok-1"))
        .unwrap()
        .await
        .unwrap();
    assert_eq!(result, json!({"state": "open"}));
    let sent = mock.first();
    assert_eq!(sent.path(), filled(route, "tok-1"));
    assert_eq!(sent.header(HEADER_SIGNATURE), None);
    assert_eq!(sent.header(HEADER_PUBLIC_ID), None);
}

#[tokio::test]
async fn a_path_value_is_percent_encoded() {
    let route = &routes::GET_PAYOUT_CLAIM;
    let mock = MockBackend::new(vec![ok(json!({}))]);
    let client = keyless_client_on(&mock);
    let _ = client
        .invoke(route.operation_id, with_path(route, "a b"))
        .unwrap()
        .await
        .unwrap();
    assert_eq!(mock.first().path(), filled(route, "a%20b"));
}

#[tokio::test]
async fn a_missing_or_unknown_path_parameter_is_refused_before_the_network() {
    let route = &routes::GET_PAYOUT_CLAIM;
    let mock = MockBackend::new(vec![]);
    let client = keyless_client_on(&mock);

    let err = client
        .invoke(route.operation_id, InvokeInput::default())
        .unwrap_err();
    assert_eq!(err.code(), "sdk.bad_path_param");
    assert_eq!(err.kind(), ErrorKind::Config);

    let mut extra = with_path(route, "tok");
    extra.path.insert("nope".into(), "x".into());
    let err = client.invoke(route.operation_id, extra).unwrap_err();
    assert_eq!(err.code(), "sdk.bad_path_param");
    assert!(err.to_string().contains("nope"), "{err}");

    let mut path = BTreeMap::new();
    path.insert("nope".to_string(), "x".to_string());
    let err = client
        .invoke(
            routes::CREATE_PAYMENT.operation_id,
            InvokeInput {
                path,
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code(), "sdk.bad_path_param");
    assert_eq!(mock.call_count(), 0);
}

#[tokio::test]
async fn invoke_list_walks_a_paged_operation() {
    let route = &routes::LIST_PAYMENT_HISTORY;
    assert!(route.list_kind.is_some());
    let mock = MockBackend::new(vec![ok(json!({
        "items": [{"uuid": "a"}, {"uuid": "b"}],
        "paginate": {"total": 2, "per_page": 2, "offset": 0, "has_pages": false}
    }))]);
    let client = client_on(&mock);
    let page = client
        .invoke_list(
            route.operation_id,
            InvokeInput {
                body: Some(json!({"status": "paid", "limit": 2})),
                ..Default::default()
            },
        )
        .unwrap()
        .await
        .unwrap();
    assert_eq!(page.items, [json!({"uuid": "a"}), json!({"uuid": "b"})]);
    assert!(!page.paginate.has_pages);
    assert_eq!(
        mock.first().json_body(),
        json!({"status": "paid", "limit": 2, "offset": 0})
    );
}

#[tokio::test]
async fn invoke_file_answers_with_the_document_and_sends_the_query() {
    let route = &routes::GET_STATEMENT_DOCUMENT;
    assert!(route.bare);
    let mock = MockBackend::new(vec![file(b"%PDF-1.7")]);
    let client = client_on(&mock);
    let doc = client
        .invoke_file(
            route.operation_id,
            InvokeInput {
                query: vec![("lang".into(), "de".into())],
                ..Default::default()
            },
        )
        .unwrap()
        .await
        .unwrap();
    assert_eq!(doc.bytes, b"%PDF-1.7");
    assert_eq!(doc.content_type, "application/pdf");
    assert_eq!(doc.filename.as_deref(), Some("statement.pdf"));
    let sent = mock.first();
    assert_eq!(sent.path(), route.path);
    assert_eq!(sent.query("lang").as_deref(), Some("de"));
}

#[tokio::test]
async fn an_unknown_operation_is_a_config_error_before_the_network() {
    let mock = MockBackend::new(vec![]);
    let client = client_on(&mock);
    let err = client
        .invoke("noSuchOperation", InvokeInput::default())
        .unwrap_err();
    assert_eq!(err.code(), "sdk.unknown_operation");
    assert_eq!(err.kind(), ErrorKind::Config);
    assert!(err.to_string().contains("noSuchOperation"), "{err}");
    assert_eq!(
        client
            .invoke_list("noSuchOperation", InvokeInput::default())
            .unwrap_err()
            .code(),
        "sdk.unknown_operation"
    );
    assert_eq!(
        client
            .invoke_file("noSuchOperation", InvokeInput::default())
            .unwrap_err()
            .code(),
        "sdk.unknown_operation"
    );
    assert_eq!(mock.call_count(), 0);
}

#[tokio::test]
async fn the_wrong_invoke_method_names_the_right_one() {
    let mock = MockBackend::new(vec![]);
    let client = client_on(&mock);
    let paged = routes::LIST_PAYMENT_HISTORY.operation_id;
    let bare = routes::GET_STATEMENT_DOCUMENT.operation_id;
    let json = routes::CREATE_PAYMENT.operation_id;

    let err = client.invoke(paged, InvokeInput::default()).unwrap_err();
    assert_eq!(err.code(), "sdk.wrong_invoke");
    assert_eq!(err.kind(), ErrorKind::Config);
    assert!(err.to_string().contains("invoke_list"), "{err}");

    let err = client.invoke(bare, InvokeInput::default()).unwrap_err();
    assert_eq!(err.code(), "sdk.wrong_invoke");
    assert!(err.to_string().contains("invoke_file"), "{err}");

    let err = client
        .invoke_list(json, InvokeInput::default())
        .unwrap_err();
    assert_eq!(err.code(), "sdk.wrong_invoke");
    assert!(err.to_string().contains("invoke`"), "{err}");

    let err = client
        .invoke_file(paged, InvokeInput::default())
        .unwrap_err();
    assert_eq!(err.code(), "sdk.wrong_invoke");
    assert_eq!(mock.call_count(), 0);
}

/// Every route of the contract is reachable by exactly one of the three methods.
#[tokio::test]
async fn every_route_has_exactly_one_invoke_method() {
    let mock = MockBackend::new(vec![]);
    let client = client_on(&mock);
    for route in routes::ROUTES {
        let input = with_path(route, "x");
        let ok = [
            client.invoke(route.operation_id, input.clone()).is_ok(),
            client
                .invoke_list(route.operation_id, input.clone())
                .is_ok(),
            client.invoke_file(route.operation_id, input).is_ok(),
        ];
        assert_eq!(ok.iter().filter(|b| **b).count(), 1, "{route}: {ok:?}");
    }
    assert_eq!(mock.call_count(), 0, "building a call sends nothing");
}

#[cfg(feature = "blocking")]
#[test]
fn the_blocking_client_invokes_too() {
    let route = &routes::CREATE_PAYMENT;
    let mock = MockBackend::new(vec![
        ok(json!({"uuid": "u-1"})),
        ok(json!({
            "items": [{"uuid": "a"}],
            "paginate": {"total": 1, "per_page": 50, "offset": 0, "has_pages": false}
        })),
        file(b"%PDF"),
    ]);
    let client = oblodai::blocking::Client::builder()
        .public_id("pk")
        .secret("s")
        .base_url("https://api.test")
        .env(Vec::<(String, String)>::new())
        .blocking_http_backend(mock.clone() as Arc<dyn oblodai::BlockingHttpBackend>)
        .build_blocking()
        .unwrap();
    let result = client
        .invoke(
            route.operation_id,
            InvokeInput {
                body: Some(json!({"amount": "1"})),
                ..Default::default()
            },
        )
        .unwrap()
        .send()
        .unwrap();
    assert_eq!(result, json!({"uuid": "u-1"}));
    assert!(mock.first().header(HEADER_IDEMPOTENCY_KEY).is_some());

    let page = client
        .invoke_list(
            routes::LIST_PAYMENT_HISTORY.operation_id,
            InvokeInput::default(),
        )
        .unwrap()
        .page()
        .unwrap();
    assert_eq!(page.items, [json!({"uuid": "a"})]);

    let doc = client
        .invoke_file(
            routes::GET_STATEMENT_DOCUMENT.operation_id,
            InvokeInput::default(),
        )
        .unwrap()
        .send()
        .unwrap();
    assert_eq!(doc.bytes, b"%PDF");

    let err = client
        .invoke("noSuchOperation", InvokeInput::default())
        .unwrap_err();
    assert_eq!(err.code(), "sdk.unknown_operation");
}
