// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::aggregator::{AggregatorV1Resource, OptionalAggregatorV1Resource};
use crate::{
    state_store::state_key::StateKey,
    write_set::{WriteOp, WriteSet, WriteSetMut},
    CoinType,
};
use move_core_types::{
    account_address::AccountAddress,
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

    /// Returns a new CoinInfo instance. Aggregator that tracks supply is
    /// initialized with random handle/key. This function is useful if we
    /// want to add CoinInfo to the fake data store.
    pub fn random(limit: u128) -> Self {
        let handle = AccountAddress::random();
        let key = AccountAddress::random();
        Self::new(handle, key, limit)
    }

    /// Returns a new CoinInfo instance. This function is useful if we want to
    /// add CoinInfo to the fake data store.
    pub fn new(handle: AccountAddress, key: AccountAddress, limit: u128) -> Self {
        let aggregator = OptionalAggregatorV1Resource {
            aggregator: Some(AggregatorV1Resource::new(handle, key, limit)),
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
    pub fn to_writeset(&self, supply: u128) -> anyhow::Result<WriteSet> {
        let value_state_key = self
            .supply
            .as_ref()
            .unwrap()
            .aggregator
            .as_ref()
            .unwrap()
            .state_key();

        // We store CoinInfo and aggregatable value separately.
        let write_set = vec![
            (
                StateKey::resource_typed::<Self>(&C::coin_info_address())?,
                WriteOp::legacy_modification(bcs::to_bytes(&self).unwrap().into()),
            ),
            (
                value_state_key,
                WriteOp::legacy_modification(bcs::to_bytes(&supply).unwrap().into()),
            ),
        ];
        Ok(WriteSetMut::new(write_set).freeze().unwrap())
    }
}
