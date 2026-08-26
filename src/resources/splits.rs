//! Revenue splits: a percentage of every payment forwarded to a partner. Payout key.

use serde_json::json;

use super::base::{Call, RequestBuilder};
use crate::contract::models::{OkResult, SplitConfig, SplitOptIn, SplitRule};
use crate::contract::requests::{SplitConfigSetRequest, SplitRuleListRequest, SplitRuleRequest};
use crate::contract::routes;
use crate::core::pagination::Pager;

/// The `splits` namespace.
#[derive(Clone, Debug)]
pub struct Splits<Tr> {
    transport: Tr,
}

impl<Tr: Clone> Splits<Tr> {
    pub(crate) fn new(transport: Tr) -> Self {
        Self { transport }
    }

    /// `POST /v1/split/rule` — to an external address (`address` + `network`) or to a platform
    /// merchant (`merchant_id`). **Payout key.**
    ///
    /// Codes to branch on: `split.disabled`, `split.bad_percent`, `split.bad_destination`,
    /// `split.self_destination`, `split.duplicate_destination`, `split.dest_not_found`,
    /// `split.recipient_not_opted_in`, `split.network_required`, `merchant.wrong_key_kind`.
    pub fn create_rule(&self, params: SplitRuleRequest) -> RequestBuilder<Tr, SplitRule> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_SPLIT_RULE,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/split/rule/list`.
    pub fn list_rules(&self, params: SplitRuleListRequest) -> Pager<Tr, SplitRule> {
        Pager::new(
            self.transport.clone(),
            &routes::POST_V1_SPLIT_RULE_LIST,
            super::base::to_value(&params),
        )
    }

    /// `POST /v1/split/rule/delete`.
    pub fn delete_rule(&self, rule_id: impl Into<String>) -> RequestBuilder<Tr, OkResult> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_SPLIT_RULE_DELETE,
            Call::new()
                .json(json!({ "rule_id": rule_id.into() }))
                .done(),
        )
    }

    /// `POST /v1/split/config/get`.
    pub fn get_config(&self) -> RequestBuilder<Tr, SplitConfig> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_SPLIT_CONFIG_GET,
            Call::new().done(),
        )
    }

    /// `POST /v1/split/config/set` — how long split shares are held back for refunds.
    pub fn set_config(&self, params: SplitConfigSetRequest) -> RequestBuilder<Tr, SplitConfig> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_SPLIT_CONFIG_SET,
            Call::new().body(&params).done(),
        )
    }

    /// `POST /v1/split/recipient/optin/get` — whether this merchant accepts being a split
    /// recipient.
    pub fn get_opt_in(&self) -> RequestBuilder<Tr, SplitOptIn> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_SPLIT_RECIPIENT_OPTIN_GET,
            Call::new().done(),
        )
    }

    /// `POST /v1/split/recipient/optin`.
    pub fn set_opt_in(&self, enabled: bool) -> RequestBuilder<Tr, SplitOptIn> {
        RequestBuilder::new(
            self.transport.clone(),
            &routes::POST_V1_SPLIT_RECIPIENT_OPTIN,
            Call::new().json(json!({ "enabled": enabled })).done(),
        )
    }
}
