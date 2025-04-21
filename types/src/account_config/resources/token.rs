// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{account_config::Object, event::EventHandle};
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    move_resource::{MoveResource, MoveStructType},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenResource {
    pub collection: Object,
    pub index: u64,
    description: String,
    name: String,
    uri: String,
    mutation_events: EventHandle,
}

impl TokenResource {
    pub fn new(
        collection: Object,
        index: u64,
        description: String,
        name: String,
        uri: String,
        mutation_events: EventHandle,
    ) -> Self {
        Self {
            collection,
            index,
            description,
            name,
            uri,
            mutation_events,
        }
    }

    pub fn collection(&self) -> &Object {
        &self.collection
    }

    pub fn index(&self) -> &u64 {
        &self.index
    }

    pub fn description(&self) -> &String {
        &self.description
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn uri(&self) -> &String {
        &self.uri
    }

    pub fn mutation_events(&self) -> &EventHandle {
        &self.mutation_events
    }
}

impl MoveStructType for TokenResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("token");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Token");
}

impl MoveResource for TokenResource {}
