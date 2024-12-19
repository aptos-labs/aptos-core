// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_sdk::types::get_apt_primary_store_address;
use aptos_types::{
    account_address::AccountAddress,
    account_config::{
        AccountResource, CoinInfoResource, CoinStoreResource, ConcurrentSupplyResource,
        FungibleStoreResource, ObjectCoreResource, ObjectGroupResource, TypeInfoResource,
    },
    event::{EventHandle, EventKey},
    state_store::{state_key::StateKey, StateView, TStateView},
    write_set::TOTAL_SUPPLY_STATE_KEY,
    AptosCoinType, CoinType,
};
use itertools::Itertools;
use move_core_types::{
    identifier::Identifier,
    language_storage::{StructTag, TypeTag},
    move_resource::MoveStructType,
};
use serde::de::DeserializeOwned;
use std::{collections::BTreeMap, str::FromStr};

pub struct CommonStructTags {
    pub account: StructTag,
    pub apt_coin_store: StructTag,
    pub object_group: StructTag,
    pub object_core: StructTag,
    pub fungible_store: StructTag,
    pub concurrent_supply: StructTag,

    pub apt_coin_type_name: String,

    pub apt_coin_info_resource: StateKey,
}

impl CommonStructTags {
    pub fn new() -> Self {
        Self {
            account: AccountResource::struct_tag(),
            apt_coin_store: CoinStoreResource::<AptosCoinType>::struct_tag(),
            object_group: ObjectGroupResource::struct_tag(),
            object_core: ObjectCoreResource::struct_tag(),
            fungible_store: FungibleStoreResource::struct_tag(),
            concurrent_supply: ConcurrentSupplyResource::struct_tag(),

            apt_coin_type_name: "0x1::aptos_coin::AptosCoin".to_string(),
            apt_coin_info_resource: StateKey::resource_typed::<CoinInfoResource<AptosCoinType>>(
                &AptosCoinType::coin_info_address(),
            )
            .unwrap(),
        }
    }
}

impl Default for CommonStructTags {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DbAccessUtil {
    pub common: CommonStructTags,
}

impl Default for DbAccessUtil {
    fn default() -> Self {
        Self::new()
    }
}

impl DbAccessUtil {
    pub fn new() -> Self {
        Self {
            common: CommonStructTags::new(),
        }
    }

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

    pub fn new_state_key_account(&self, address: &AccountAddress) -> StateKey {
        StateKey::resource(address, &self.common.account).unwrap()
    }

    pub fn new_state_key_aptos_coin(&self, address: &AccountAddress) -> StateKey {
        StateKey::resource(address, &self.common.apt_coin_store).unwrap()
    }

    pub fn new_state_key_object_resource_group(&self, address: &AccountAddress) -> StateKey {
        StateKey::resource_group(address, &self.common.object_group)
    }

    pub fn get_account(
        account_key: &StateKey,
        state_view: &impl StateView,
    ) -> Result<Option<AccountResource>> {
        Self::get_value(account_key, state_view)
    }

    pub fn get_apt_coin_store(
        coin_store_key: &StateKey,
        state_view: &impl StateView,
    ) -> Result<Option<CoinStoreResource<AptosCoinType>>> {
        Self::get_value(coin_store_key, state_view)
    }

    pub fn get_fa_store(
        account: AccountAddress,
        state_view: &impl StateView,
    ) -> Option<FungibleStoreResource> {
        let bytes_opt = TStateView::get_state_value_bytes(
            &state_view,
            &StateKey::resource_group(
                &get_apt_primary_store_address(account),
                &ObjectGroupResource::struct_tag(),
            ),
        )
        .expect("account must exist in data store");
        let group: Option<BTreeMap<StructTag, Vec<u8>>> = bytes_opt
            .map(|bytes| bcs::from_bytes(&bytes))
            .transpose()
            .unwrap();
        group
            .and_then(|g| {
                g.get(&FungibleStoreResource::struct_tag())
                    .map(|b| bcs::from_bytes(b))
            })
            .transpose()
            .unwrap()
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

    pub fn get_total_supply(state_view: &impl StateView) -> Result<Option<u128>> {
        Self::get_value(&TOTAL_SUPPLY_STATE_KEY, state_view)
    }

    pub fn new_account_resource(address: AccountAddress) -> AccountResource {
        AccountResource::new(
            0,
            address.to_vec(),
            EventHandle::new(EventKey::new(1, address), 0),
            EventHandle::new(EventKey::new(2, address), 0),
        )
    }

    pub fn new_apt_coin_store(
        balance: u64,
        address: AccountAddress,
    ) -> CoinStoreResource<AptosCoinType> {
        CoinStoreResource::<AptosCoinType>::new(
            balance,
            false,
            EventHandle::new(EventKey::new(1, address), 0),
            EventHandle::new(EventKey::new(2, address), 0),
        )
    }

    pub fn new_object_core(address: AccountAddress, owner: AccountAddress) -> ObjectCoreResource {
        ObjectCoreResource::new(owner, false, EventHandle::new(EventKey::new(1, address), 0))
    }

    pub fn new_type_info_resource<T: MoveStructType>() -> anyhow::Result<TypeInfoResource> {
        let struct_tag = T::struct_tag();
        Ok(TypeInfoResource {
            account_address: struct_tag.address,
            module_name: bcs::to_bytes(&struct_tag.module.to_string())?,
            struct_name: bcs::to_bytes(
                &if struct_tag.type_args.is_empty() {
                    struct_tag.name.to_string()
                } else {
                    format!(
                        "{}<{}>",
                        struct_tag.name,
                        struct_tag
                            .type_args
                            .iter()
                            .map(|v| v.to_string())
                            .join(", ")
                    )
                    .to_string()
                },
            )?,
        })
    }
}
