//! How a caller names an object and a page of a list.
//!
//! `IdRef` is the Rust spelling of the reference SDK's `string | model`; `Lookup` is the
//! `{uuid} | {order_id}` pair invoices and payouts are addressed by.

/// Page size and starting offset for a paged list route. Every field is optional; the SDK sends
/// `limit: 50, offset: 0` when you leave them out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct PageParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

impl PageParams {
    /// Page size.
    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Where to start.
    pub fn offset(mut self, offset: i64) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// An object named either by its bare id or by the object itself — the Rust spelling of the
/// reference SDK's `string | model`.
///
/// ```no_run
/// # async fn demo(client: &oblodai::Client) -> oblodai::Result<()> {
/// let link = client.payout_links().create(Default::default()).await?;
/// let same = client.payout_links().info(&link).await?;   // or .info(link.link_id.as_str())
/// # let _ = same; Ok(()) }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdRef(String);

impl IdRef {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<&str> for IdRef {
    fn from(value: &str) -> Self {
        IdRef(value.to_string())
    }
}

impl From<String> for IdRef {
    fn from(value: String) -> Self {
        IdRef(value)
    }
}

impl From<&String> for IdRef {
    fn from(value: &String) -> Self {
        IdRef(value.clone())
    }
}

impl From<&crate::contract::models::PayoutLink> for IdRef {
    fn from(value: &crate::contract::models::PayoutLink) -> Self {
        IdRef(value.link_id.clone())
    }
}

impl From<&crate::contract::models::PaymentLink> for IdRef {
    fn from(value: &crate::contract::models::PaymentLink) -> Self {
        IdRef(value.link_id.clone())
    }
}

impl From<&crate::contract::models::PaymentLinkCreated> for IdRef {
    fn from(value: &crate::contract::models::PaymentLinkCreated) -> Self {
        IdRef(value.link_id.clone())
    }
}

impl From<&crate::contract::models::Payout> for IdRef {
    fn from(value: &crate::contract::models::Payout) -> Self {
        IdRef(value.uuid.clone())
    }
}

impl From<&crate::contract::models::Payment> for IdRef {
    fn from(value: &crate::contract::models::Payment) -> Self {
        IdRef(value.uuid.clone())
    }
}

/// Name an invoice or a payout by its `uuid` or by your own `order_id`; one of them is required.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Lookup {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
}

impl Lookup {
    /// By the gateway's own id.
    pub fn uuid(uuid: impl Into<String>) -> Self {
        Self {
            uuid: Some(uuid.into()),
            order_id: None,
        }
    }

    /// By your reference.
    pub fn order_id(order_id: impl Into<String>) -> Self {
        Self {
            uuid: None,
            order_id: Some(order_id.into()),
        }
    }
}

impl From<&str> for Lookup {
    /// A bare string is taken as the `uuid`.
    fn from(uuid: &str) -> Self {
        Lookup::uuid(uuid)
    }
}

impl From<String> for Lookup {
    fn from(uuid: String) -> Self {
        Lookup::uuid(uuid)
    }
}

impl From<&String> for Lookup {
    fn from(uuid: &String) -> Self {
        Lookup::uuid(uuid.clone())
    }
}

impl From<&crate::contract::models::Payment> for Lookup {
    fn from(payment: &crate::contract::models::Payment) -> Self {
        Lookup::uuid(payment.uuid.clone())
    }
}

impl From<&crate::contract::models::Payout> for Lookup {
    fn from(payout: &crate::contract::models::Payout) -> Self {
        Lookup::uuid(payout.uuid.clone())
    }
}

/// Name an invoice by `uuid` or `order_id`.
pub type PaymentLookup = Lookup;
/// Name a payout by `uuid` or `order_id`.
pub type PayoutLookup = Lookup;
