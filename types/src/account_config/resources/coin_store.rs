// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{event::EventHandle, utility_coin::APTOS_COIN_TYPE};
use move_deps::move_core_types::{
    ident_str,
    identifier::IdentStr,
    language_storage::TypeTag,
    move_resource::{MoveResource, MoveStructType},
};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};

/// The balance resource held under an account.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct CoinStoreResource {
    coin: u64,
    frozen: bool,
    deposit_events: EventHandle,
    withdraw_events: EventHandle,
}

impl CoinStoreResource {
    pub fn new(
        coin: u64,
        frozen: bool,
        deposit_events: EventHandle,
        withdraw_events: EventHandle,
    ) -> Self {
        Self {
            coin,
            frozen,
            deposit_events,
            withdraw_events,
        }
    }

    pub fn coin(&self) -> u64 {
        self.coin
    }

    pub fn frozen(&self) -> bool {
        self.frozen
    }

    pub fn deposit_events(&self) -> &EventHandle {
        &self.deposit_events
    }

    pub fn withdraw_events(&self) -> &EventHandle {
        &self.withdraw_events
    }
}

impl MoveStructType for CoinStoreResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("coin");
    const STRUCT_NAME: &'static IdentStr = ident_str!("CoinStore");

    fn type_params() -> Vec<TypeTag> {
        vec![APTOS_COIN_TYPE.clone()]
    }
}

impl MoveResource for CoinStoreResource {}
