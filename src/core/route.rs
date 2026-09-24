//! The shape of a route, as the generated registry (`crate::generated::routes`) builds it from the
//! OpenAPI contract: one [`RouteSpec`] per `operationId`.

/// HTTP method of a route. `#[non_exhaustive]` so a new one is not a breaking change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Patch => "PATCH",
            Method::Delete => "DELETE",
        }
    }

    /// Whether the request carries a JSON body (everything but `GET`).
    pub fn has_body(self) -> bool {
        self != Method::Get
    }
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Which credential the gateway expects on a route.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RouteAuth {
    /// No credentials: payer- and recipient-facing endpoints.
    Public,
    /// Signed with the merchant's API key.
    Key,
    /// Store provisioning: unsigned, gated by `X-Admin-Token` on a self-hosted gateway.
    Onboard,
}

impl RouteAuth {
    pub fn as_str(self) -> &'static str {
        match self {
            RouteAuth::Public => "public",
            RouteAuth::Key => "key",
            RouteAuth::Onboard => "onboard",
        }
    }
}

/// Shape of a list result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ListKind {
    /// `{ items, paginate }` — offset pagination.
    Paged,
}

/// One operation of the API.
///
/// Only the generated registry constructs these, so the struct is `#[non_exhaustive]`: the
/// contract can start declaring another per-route fact without a major version here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct RouteSpec {
    /// The OpenAPI `operationId` (`createPayment`).
    pub operation_id: &'static str,
    pub method: Method,
    /// Path template; `{name}` segments are filled from path parameters.
    pub path: &'static str,
    pub auth: RouteAuth,
    /// The gateway deduplicates it by `Idempotency-Key` (`x-idempotent`): the SDK generates a key
    /// when the caller supplies none and reuses it on every retry.
    pub idempotent: bool,
    /// Retry-safe (`GET`, or `x-retry-safe`): re-sending cannot duplicate a side effect.
    pub safe: bool,
    /// Outside the JSON envelope: the answer is a file (PDF/CSV).
    pub bare: bool,
    pub list_kind: Option<ListKind>,
}

impl std::fmt::Display for RouteSpec {
    /// `POST /v1/payment (createPayment)`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} ({})", self.method, self.path, self.operation_id)
    }
}
