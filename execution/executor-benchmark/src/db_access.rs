// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_storage_interface::state_view::DbStateView;
use aptos_types::{
    account_address::AccountAddress, account_config::{CoinStoreResource, FungibleStoreResource, ObjectGroupResource}, state_store::{state_key::StateKey, StateView}, write_set::TOTAL_SUPPLY_STATE_KEY, AptosCoinType
};
use move_core_types::{
    identifier::Identifier,
    language_storage::{StructTag, TypeTag}, move_resource::MoveStructType,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::BTreeMap, str::FromStr};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CoinStore {
    pub coin: u64,
    pub _frozen: bool,
    pub _deposit_events: EventHandle,
    pub _withdraw_events: EventHandle,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct EventHandle {
    _counter: u64,
    _guid: GUID,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct GUID {
    _creation_num: u64,
    _address: Address,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Account {
    pub authentication_key: Vec<u8>,
    pub sequence_number: u64,
    pub _guid_creation_num: u64,
    pub _coin_register_events: EventHandle,
    pub _key_rotation_events: EventHandle,
    pub _rotation_capability_offer: CapabilityOffer,
    pub _signer_capability_offer: CapabilityOffer,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CapabilityOffer {
    _for_address: Option<Address>,
}

pub type Address = [u8; 32];

pub struct DbAccessUtil;

impl DbAccessUtil {
    pub fn new_struct_tag(
        address: AccountAddress,
        module: &str,
        name: &str,
        type_args: Vec<TypeTag>,
    ) -> StructTag {
        StructTag {
            address,
            module: Identifier::from_str(module).unwrap(),
            name: Identifier::from_str(name).unwrap(),
            type_args,
        }
    }

    pub fn new_state_key(
        address: AccountAddress,
        resource_address: AccountAddress,
        module: &str,
        name: &str,
        type_args: Vec<TypeTag>,
    ) -> StateKey {
        StateKey::resource(
            &address,
            &Self::new_struct_tag(resource_address, module, name, type_args),
        )
        .unwrap()
    }

    pub fn new_state_key_account(address: AccountAddress) -> StateKey {
        Self::new_state_key(address, AccountAddress::ONE, "account", "Account", vec![])
    }

    pub fn new_state_key_aptos_coin(address: &AccountAddress) -> StateKey {
        StateKey::resource_typed::<CoinStoreResource<AptosCoinType>>(address).unwrap()
    }

    pub fn new_state_key_object_resource_group(address: &AccountAddress) -> StateKey {
        StateKey::resource_group(address, &ObjectGroupResource::struct_tag())
    }

    pub fn get_account(
        account_key: &StateKey,
        state_view: &impl StateView,
    ) -> Result<Option<Account>> {
        Self::get_value(account_key, state_view)
    }

    pub fn get_fa_store(
        store_key: &StateKey,
        state_view: &impl StateView,
    ) -> Result<Option<FungibleStoreResource>> {
        Self::get_value(store_key, state_view)
    }

    pub fn get_coin_store(
        coin_store_key: &StateKey,
        state_view: &impl StateView,
    ) -> Result<Option<CoinStore>> {
        Self::get_value(coin_store_key, state_view)
    }

    pub fn get_value<T: DeserializeOwned>(
        state_key: &StateKey,
        state_view: &impl StateView,
    ) -> Result<Option<T>> {
        let value = state_view
            .get_state_value_bytes(state_key)?
            .map(move |value| bcs::from_bytes(&value));
        value.transpose().map_err(anyhow::Error::msg)
    }

    pub fn get_resource_group(
        state_key: &StateKey,
        state_view: &impl StateView,
    ) -> Result<Option<BTreeMap<StructTag, Vec<u8>>>> {
        Self::get_value(state_key, state_view)
    }

    pub fn get_db_value<T: DeserializeOwned>(
        state_key: &StateKey,
        state_view: &DbStateView,
    ) -> Result<Option<T>> {
        Self::get_value(state_key, state_view)
    }

    pub fn get_total_supply(state_view: &impl StateView) -> Result<Option<u128>> {
        Self::get_value(&TOTAL_SUPPLY_STATE_KEY, state_view)
    }
}
