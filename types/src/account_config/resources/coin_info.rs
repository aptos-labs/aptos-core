// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access_path::AccessPath,
    state_store::{state_key::StateKey, table::TableHandle},
    utility_coin::APTOS_COIN_TYPE,
    write_set::{WriteOp, WriteSet, WriteSetMut},
};
use move_deps::move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    language_storage::{ResourceKey, TypeTag},
    move_resource::{MoveResource, MoveStructType},
};
use serde::{Deserialize, Serialize};
use std::string::FromUtf8Error;

/// Rust representation of Aggregator Move struct.
#[derive(Debug, Serialize, Deserialize)]
pub struct Aggregator {
    handle: AccountAddress,
    key: AccountAddress,
    limit: u128,
}

impl Aggregator {
    pub fn new(handle: AccountAddress, key: AccountAddress, limit: u128) -> Self {
        Self { handle, key, limit }
    }

    /// Helper function to return the state key where the actual value is stored.
    pub fn state_key(&self) -> StateKey {
        let key_bytes = self.key.to_vec();
        StateKey::table_item(TableHandle(self.handle), key_bytes)
    }
}

/// Rust representation of Integer Move struct.
#[derive(Debug, Serialize, Deserialize)]
pub struct Integer {
    pub value: u128,
    limit: u128,
}

/// Rust representation of OptionalAggregator Move struct.
#[derive(Debug, Serialize, Deserialize)]
pub struct OptionalAggregator {
    pub aggregator: Option<Aggregator>,
    pub integer: Option<Integer>,
}

/// Rust representation of CoinInfo Move resource.
#[derive(Debug, Serialize, Deserialize)]
pub struct CoinInfoResource {
    name: Vec<u8>,
    symbol: Vec<u8>,
    decimals: u8,
    supply: Option<OptionalAggregator>,
}

impl MoveStructType for CoinInfoResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("coin");
    const STRUCT_NAME: &'static IdentStr = ident_str!("CoinInfo");

    fn type_params() -> Vec<TypeTag> {
        vec![APTOS_COIN_TYPE.clone()]
    }
}

impl MoveResource for CoinInfoResource {}

impl CoinInfoResource {
    pub fn symbol(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.symbol.clone())
    }

    pub fn decimals(&self) -> u8 {
        self.decimals
    }

    pub fn supply(&self) -> &Option<OptionalAggregator> {
        &self.supply
    }

    /// Returns a new CoinInfo instance. Aggregator that tracks supply is
    /// initialized with random handle/key. This function is useful if we
    /// want to add CoinInfo to the fake data store.
    pub fn random(limit: u128) -> Self {
        let handle = AccountAddress::random();
        let key = AccountAddress::random();
        CoinInfoResource::new(handle, key, limit)
    }

    /// Returns a new CoinInfo instance. This function is useful if we want to
    /// add CoinInfo to the fake data store.
    pub fn new(handle: AccountAddress, key: AccountAddress, limit: u128) -> Self {
        let aggregator = OptionalAggregator {
            aggregator: Some(Aggregator::new(handle, key, limit)),
            integer: None,
        };
        Self {
            name: "AptosCoin".to_string().into_bytes(),
            symbol: "APT".to_string().into_bytes(),
            decimals: 8,
            supply: Some(aggregator),
        }
    }

    /// Returns a writeset corresponding to the creation of CoinInfo in Move.
    /// This can be passed to data store for testing total supply.
    pub fn to_writeset(&self) -> WriteSet {
        let tag = ResourceKey::new(AccountAddress::ONE, CoinInfoResource::struct_tag());
        let ap = AccessPath::resource_access_path(tag);

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
                StateKey::AccessPath(ap),
                WriteOp::Modification(bcs::to_bytes(&self).unwrap()),
            ),
            (
                value_state_key,
                WriteOp::Modification(bcs::to_bytes(&0_u128).unwrap()),
            ),
        ];
        WriteSetMut::new(write_set).freeze().unwrap()
    }
}
