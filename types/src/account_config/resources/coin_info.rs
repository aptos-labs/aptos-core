// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    state_store::{state_key::StateKey, table::TableHandle},
    utility_coin::APTOS_COIN_TYPE,
};
use move_deps::move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    language_storage::TypeTag,
    move_resource::{MoveResource, MoveStructType},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Aggregator {
    handle: AccountAddress,
    key: AccountAddress,
    limit: u128,
}

impl Aggregator {
    pub fn state_key(&self) -> StateKey {
        let key_bytes = self.key.to_vec();
        StateKey::table_item(TableHandle(self.handle), key_bytes)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Integer {
    pub value: u128,
    limit: u128,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OptionalAggregator {
    pub aggregator: Option<Aggregator>,
    pub integer: Option<Integer>,
}

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
    pub fn supply(&self) -> &Option<OptionalAggregator> {
        &self.supply
    }
}
