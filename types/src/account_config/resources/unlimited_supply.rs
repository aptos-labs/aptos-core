// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::event::EventHandle;
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    move_resource::{MoveResource, MoveStructType},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UnlimitedSupplyResource {
    current_supply: u64,
    total_minted: u64,
    burn_events: EventHandle,
    mint_events: EventHandle,
}

impl UnlimitedSupplyResource {
    pub fn new(
        current_supply: u64,
        total_minted: u64,
        burn_events: EventHandle,
        mint_events: EventHandle,
    ) -> Self {
        Self {
            current_supply,
            total_minted,
            burn_events,
            mint_events,
        }
    }

    pub fn current_supply(&self) -> u64 {
        self.current_supply
    }

    pub fn total_minted(&self) -> u64 {
        self.total_minted
    }

    pub fn burn_events(&self) -> &EventHandle {
        &self.burn_events
    }

    pub fn mint_events(&self) -> &EventHandle {
        &self.mint_events
    }
}

impl MoveStructType for UnlimitedSupplyResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("collection");
    const STRUCT_NAME: &'static IdentStr = ident_str!("UnlimitedSupply");
}

impl MoveResource for UnlimitedSupplyResource {}
