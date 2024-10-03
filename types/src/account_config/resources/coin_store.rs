// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{event::EventHandle, CoinType};
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    language_storage::TypeTag,
    move_resource::{MoveResource, MoveStructType},
};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// The balance resource held under an account.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct CoinStoreResourceUntyped {
    coin: u64,
    frozen: bool,
    deposit_events: EventHandle,
    withdraw_events: EventHandle,
}

impl CoinStoreResourceUntyped {
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

// Separate out typed info that goes with "key", to not require CoinInfoResource to be typed when not needed
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct CoinStoreResource<C: CoinType> {
    coin: u64,
    frozen: bool,
    deposit_events: EventHandle,
    withdraw_events: EventHandle,
    #[serde(skip)]
    phantom_data: PhantomData<C>,
}

impl<C: CoinType> CoinStoreResource<C> {
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
            phantom_data: PhantomData,
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

impl<C: CoinType> MoveStructType for CoinStoreResource<C> {
    const MODULE_NAME: &'static IdentStr = ident_str!("coin");
    const STRUCT_NAME: &'static IdentStr = ident_str!("CoinStore");

    fn type_args() -> Vec<TypeTag> {
        vec![C::type_tag()]
    }
}

impl<C: CoinType> MoveResource for CoinStoreResource<C> {}
