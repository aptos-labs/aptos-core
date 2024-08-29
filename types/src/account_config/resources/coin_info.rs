// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::aggregator::{AggregatorResource, OptionalAggregatorV1Resource};
use crate::{
    state_store::state_key::StateKey,
    write_set::{WriteOp, WriteSet, WriteSetMut},
    CoinType,
};
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    language_storage::TypeTag,
    move_resource::{MoveResource, MoveStructType},
};
use serde::{Deserialize, Serialize};
use std::{marker::PhantomData, string::FromUtf8Error, u128};

#[derive(Debug, Serialize, Deserialize)]
pub struct CoinInfoResource<C: CoinType> {
    name: Vec<u8>,
    symbol: Vec<u8>,
    decimals: u8,
    supply: Option<OptionalAggregatorV1Resource>,
    #[serde(skip)]
    phantom_data: PhantomData<C>,
}

impl<C: CoinType> MoveStructType for CoinInfoResource<C> {
    const MODULE_NAME: &'static IdentStr = ident_str!("coin");
    const STRUCT_NAME: &'static IdentStr = ident_str!("CoinInfo");

    fn type_args() -> Vec<TypeTag> {
        vec![C::type_tag()]
    }
}

impl<C: CoinType> MoveResource for CoinInfoResource<C> {}

impl<C: CoinType> CoinInfoResource<C> {
    pub fn symbol(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.symbol.clone())
    }

    pub fn decimals(&self) -> u8 {
        self.decimals
    }

    pub fn supply(&self) -> &Option<OptionalAggregatorV1Resource> {
        &self.supply
    }

    /// Returns a new CoinInfo instance. This function is useful if we want to
    /// add CoinInfo to the fake data store.
    pub fn new_apt() -> Self {
        let aggregator = OptionalAggregatorV1Resource {
            aggregator: None,
            integer: None,
        };
        Self {
            name: "AptosCoin".to_string().into_bytes(),
            symbol: "APT".to_string().into_bytes(),
            decimals: 8,
            supply: Some(aggregator),
            phantom_data: PhantomData,
        }
    }

    /// Returns a writeset corresponding to the creation of CoinInfo in Move.
    /// This can be passed to data store for testing total supply.
    pub fn to_writeset(&self) -> anyhow::Result<WriteSet> {
        // We store CoinInfo and aggregatable value separately.
        let write_set = vec![(
            StateKey::resource_typed::<Self>(&C::coin_info_address())?,
            WriteOp::legacy_modification(bcs::to_bytes(&self).unwrap().into()),
        )];
        Ok(WriteSetMut::new(write_set).freeze().unwrap())
    }
}

// Separate out typed info that goes with "key", to not require CoinInfoResource to be typed when not needed
#[derive(Debug, Serialize, Deserialize)]
pub struct CoinSupplyResource<C: CoinType> {
    supply: AggregatorResource<u128>,
    phantom_data: PhantomData<C>,
}
impl<C: CoinType> MoveStructType for CoinSupplyResource<C> {
    const MODULE_NAME: &'static IdentStr = ident_str!("coin");
    const STRUCT_NAME: &'static IdentStr = ident_str!("CoinSupply");

    fn type_args() -> Vec<TypeTag> {
        vec![C::type_tag()]
    }
}

impl<C: CoinType> MoveResource for CoinSupplyResource<C> {}

impl<C: CoinType> CoinSupplyResource<C> {
    pub fn new(supply: u128) -> Self {
        Self {
            supply: AggregatorResource::new(supply, u128::MAX),
            phantom_data: PhantomData,
        }
    }

    pub fn get(&self) -> u128 {
        *self.supply.get()
    }

    pub fn set(&mut self, new_supply: u128) {
        self.supply.set(new_supply);
    }

    pub fn to_writeset(&self) -> anyhow::Result<WriteSet> {
        let write_set = vec![(
            StateKey::resource_typed::<CoinSupplyResource<C>>(&C::coin_info_address())?,
            WriteOp::legacy_modification(bcs::to_bytes(&self).unwrap().into()),
        )];
        Ok(WriteSetMut::new(write_set).freeze().unwrap())
    }
}
