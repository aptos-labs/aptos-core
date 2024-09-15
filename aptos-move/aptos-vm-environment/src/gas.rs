// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_schedule::{AptosGasParameters, FromOnChainGasSchedule};
use aptos_types::{
    on_chain_config::{Features, GasSchedule, GasScheduleV2, OnChainConfig},
    state_store::StateView,
};
use aptos_vm_types::storage::{io_pricing::IoPricing, StorageGasParameters};
use move_core_types::gas_algebra::NumArgs;

/// Returns the gas parameters and the gas feature version from the state. If no gas parameters are
/// found, returns an error.
pub fn get_gas_config_from_storage(
    state_view: &impl StateView,
) -> (Result<AptosGasParameters, String>, u64) {
    match GasScheduleV2::fetch_config(state_view) {
        Some(gas_schedule) => {
            let feature_version = gas_schedule.feature_version;
            let map = gas_schedule.into_btree_map();
            (
                AptosGasParameters::from_on_chain_gas_schedule(&map, feature_version),
                feature_version,
            )
        },
        None => match GasSchedule::fetch_config(state_view) {
            Some(gas_schedule) => {
                let map = gas_schedule.into_btree_map();
                (AptosGasParameters::from_on_chain_gas_schedule(&map, 0), 0)
            },
            None => (Err("Neither gas schedule v2 nor v1 exists.".to_string()), 0),
        },
    }
}

/// Returns gas and storage gas parameters, as well as the gas feature version, from the state. In
/// case parameters are not found on-chain, errors are returned.
pub fn get_gas_parameters(
    features: &Features,
    state_view: &impl StateView,
) -> (
    Result<AptosGasParameters, String>,
    Result<StorageGasParameters, String>,
    u64,
) {
    let (mut gas_params, gas_feature_version) = get_gas_config_from_storage(state_view);

    let storage_gas_params = match &mut gas_params {
        Ok(gas_params) => {
            let storage_gas_params =
                StorageGasParameters::new(gas_feature_version, features, gas_params, state_view);

            // TODO(gas): Table extension utilizes IoPricing directly.
            // Overwrite table io gas parameters with global io pricing.
            let g = &mut gas_params.natives.table;
            match gas_feature_version {
                0..=1 => (),
                2..=6 => {
                    if let IoPricing::V2(pricing) = &storage_gas_params.io_pricing {
                        g.common_load_base_legacy = pricing.per_item_read * NumArgs::new(1);
                        g.common_load_base_new = 0.into();
                        g.common_load_per_byte = pricing.per_byte_read;
                        g.common_load_failure = 0.into();
                    }
                }
                7..=9 => {
                    if let IoPricing::V2(pricing) = &storage_gas_params.io_pricing {
                        g.common_load_base_legacy = 0.into();
                        g.common_load_base_new = pricing.per_item_read * NumArgs::new(1);
                        g.common_load_per_byte = pricing.per_byte_read;
                        g.common_load_failure = 0.into();
                    }
                }
                10.. => {
                    g.common_load_base_legacy = 0.into();
                    g.common_load_base_new = gas_params.vm.txn.storage_io_per_state_slot_read * NumArgs::new(1);
                    g.common_load_per_byte = gas_params.vm.txn.storage_io_per_state_byte_read;
                    g.common_load_failure = 0.into();
                }
            };
            Ok(storage_gas_params)
        },
        Err(err) => Err(format!("Failed to initialize storage gas params due to failure to load main gas parameters: {}", err)),
    };

    (gas_params, storage_gas_params, gas_feature_version)
}
