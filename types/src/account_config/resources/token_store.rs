// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::event::EventHandle;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    move_resource::{MoveResource, MoveStructType},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenStoreResource {
    tokens: Table,
    direct_transfer: bool,
    deposit_events: EventHandle,
    withdraw_events: EventHandle,
    burn_events: EventHandle,
    mutate_token_property_events: EventHandle,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Table {
    handle: AccountAddress,
}

impl TokenStoreResource {
    pub fn new(
        tokens: Table,
        direct_transfer: bool,
        deposit_events: EventHandle,
        withdraw_events: EventHandle,
        burn_events: EventHandle,
        mutate_token_property_events: EventHandle,
    ) -> Self {
        Self {
            tokens,
            direct_transfer,
            deposit_events,
            withdraw_events,
            burn_events,
            mutate_token_property_events,
        }
    }

    pub fn tokens(&self) -> &Table {
        &self.tokens
    }

    pub fn direct_transfer(&self) -> bool {
        self.direct_transfer
    }

    pub fn deposit_events(&self) -> &EventHandle {
        &self.deposit_events
    }

    pub fn withdraw_events(&self) -> &EventHandle {
        &self.withdraw_events
    }

    pub fn burn_events(&self) -> &EventHandle {
        &self.burn_events
    }

    pub fn mutate_token_property_events(&self) -> &EventHandle {
        &self.mutate_token_property_events
    }
}

impl MoveStructType for TokenStoreResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("token");
    const STRUCT_NAME: &'static IdentStr = ident_str!("TokenStore");
}

impl MoveResource for TokenStoreResource {}
