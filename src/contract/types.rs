//! Hand-written types describing the generated route registry (`src/contract/routes.rs`).
//!
//! The registry itself is produced by `scripts/codegen.py` from `contract/contract.json`, which is
//! exported from the core's own conformance table — so every route the SDK can call is one the core
//! declares, with the same auth gate and idempotency wrapper.

/// HTTP method of a route. The gateway speaks only these two.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Method {
    Get,
    Post,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
        }
    }
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Which credential the core's gate expects. Mirrors `api_conformance_test.go` constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RouteAuth {
    /// No credentials: payer- and recipient-facing endpoints.
    Public,
    /// The payment key pair.
    Payment,
    /// The payout key pair (falls back to the payment pair when none is configured).
    Payout,
    /// Either key kind is accepted.
    Any,
    /// Merchant provisioning: unsigned, gated by `X-Admin-Token` on a self-hosted gateway.
    Onboard,
}

impl RouteAuth {
    pub fn as_str(self) -> &'static str {
        match self {
            RouteAuth::Public => "public",
            RouteAuth::Payment => "payment",
            RouteAuth::Payout => "payout",
            RouteAuth::Any => "any",
            RouteAuth::Onboard => "onboard",
        }
    }
}

/// Shape of a list result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListKind {
    /// `{ items, paginate }` — offset pagination.
    Paged,
    /// `{ items }` — the core caps it by catalog size instead of paginating.
    Plain,
}

/// One route of the merchant API.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteSpec {
    /// `"POST /v1/payment"` — the key the core's conformance table uses.
    pub key: &'static str,
    pub method: Method,
    /// Path template; `{name}` segments are filled from path parameters.
    pub path: &'static str,
    pub auth: RouteAuth,
    /// Wrapped in the core's `withIdempotency`: a key is generated when the caller supplies none.
    pub idempotent: bool,
    /// Read-only: a transport failure may be retried without risking a duplicate side effect.
    pub safe: bool,
    /// Outside the JSON envelope (binary documents).
    pub bare: bool,
    pub list: Option<ListKind>,
}
