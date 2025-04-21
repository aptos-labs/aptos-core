// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{account_config::Table, event::EventHandle};
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    move_resource::{MoveResource, MoveStructType},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectionsResource {
    collection_data: Table,
    token_data: Table,
    create_collection_events: EventHandle,
    create_token_data_events: EventHandle,
    mint_token_events: EventHandle,
}

impl CollectionsResource {
    pub fn new(
        collection_data: Table,
        token_data: Table,
        create_collection_events: EventHandle,
        create_token_data_events: EventHandle,
        mint_token_events: EventHandle,
    ) -> Self {
        Self {
            collection_data,
            token_data,
            create_collection_events,
            create_token_data_events,
            mint_token_events,
        }
    }

    pub fn collection_data(&self) -> &Table {
        &self.collection_data
    }

    pub fn token_data(&self) -> &Table {
        &self.token_data
    }

    pub fn create_collection_events(&self) -> &EventHandle {
        &self.create_collection_events
    }

    pub fn create_token_data_events(&self) -> &EventHandle {
        &self.create_token_data_events
    }

    pub fn mint_token_events(&self) -> &EventHandle {
        &self.mint_token_events
    }
}

impl MoveStructType for CollectionsResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("token");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Collections");
}

impl MoveResource for CollectionsResource {}
