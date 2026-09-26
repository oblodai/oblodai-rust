//! Calling any operation by its OpenAPI `operationId` — for tools driven by the contract rather
//! than by typed models (an MCP server, a generic proxy). The call is the one the generated method
//! would build, and goes through the same transport: signing, idempotency, retries, error
//! classification. Public operations work on a client without an API key.
//!
//! ```no_run
//! # async fn demo(client: &oblodai::Client) -> oblodai::Result<()> {
//! use oblodai::{routes, InvokeInput};
//! use serde_json::json;
//!
//! let invoice = client
//!     .invoke(
//!         routes::CREATE_PAYMENT.operation_id,
//!         InvokeInput {
//!             body: Some(json!({"amount": "25", "currency": "USDT"})),
//!             ..Default::default()
//!         },
//!     )?
//!     .idempotency_key("order-1001")
//!     .await?;
//! println!("{}", invoice["url"]);
//! # Ok(()) }
//! ```

use std::collections::BTreeMap;

use serde_json::Value;

use crate::core::request::fill_path;
use crate::core::route::RouteSpec;
use crate::error::{Error, Result};
use crate::generated::routes;
use crate::resources::Call;

/// Error code: no route of this SDK release carries the `operationId`.
pub const CODE_UNKNOWN_OPERATION: &str = "sdk.unknown_operation";
/// Error code: the `operationId` names a route of another kind (a paged list, a file, or a plain
/// envelope) than the `invoke*` method called.
pub const CODE_WRONG_INVOKE: &str = "sdk.wrong_invoke";

/// The request of [`Client::invoke`](crate::Client::invoke),
/// [`invoke_list`](crate::Client::invoke_list) or [`invoke_file`](crate::Client::invoke_file).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InvokeInput {
    /// Fills the `{name}` segments of the route's path; every one is required, no other allowed.
    pub path: BTreeMap<String, String>,
    /// The query string, in order.
    pub query: Vec<(String, String)>,
    /// The JSON body; `None` goes out as `{}` on a route that has a body. For a paged route its
    /// `limit` and `offset` (here or in `query`) set the first page.
    pub body: Option<Value>,
}

/// What an `invoke*` method answers with; a route has exactly one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Json,
    List,
    File,
}

impl Kind {
    fn of(route: &RouteSpec) -> Self {
        if route.bare {
            Kind::File
        } else if route.list_kind.is_some() {
            Kind::List
        } else {
            Kind::Json
        }
    }

    fn describe(self) -> (&'static str, &'static str) {
        match self {
            Kind::Json => ("a JSON", "invoke"),
            Kind::List => ("a paged", "invoke_list"),
            Kind::File => ("a file", "invoke_file"),
        }
    }
}

/// The `{name}` segments of a path template, as `&'static str` slices of it.
fn path_names(template: &'static str) -> impl Iterator<Item = &'static str> {
    template
        .split('/')
        .filter_map(|s| s.strip_prefix('{').and_then(|s| s.strip_suffix('}')))
}

/// Look `operation_id` up and build the call a generated method would, checking before the
/// network that it is the kind the caller means and that the path parameters fit the template.
pub(crate) fn call(operation_id: &str, input: InvokeInput, want: Kind) -> Result<Call> {
    let route = routes::route(operation_id).ok_or_else(|| {
        Error::config(
            CODE_UNKNOWN_OPERATION,
            format!("no operation {operation_id:?} in this SDK release"),
            Some("operation_id"),
        )
    })?;
    let kind = Kind::of(route);
    if kind != want {
        let (what, method) = kind.describe();
        return Err(Error::config(
            CODE_WRONG_INVOKE,
            format!("{operation_id} is {what} operation; call `{method}`"),
            Some("operation_id"),
        ));
    }
    let InvokeInput {
        mut path,
        query,
        body,
    } = input;
    let mut call = Call::new(route);
    let mut params = Vec::new();
    for name in path_names(route.path) {
        if let Some(value) = path.remove(name) {
            params.push((name, value));
        }
    }
    if let Some(extra) = path.keys().next() {
        return Err(Error::config(
            "sdk.bad_path_param",
            format!("{} has no path parameter \"{extra}\"", route.path),
            Some("path"),
        ));
    }
    // A missing or malformed value is refused now, with the error the send would give.
    fill_path(route.path, &params)?;
    for (name, value) in params {
        call.path(name, value);
    }
    call.query_pairs(query);
    if let Some(body) = body {
        call.json_body(body);
    }
    if route.body_idempotency_key {
        call.idempotency_key_in_body();
    }
    Ok(call)
}

/// The `invoke*` methods, for the async client and the blocking one.
macro_rules! invoke_methods {
    ($tr:ty) => {
        /// Call the envelope operation `operation_id` (neither a paged list nor a file) and answer
        /// with its `result` as raw JSON. Every call option (`idempotency_key`, `timeout`, …)
        /// works as on a typed method.
        ///
        /// Refused before the network with `sdk.unknown_operation` for an id this release does
        /// not know, `sdk.wrong_invoke` for a paged or file operation (use
        /// [`invoke_list`](Self::invoke_list) / [`invoke_file`](Self::invoke_file)), and
        /// `sdk.bad_path_param` when `input.path` does not fit the route's path.
        pub fn invoke(
            &self,
            operation_id: &str,
            input: $crate::InvokeInput,
        ) -> $crate::Result<$crate::Request<$tr, serde_json::Value>> {
            let call = $crate::invoke::call(operation_id, input, $crate::invoke::Kind::Json)?;
            Ok($crate::Request::new(self.transport.clone(), call))
        }

        /// Walk the paged operation `operation_id` lazily; its items are raw JSON. `limit` and
        /// `offset` in the body or the query set the first page. Errors as
        /// [`invoke`](Self::invoke).
        pub fn invoke_list(
            &self,
            operation_id: &str,
            input: $crate::InvokeInput,
        ) -> $crate::Result<$crate::Pager<$tr, serde_json::Value>> {
            let call = $crate::invoke::call(operation_id, input, $crate::invoke::Kind::List)?;
            Ok($crate::Pager::new(self.transport.clone(), call))
        }

        /// Fetch the document of the file operation `operation_id` (PDF/CSV). Errors as
        /// [`invoke`](Self::invoke).
        pub fn invoke_file(
            &self,
            operation_id: &str,
            input: $crate::InvokeInput,
        ) -> $crate::Result<$crate::Request<$tr, $crate::FileResult>> {
            let call = $crate::invoke::call(operation_id, input, $crate::invoke::Kind::File)?;
            Ok($crate::Request::new(self.transport.clone(), call))
        }
    };
}
pub(crate) use invoke_methods;
