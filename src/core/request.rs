//! Builds the outgoing request — URL, headers, body — as a pure function of its inputs, so the
//! signing material (what is signed) and the wire bytes (what is sent) come from one place and
//! cannot disagree. Nothing here touches the network or the clock.

use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use url::Url;

use super::route::{Method, RouteAuth, RouteSpec};
use super::signing::{
    sign_request, SignInput, HEADER_ADMIN_TOKEN, HEADER_IDEMPOTENCY_KEY, HEADER_PUBLIC_ID,
    HEADER_SIGNATURE, HEADER_TIMESTAMP,
};
use crate::error::{Error, Result};

/// One API key pair.
#[derive(Clone, PartialEq, Eq)]
pub struct Credentials {
    pub public_id: String,
    pub secret: String,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("public_id", &self.public_id)
            .field("secret", &"[redacted]")
            .finish()
    }
}

/// Everything the request line and headers are derived from.
pub struct BuildInput<'a> {
    pub base_url: &'a str,
    pub route: &'a RouteSpec,
    pub path_params: &'a [(&'static str, String)],
    pub query: &'a [(String, String)],
    /// Already-serialized body; empty for GET.
    pub body: &'a str,
    pub credentials: Option<&'a Credentials>,
    pub idempotency_key: Option<&'a str>,
    /// Unix seconds; signed into [`HEADER_TIMESTAMP`].
    pub ts: i64,
    pub user_agent: &'a str,
    /// Admin token of a self-hosted gateway. Sent on `onboard` routes and nowhere else, whatever
    /// the caller configured.
    pub admin_token: Option<&'a str>,
    pub extra_headers: &'a [(String, String)],
    /// `X-Request-ID` of the call, sent on every attempt.
    pub request_id: &'a str,
}

/// The request as it will go on the wire.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuiltRequest {
    pub url: String,
    pub method: &'static str,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    /// What was signed (path + query); kept for debugging signature mismatches.
    pub request_uri: String,
}

/// Headers the SDK owns; a caller-supplied header with one of these names is dropped, compared
/// case-insensitively. `Accept`, `User-Agent` and `X-Admin-Token` are here too: `reqwest` (like
/// most clients) *appends* headers, so leaving them out put two `Accept` lines on the wire and let
/// a caller header shadow the configured admin token.
/// The header naming one call (the same on every attempt), for matching the merchant's logs with
/// the gateway's.
pub const HEADER_REQUEST_ID: &str = "X-Request-ID";

const RESERVED_HEADERS: [&str; 11] = [
    HEADER_PUBLIC_ID,
    HEADER_SIGNATURE,
    HEADER_TIMESTAMP,
    HEADER_ADMIN_TOKEN,
    HEADER_IDEMPOTENCY_KEY,
    "content-type",
    "content-length",
    "host",
    "accept",
    "user-agent",
    "x-request-id",
];

/// Reject a caller-supplied header before it reaches the socket.
///
/// A CR or LF in a value is request splitting; a non-ASCII byte is not representable in an HTTP/1
/// field and different clients mangle it differently. Both are configuration mistakes, so they are
/// refused up front rather than silently dropped.
fn assert_header(name: &str, value: &str) -> Result<()> {
    let bad = |what: &str, field: &str| {
        Err(Error::config(
            "sdk.bad_header",
            format!("header {name:?}: {what}"),
            Some(field),
        ))
    };
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
    {
        return bad("name must be a non-empty HTTP token", "header_name");
    }
    if value.bytes().any(|b| b == b'\r' || b == b'\n') {
        return bad("value must not contain CR or LF", "header_value");
    }
    if !value.is_ascii() {
        return bad("value must be ASCII", "header_value");
    }
    Ok(())
}

/// `encodeURIComponent`: everything but ASCII alphanumerics and `-_.!~*'()`.
const COMPONENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'!')
    .remove(b'~')
    .remove(b'*')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')');

