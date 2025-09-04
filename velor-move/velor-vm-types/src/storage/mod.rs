// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::storage::{
    change_set_configs::ChangeSetConfigs, io_pricing::IoPricing, space_pricing::DiskSpacePricing,
};
use velor_gas_schedule::{VelorGasParameters, InitialGasSchedule, LATEST_GAS_FEATURE_VERSION};
use velor_types::{
    on_chain_config::{ConfigStorage, Features},
    state_store::state_key::StateKey,
};
use bytes::Bytes;
use std::fmt::Debug;

pub mod change_set_configs;
pub mod io_pricing;
pub mod space_pricing;

#[derive(Clone, Debug)]
pub struct StorageGasParameters {
    pub io_pricing: IoPricing,
    pub space_pricing: DiskSpacePricing,
    pub change_set_configs: ChangeSetConfigs,
}

impl StorageGasParameters {
    pub fn new(
        gas_feature_version: u64,
        features: &Features,
        gas_params: &VelorGasParameters,
        config_storage: &impl ConfigStorage,
    ) -> Self {
        Self::new_impl(
            gas_feature_version,
            features,
            gas_params,
            config_storage,
            ChangeSetConfigs::new(gas_feature_version, gas_params),
        )
    }

    pub fn unlimited() -> Self {
        Self::new_impl(
            LATEST_GAS_FEATURE_VERSION,
            &Features::default(),
            &VelorGasParameters::zeros(), // free of charge
            &DummyConfigStorage,
            ChangeSetConfigs::unlimited_at_gas_feature_version(LATEST_GAS_FEATURE_VERSION), // no limits
        )
    }

    pub fn latest() -> Self {
        Self::new(
            LATEST_GAS_FEATURE_VERSION,
            &Features::default(),
            &VelorGasParameters::initial(),
            &DummyConfigStorage,
        )
    }

    fn new_impl(
        gas_feature_version: u64,
        features: &Features,
        gas_params: &VelorGasParameters,
        config_storage: &impl ConfigStorage,
        change_set_configs: ChangeSetConfigs,
    ) -> Self {
        let io_pricing = IoPricing::new(gas_feature_version, gas_params, config_storage);
        let space_pricing = DiskSpacePricing::new(gas_feature_version, features);

        Self {
            io_pricing,
            space_pricing,
            change_set_configs,
        }
    }
}

struct DummyConfigStorage;

impl ConfigStorage for DummyConfigStorage {
    fn fetch_config_bytes(&self, _state_key: &StateKey) -> Option<Bytes> {
        unreachable!("Not supposed to be called from latest() / tests.")
    }
}
