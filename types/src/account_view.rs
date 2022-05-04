// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access_path::AccessPath,
    account_config::{AccountResource, BalanceResource, CRSNResource, ChainIdResource},
    on_chain_config::{
        access_path_for_config, ConfigurationResource, OnChainConfig, ValidatorSet, Version,
    },
    state_store::state_key::StateKey,
    validator_config::ValidatorConfig,
};
use move_core_types::{account_address::AccountAddress, move_resource::MoveResource};
use serde::de::DeserializeOwned;

pub trait AccountView {
    fn get_state_value(&self, state_key: &StateKey) -> anyhow::Result<Option<Vec<u8>>>;

    fn get_account_address(&self) -> &AccountAddress;

    fn get_validator_set(&self) -> anyhow::Result<Option<ValidatorSet>> {
        self.get_on_chain_config::<ValidatorSet>()
    }

    fn get_configuration_resource(&self) -> anyhow::Result<Option<ConfigurationResource>> {
        self.get_move_resource::<ConfigurationResource>()
    }

    fn get_move_resource<T: MoveResource>(&self) -> anyhow::Result<Option<T>> {
        let state_key = self.get_state_key_for_path(T::struct_tag().access_vector());
        self.get_resource_impl(&state_key)
    }

    fn get_validator_config_resource(&self) -> anyhow::Result<Option<ValidatorConfig>> {
        self.get_resource::<ValidatorConfig>()
    }

    fn get_on_chain_config<T: OnChainConfig>(&self) -> anyhow::Result<Option<T>> {
        let state_key = self.get_state_key_for_path(access_path_for_config(T::CONFIG_ID).path);
        self.get_resource_impl(&state_key)
    }

    fn get_version(&self) -> anyhow::Result<Option<Version>> {
        self.get_on_chain_config::<Version>()
    }

    fn get_resource<T: MoveResource>(&self) -> anyhow::Result<Option<T>> {
        self.get_resource_impl(&self.get_state_key_for_path(T::struct_tag().access_vector()))
    }

    fn get_chain_id_resource(&self) -> anyhow::Result<Option<ChainIdResource>> {
        self.get_resource::<ChainIdResource>()
    }

    fn get_crsn_resource(&self) -> anyhow::Result<Option<CRSNResource>> {
        self.get_resource::<CRSNResource>()
    }

    fn get_balance_resource(&self) -> anyhow::Result<Option<BalanceResource>> {
        self.get_resource::<BalanceResource>()
    }

    fn get_state_key_for_path(&self, path: Vec<u8>) -> StateKey {
        StateKey::AccessPath(AccessPath::new(*self.get_account_address(), path))
    }

    fn get_account_resource(&self) -> anyhow::Result<Option<AccountResource>> {
        self.get_resource::<AccountResource>()
    }

    fn get_resource_impl<T: DeserializeOwned>(
        &self,
        state_key: &StateKey,
    ) -> anyhow::Result<Option<T>> {
        self.get_state_value(state_key)?
            .map(|bytes| bcs::from_bytes(&bytes))
            .transpose()
            .map_err(Into::into)
    }
}