pub fn build_request(input: BuildInput<'_>) -> Result<BuiltRequest> {
    let route = input.route;
    let mut url = join_url(input.base_url, &fill_path(route.path, input.path_params)?)?;
    if !input.query.is_empty() {
        let mut pairs = url.query_pairs_mut();
        for (k, v) in input.query {
            pairs.append_pair(k, v);
        }
    }
    let request_uri = match url.query() {
        Some(q) => format!("{}?{}", url.path(), q),
        None => url.path().to_string(),
    };

    let mut headers: Vec<(String, String)> = Vec::new();
    for (k, v) in input.extra_headers {
        assert_header(k, v)?;
        if RESERVED_HEADERS.iter().any(|r| r.eq_ignore_ascii_case(k)) {
            continue;
        }
        // Deduplicate the caller's own headers too: the backend appends, so two entries with the
        // same name would both go on the wire. The last one given wins, as a map would.
        match headers
            .iter_mut()
            .find(|(name, _)| name.eq_ignore_ascii_case(k))
        {
            Some(slot) => slot.1 = v.clone(),
            None => headers.push((k.clone(), v.clone())),
        }
    }
    headers.push(("Accept".into(), "application/json".into()));
    headers.push(("User-Agent".into(), input.user_agent.into()));
    if !input.request_id.is_empty() {
        assert_header(HEADER_REQUEST_ID, input.request_id)?;
        headers.push((HEADER_REQUEST_ID.into(), input.request_id.into()));
    }
    let has_body = route.method.has_body();
    if has_body {
        headers.push(("Content-Type".into(), "application/json".into()));
    }
    if let Some(key) = input.idempotency_key {
        headers.push((HEADER_IDEMPOTENCY_KEY.into(), key.into()));
    }
    if route.auth == RouteAuth::Onboard {
        if let Some(token) = input.admin_token {
            headers.push((HEADER_ADMIN_TOKEN.into(), token.into()));
        }
    }

    if route.auth == RouteAuth::Key {
        let creds = input.credentials.ok_or_else(|| {
            Error::config(
                "sdk.missing_credentials",
                format!(
                    "{} {} needs the merchant API key: pass public_id/secret to the client builder \
                     or set OBLODAI_PUBLIC_ID / OBLODAI_SECRET",
                    route.method, route.path,
                ),
                None,
            )
        })?;
        let signature = sign_request(
            &creds.secret,
            &SignInput {
                ts: input.ts,
                method: route.method.as_str(),
                request_uri: &request_uri,
                idempotency_key: input.idempotency_key,
                body: if has_body { input.body.as_bytes() } else { b"" },
            },
        );
        headers.push((HEADER_PUBLIC_ID.into(), creds.public_id.clone()));
        headers.push((HEADER_TIMESTAMP.into(), input.ts.to_string()));
        headers.push((HEADER_SIGNATURE.into(), signature));
    }

    Ok(BuiltRequest {
        url: url.to_string(),
        method: route.method.as_str(),
        headers,
        body: if has_body {
            Some(input.body.to_string())
        } else {
            None
        },
        request_uri,
    })
}

/// Append a route path to the base URL, keeping any path prefix the base carries
/// (`https://host/api` → `https://host/api/v1/payment`).
pub fn join_url(base_url: &str, route_path: &str) -> Result<Url> {
    let mut base = Url::parse(base_url).map_err(|e| {
        Error::config(
            "sdk.bad_config",
            format!("base_url is not a valid URL: {e}"),
            Some("base_url"),
        )
    })?;
    let prefix = base.path().trim_end_matches('/').to_string();
    base.set_path(&format!("{prefix}{route_path}"));
    base.set_query(None);
    base.set_fragment(None);
    Ok(base)
}

/// Substitute `{name}` segments; every placeholder must be supplied, values are percent-encoded.
pub fn fill_path(template: &str, params: &[(&'static str, String)]) -> Result<String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        let end = rest[start..].find('}').map(|i| start + i).ok_or_else(|| {
            Error::config(
                "sdk.bad_path_param",
                format!("unterminated path template {template}"),
                None,
            )
        })?;
        out.push_str(&rest[..start]);
        let name = &rest[start + 1..end];
        let value = params
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if value.is_empty() || value == "." || value == ".." || value.contains('/') {
            return Err(Error::config(
                "sdk.bad_path_param",
                format!(
                    "path parameter \"{name}\" for {template} must be a non-empty single segment \
                     (got {value:?})"
                ),
                Some("path"),
            ));
        }
        out.push_str(&utf8_percent_encode(value, COMPONENT).to_string());
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Serialize a request body once; a missing POST body becomes `{}`, a GET body is empty.
pub fn serialize_body(body: Option<&serde_json::Value>, method: Method) -> String {
    if !method.has_body() {
        return String::new();
    }
    match body {
        None | Some(serde_json::Value::Null) => "{}".to_string(),
        Some(v) => v.to_string(),
    }
}
