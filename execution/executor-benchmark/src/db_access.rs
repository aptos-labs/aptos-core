// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_state_view::TStateView;
use aptos_storage_interface::{cached_state_view::CachedStateView, state_view::DbStateView};
use aptos_types::{
    access_path::AccessPath, account_address::AccountAddress, state_store::state_key::StateKey,
};
use move_core_types::{
    identifier::Identifier,
    language_storage::{StructTag, TypeTag},
};
use once_cell::sync::Lazy;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::str::FromStr;

// Note: in case this changes in the future, it doesn't have to be a constant, and can be read from
// genesis directly if necessary.
pub static TOTAL_SUPPLY_STATE_KEY: Lazy<StateKey> = Lazy::new(|| {
    StateKey::table_item(
        "1b854694ae746cdbd8d44186ca4929b2b337df21d1c74633be19b2710552fdca"
            .parse()
            .unwrap(),
        vec![
            6, 25, 220, 41, 160, 170, 200, 250, 20, 103, 20, 5, 142, 141, 214, 210, 208, 243, 189,
            245, 246, 51, 25, 7, 191, 145, 243, 172, 216, 30, 105, 53,
        ],
    )
});

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
        type_params: Vec<TypeTag>,
    ) -> StructTag {
        StructTag {
            address,
            module: Identifier::from_str(module).unwrap(),
            name: Identifier::from_str(name).unwrap(),
            type_params,
        }
    }

    pub fn new_state_key(
        address: AccountAddress,
        resource_address: AccountAddress,
        module: &str,
        name: &str,
        type_params: Vec<TypeTag>,
    ) -> StateKey {
        StateKey::access_path(AccessPath::new(
            address,
            AccessPath::resource_path_vec(Self::new_struct_tag(
                resource_address,
                module,
                name,
                type_params,
            ))
            .expect("access path in test"),
        ))
    }

    pub fn new_state_key_account(address: AccountAddress) -> StateKey {
        Self::new_state_key(address, AccountAddress::ONE, "account", "Account", vec![])
    }

    pub fn new_state_key_aptos_coin(address: AccountAddress) -> StateKey {
        Self::new_state_key(address, AccountAddress::ONE, "coin", "CoinStore", vec![
            TypeTag::Struct(Box::new(Self::new_struct_tag(
                AccountAddress::ONE,
                "aptos_coin",
                "AptosCoin",
                vec![],
            ))),
        ])
    }

    pub fn get_account(
        account_key: &StateKey,
        state_view: &CachedStateView,
    ) -> Result<Option<Account>> {
        Self::get_value(account_key, state_view)
    }

    pub fn get_coin_store(
        coin_store_key: &StateKey,
        state_view: &CachedStateView,
    ) -> Result<Option<CoinStore>> {
        Self::get_value(coin_store_key, state_view)
    }

    pub fn get_value<T: DeserializeOwned>(
        state_key: &StateKey,
        state_view: &CachedStateView,
    ) -> Result<Option<T>> {
        let value = state_view
            .get_state_value_bytes(state_key)?
            .map(move |value| bcs::from_bytes(value.as_slice()));
        value.transpose().map_err(anyhow::Error::msg)
    }

    pub fn get_db_value<T: DeserializeOwned>(
        state_key: &StateKey,
        state_view: &DbStateView,
    ) -> Result<Option<T>> {
        let value = state_view
            .get_state_value(state_key)?
            .map(move |value| bcs::from_bytes(value.into_bytes().as_slice()));
        value.transpose().map_err(anyhow::Error::msg)
    }
}
