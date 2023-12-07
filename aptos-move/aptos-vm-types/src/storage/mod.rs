// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::storage::{
    change_set_configs::ChangeSetConfigs,
    pricing::{StoragePricing, StoragePricingV3},
};
use aptos_gas_schedule::{AptosGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_types::on_chain_config::ConfigStorage;
use move_core_types::gas_algebra::NumBytes;
use std::fmt::Debug;

pub mod change_set_configs;
pub mod pricing;

#[derive(Clone, Debug)]
pub struct StorageGasParameters {
    pub pricing: StoragePricing,
    pub change_set_configs: ChangeSetConfigs,
}

impl StorageGasParameters {
    pub fn new(
        feature_version: u64,
        gas_params: &AptosGasParameters,
        config_storage: &impl ConfigStorage,
    ) -> Self {
        let pricing = StoragePricing::new(feature_version, gas_params, config_storage);
        let change_set_configs = ChangeSetConfigs::new(feature_version, gas_params);

        Self {
            pricing,
            change_set_configs,
        }
    }

    pub fn unlimited(free_write_bytes_quota: NumBytes) -> Self {
        Self {
            pricing: StoragePricing::V3(StoragePricingV3 {
                feature_version: LATEST_GAS_FEATURE_VERSION,
                free_write_bytes_quota,
            }),
            change_set_configs: ChangeSetConfigs::unlimited_at_gas_feature_version(
                LATEST_GAS_FEATURE_VERSION,
            ),
        }
    }
}
