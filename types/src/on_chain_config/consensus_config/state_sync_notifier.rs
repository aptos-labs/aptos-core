use crate::contract_event::ContractEvent;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// When consensus notifies state sync of a commit, this filter is applied to the all the transaction events.
/// This way we can control what transaction events can be subscribed by validator components.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum EventFilter {
    ReconfigOnly,
    TypeNameAllowlist(BTreeSet<String>),
}

impl EventFilter {
    pub fn should_notify(&self, event: &ContractEvent) -> bool {
        match self {
            EventFilter::ReconfigOnly => event.is_new_epoch_event(),
            EventFilter::TypeNameAllowlist(allowlist) => {
                allowlist.contains(&event.type_tag().to_string())
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct StateSyncNotifierConfig {
    pub event_filter: EventFilter,
}

impl StateSyncNotifierConfig {
    pub fn default_for_genesis() -> Self {
        Self {
            event_filter: EventFilter::TypeNameAllowlist(BTreeSet::from([
                "0x1::reconfiguration::NewEpochEvent".to_string(),
                "0x1::dkg::StartDKGEvent".to_string(),
                "0x1::jwks::OnChainJWKMapUpdated".to_string(),
            ])),
        }
    }

    pub fn default_if_missing() -> Self {
        Self {
            event_filter: EventFilter::ReconfigOnly,
        }
    }
}

impl Default for StateSyncNotifierConfig {
    fn default() -> Self {
        Self::default_if_missing()
    }
}
