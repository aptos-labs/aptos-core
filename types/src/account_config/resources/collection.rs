// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::event::EventHandle;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    move_resource::{MoveResource, MoveStructType},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CollectionResource {
    creator: AccountAddress,
    description: String,
    name: String,
    uri: String,
    mutation_events: EventHandle,
}

impl CollectionResource {
    pub fn new(
        creator: AccountAddress,
        description: String,
        name: String,
        uri: String,
        mutation_events: EventHandle,
    ) -> Self {
        Self {
            creator,
            description,
            name,
            uri,
            mutation_events,
        }
    }

    pub fn creator(&self) -> &AccountAddress {
        &self.creator
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

impl MoveStructType for CollectionResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("collection");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Collection");
}

impl MoveResource for CollectionResource {}
