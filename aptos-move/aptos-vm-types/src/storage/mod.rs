// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::storage::{
    change_set_configs::ChangeSetConfigs, io_pricing::IoPricing, space_pricing::DiskSpacePricing,
};
use aptos_gas_schedule::{AptosGasParameters, InitialGasSchedule, LATEST_GAS_FEATURE_VERSION};
use aptos_types::{
    access_path::AccessPath,
    on_chain_config::{ConfigStorage, Features},
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
        gas_params: &AptosGasParameters,
        config_storage: &impl ConfigStorage,
    ) -> Self {
        let io_pricing = IoPricing::new(gas_feature_version, gas_params, config_storage);
        let space_pricing = DiskSpacePricing::new(gas_feature_version, features);
        let change_set_configs = ChangeSetConfigs::new(gas_feature_version, gas_params);

        Self {
            io_pricing,
            space_pricing,
            change_set_configs,
        }
    }

    pub fn latest() -> Self {
        struct DummyConfigStorage;

        impl ConfigStorage for DummyConfigStorage {
            fn fetch_config(&self, _access_path: AccessPath) -> Option<Bytes> {
                unreachable!("Not supposed to be called from latest() / tests.")
            }
        }

        Self::new(
            LATEST_GAS_FEATURE_VERSION,
            &Features::default(),
            &AptosGasParameters::initial(),
            &DummyConfigStorage,
        )
    }
}
