// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access_path::AccessPath,
    account_config::{AccountResource, ChainIdResource, CoinStoreResource, ObjectGroupResource},
    on_chain_config::{ConfigurationResource, OnChainConfig, ValidatorSet, Version},
    state_store::state_key::StateKey,
    validator_config::{ValidatorConfig, ValidatorOperatorConfigResource},
};
use anyhow::anyhow;
use bytes::Bytes;
use move_core_types::{account_address::AccountAddress, move_resource::MoveResource};
use serde::de::DeserializeOwned;
use move_core_types::move_resource::MoveStructType;
use std::collections::BTreeMap;
use move_core_types::language_storage::StructTag;
pub trait AccountView {
    fn get_state_value(&self, state_key: &StateKey) -> anyhow::Result<Option<Bytes>>;

    fn get_account_address(&self) -> anyhow::Result<Option<AccountAddress>>;

    fn get_validator_set(&self) -> anyhow::Result<Option<ValidatorSet>> {
        self.get_on_chain_config::<ValidatorSet>()
    }

    fn get_configuration_resource(&self) -> anyhow::Result<Option<ConfigurationResource>> {
        self.get_move_resource::<ConfigurationResource>()
    }

    fn get_move_resource<T: MoveResource>(&self) -> anyhow::Result<Option<T>> {
        self.get_resource_impl(T::struct_tag().access_vector())
    }

    fn get_validator_config_resource(&self) -> anyhow::Result<Option<ValidatorConfig>> {
        self.get_resource::<ValidatorConfig>()
    }

    fn get_validator_operator_config_resource(
        &self,
    ) -> anyhow::Result<Option<ValidatorOperatorConfigResource>> {
        self.get_resource::<ValidatorOperatorConfigResource>()
    }

    fn get_on_chain_config<T: OnChainConfig>(&self) -> anyhow::Result<Option<T>> {
        self.get_resource_impl(T::access_path()?.path)
    }

    fn get_version(&self) -> anyhow::Result<Option<Version>> {
        self.get_on_chain_config::<Version>()
    }

    fn get_resource<T: MoveResource>(&self) -> anyhow::Result<Option<T>> {
        self.get_resource_impl(T::struct_tag().access_vector())
    }

    fn get_chain_id_resource(&self) -> anyhow::Result<Option<ChainIdResource>> {
        self.get_resource::<ChainIdResource>()
    }

    fn get_coin_store_resource(&self) -> anyhow::Result<Option<CoinStoreResource>> {
        self.get_resource::<CoinStoreResource>()
    }

    fn get_state_key_for_path(&self, path: Vec<u8>) -> anyhow::Result<StateKey> {
        let account_address = self
            .get_account_address()?
            .ok_or_else(|| anyhow!("Could not fetch account address"))?;
        Ok(StateKey::access_path(AccessPath::new(
            account_address,
            path,
        )))
    }

    fn get_account_resource(&self) -> anyhow::Result<Option<AccountResource>> {
        self.get_resource_from_resource_group::<AccountResource>(
            &self.get_account_address()?
            .ok_or_else(|| anyhow!("Could not fetch account address"))?,
            ObjectGroupResource::struct_tag(),
            AccountResource::struct_tag(),
        )

        // self.get_resource::<AccountResource>()
    }

    fn get_config<T: OnChainConfig>(&self) -> anyhow::Result<Option<T>> {
        self.get_resource_impl(T::access_path()?.path)
    }

    fn get_resource_impl<T: DeserializeOwned>(&self, path: Vec<u8>) -> anyhow::Result<Option<T>> {
        self.get_state_value(&self.get_state_key_for_path(path)?)?
            .map(|bytes| bcs::from_bytes(&bytes))
            .transpose()
            .map_err(Into::into)
    }

    fn get_resource_group(
        &self,
        addr: &AccountAddress,
        struct_tag: StructTag,
    ) -> anyhow::Result<Option<BTreeMap<StructTag, Vec<u8>>>> {
        let path = AccessPath::resource_group_access_path(*addr, struct_tag);
        self.get_state_value(&StateKey::access_path(path))?
            .map(|data| bcs::from_bytes(&data))
            .transpose()
            .map_err(Into::into)
    }

    fn get_resource_from_resource_group<T: DeserializeOwned>(
        &self,
        addr: &AccountAddress,
        resource_group: StructTag,
        struct_tag: StructTag,
    ) -> anyhow::Result<Option<T>> {
        if let Some(group) = self.get_resource_group(addr, resource_group)? {
            if let Some(data) = group.get(&struct_tag) {
                return bcs::from_bytes::<T>(data)
                    .map(Some)
                    .map_err(Into::into);
            }
        }
        Ok(None)
    }

}
