// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{account_config::AnyResource, event::EventHandle};
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    move_resource::{MoveResource, MoveStructType},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenEventStoreV1Resource {
    collection_uri_mutate_events: EventHandle,
    collection_maximum_mutate_events: EventHandle,
    collection_description_mutate_events: EventHandle,
    opt_in_events: EventHandle,
    uri_mutate_events: EventHandle,
    default_property_mutate_events: EventHandle,
    description_mutate_events: EventHandle,
    royalty_mutate_events: EventHandle,
    maximum_mutate_events: EventHandle,
    extension: Option<AnyResource>,
}

impl TokenEventStoreV1Resource {
    pub fn new(
        collection_uri_mutate_events: EventHandle,
        collection_maximum_mutate_events: EventHandle,
        collection_description_mutate_events: EventHandle,
        opt_in_events: EventHandle,
        uri_mutate_events: EventHandle,
        default_property_mutate_events: EventHandle,
        description_mutate_events: EventHandle,
        royalty_mutate_events: EventHandle,
        maximum_mutate_events: EventHandle,
        extension: Option<AnyResource>,
    ) -> Self {
        Self {
            collection_uri_mutate_events,
            collection_maximum_mutate_events,
            collection_description_mutate_events,
            opt_in_events,
            uri_mutate_events,
            default_property_mutate_events,
            description_mutate_events,
            royalty_mutate_events,
            maximum_mutate_events,
            extension,
        }
    }

    pub fn collection_uri_mutate_events(&self) -> &EventHandle {
        &self.collection_uri_mutate_events
    }

    pub fn collection_maximum_mutate_events(&self) -> &EventHandle {
        &self.collection_maximum_mutate_events
    }

    pub fn collection_description_mutate_events(&self) -> &EventHandle {
        &self.collection_description_mutate_events
    }

    pub fn opt_in_events(&self) -> &EventHandle {
        &self.opt_in_events
    }

    pub fn uri_mutate_events(&self) -> &EventHandle {
        &self.uri_mutate_events
    }

    pub fn default_property_mutate_events(&self) -> &EventHandle {
        &self.default_property_mutate_events
    }

    pub fn description_mutate_events(&self) -> &EventHandle {
        &self.description_mutate_events
    }

    pub fn royalty_mutate_events(&self) -> &EventHandle {
        &self.royalty_mutate_events
    }

    pub fn maximum_mutate_events(&self) -> &EventHandle {
        &self.maximum_mutate_events
    }

    pub fn extension(&self) -> &Option<AnyResource> {
        &self.extension
    }
}

impl MoveStructType for TokenEventStoreV1Resource {
    const MODULE_NAME: &'static IdentStr = ident_str!("token_event_store");
    const STRUCT_NAME: &'static IdentStr = ident_str!("TokenEventStoreV1");
}

impl MoveResource for TokenEventStoreV1Resource {}
