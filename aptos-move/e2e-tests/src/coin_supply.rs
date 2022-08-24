// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::delta_change_set::deserialize;
use aptos_state_view::StateView;
use aptos_types::{
    access_path::AccessPath,
    state_store::{state_key::StateKey, table::TableHandle},
    transaction::{SignedTransaction, TransactionOutput},
    APTOS_COIN_TYPE,
};
use move_deps::move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    language_storage::{ResourceKey, TypeTag},
    move_resource::{MoveResource, MoveStructType},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Aggregator {
    handle: AccountAddress,
    key: AccountAddress,
    limit: u128,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Integer {
    value: u128,
    limit: u128,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OptionalAggregator {
    aggregator: Option<Aggregator>,
    integer: Option<Integer>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoinInfo {
    name: Vec<u8>,
    symbol: Vec<u8>,
    decimals: u8,
    supply: Option<OptionalAggregator>,
}

impl MoveStructType for CoinInfo {
    const MODULE_NAME: &'static IdentStr = ident_str!("coin");
    const STRUCT_NAME: &'static IdentStr = ident_str!("CoinInfo");

    fn type_params() -> Vec<TypeTag> {
        vec![APTOS_COIN_TYPE.clone()]
    }
}

impl MoveResource for CoinInfo {}

pub fn fetch_coin_info(state_view: &impl StateView) -> anyhow::Result<CoinInfo> {
    let addr = AccountAddress::ONE;
    let account_access_path =
        AccessPath::resource_access_path(ResourceKey::new(addr, CoinInfo::struct_tag()));
    let blob = state_view
        .get_state_value(&StateKey::AccessPath(account_access_path))
        .unwrap()
        .ok_or_else(|| {
            anyhow::format_err!("Failed to fetch coin info resource under address {}.", addr)
        })?;
    Ok(bcs::from_bytes(&blob).unwrap())
}

pub fn fetch_coin_supply(state_view: &impl StateView) -> anyhow::Result<u128> {
    let coin_info = fetch_coin_info(state_view)?;
    let supply = coin_info.supply.expect("total supply is not tracked");
    match supply.aggregator {
        Some(agg) => {
            let key_bytes = agg.key.to_vec();
            let state_key =
                StateKey::table_item(TableHandle::from(TableHandle(agg.handle)), key_bytes);

            let value_bytes = state_view.get_state_value(&state_key).unwrap().unwrap();
            Ok(deserialize(&value_bytes))
        }
        None => Ok(supply.integer.unwrap().value),
    }
}

/// Returns the sum of gas fees for executed transactions.
pub fn calculate_gas_fees(txns: &Vec<SignedTransaction>, outputs: &Vec<TransactionOutput>) -> u128 {
    assert!(txns.len() == outputs.len());

    txns.iter()
        .map(|t| t.gas_unit_price())
        .zip(outputs.iter().map(|o| o.gas_used()))
        .map(|(price, used)| (price as u128) * (used as u128))
        .reduce(|a, b| a + b)
        .unwrap()
}
