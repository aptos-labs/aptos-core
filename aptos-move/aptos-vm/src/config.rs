// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::gas::get_gas_parameters;
use aptos_gas_schedule::{NativeGasParameters, VMGasParameters};
use aptos_types::{
    chain_id::ChainId,
    on_chain_config::{
        randomness_api_v0_config::RequiredGasDeposit, ConfigurationResource, FeatureFlag, Features,
        OnChainConfig, TimedFeatureFlag, TimedFeatures, TimedFeaturesBuilder,
    },
    state_store::StateView,
};
use aptos_vm_types::storage::StorageGasParameters;
use move_binary_format::deserializer::DeserializerConfig;
use move_bytecode_verifier::VerifierConfig;
use move_vm_runtime::config::VMConfig;

// TODO: Consider different config levels: permanent, per epoch, per block. Right
//       now everything is per-block because we do not have long-living caches
//       anyway. Decoupling can help with invalidation based on the lifetime.
#[allow(dead_code)]
pub struct BlockVMConfig {
    // Gas related configs.
    gas_feature_version: u64,
    vm_gas_params: VMGasParameters,
    native_gas_params: NativeGasParameters,
    storage_gas_params: StorageGasParameters,

    // Other configs.
    // TODO: VM should not take deserializer config and verifier configs, they are orthogonal!
    vm_config: VMConfig,

    features: Features,
    timed_features: TimedFeatures,
    chain_id: u8,

    randomness_deposit: Option<u64>,
}

impl BlockVMConfig {
    pub fn new(
        state_view: &impl StateView,
        is_delayed_field_optimization_capable: bool,
    ) -> Result<Self, String> {
        let features = Features::fetch_config(state_view).unwrap_or_default();
        let (gas_params, storage_gas_params, gas_feature_version) =
            get_gas_parameters(&features, state_view)?;
        let (vm_gas_params, native_gas_params) = gas_params.unpack();

        // If no chain ID is in storage, we assume we are in a testing environment.
        let chain_id = ChainId::fetch_config(state_view).unwrap_or_else(ChainId::test);

        let timestamp = ConfigurationResource::fetch_config(state_view)
            .map(|config| config.last_reconfiguration_time())
            .unwrap_or(0);
        let mut timed_features_builder = TimedFeaturesBuilder::new(chain_id, timestamp);
        if let Some(profile) = crate::AptosVM::get_timed_feature_override() {
            timed_features_builder = timed_features_builder.with_override_profile(profile)
        }
        let timed_features = timed_features_builder.build();

        let aggregator_v2_type_tagging = is_delayed_field_optimization_capable
            && features.is_aggregator_v2_delayed_fields_enabled();
        let vm_config = aptos_prod_vm_config(
            &features,
            &timed_features,
            gas_feature_version,
            aggregator_v2_type_tagging,
        );

        let randomness_deposit = RequiredGasDeposit::fetch_config(state_view)
            .unwrap_or_else(RequiredGasDeposit::default_if_missing)
            .gas_amount;

        Ok(Self {
            gas_feature_version,
            vm_gas_params,
            native_gas_params,
            storage_gas_params,
            vm_config,
            features,
            timed_features,
            chain_id: chain_id.id(),
            randomness_deposit,
        })
    }
}

pub fn aptos_prod_deserializer_config(
    features: &Features,
    gas_feature_version: u64,
) -> DeserializerConfig {
    DeserializerConfig::new(
        features.get_max_binary_format_version(Some(gas_feature_version)),
        features.get_max_identifier_size(),
    )
}

pub fn aptos_prod_vm_config(
    features: &Features,
    timed_features: &TimedFeatures,
    gas_feature_version: u64,
    aggregator_v2_type_tagging: bool,
) -> VMConfig {
    let enable_invariant_violation_check_in_swap_loc =
        !timed_features.is_enabled(TimedFeatureFlag::DisableInvariantViolationCheckInSwapLoc);

    let mut type_max_cost = 0;
    let mut type_base_cost = 0;
    let mut type_byte_cost = 0;
    if timed_features.is_enabled(TimedFeatureFlag::LimitTypeTagSize) {
        // 5000 limits type tag total size < 5000 bytes and < 50 nodes
        type_max_cost = 5000;
        type_base_cost = 100;
        type_byte_cost = 1;
    }

    VMConfig {
        verifier: aptos_prod_verifier_config(features),
        deserializer_config: aptos_prod_deserializer_config(features, gas_feature_version),
        paranoid_type_checks: crate::AptosVM::get_paranoid_checks(),
        enable_invariant_violation_check_in_swap_loc,
        type_size_limit: true,
        max_value_nest_depth: Some(128),
        type_max_cost,
        type_base_cost,
        type_byte_cost,
        aggregator_v2_type_tagging,
    }
}

pub fn aptos_prod_verifier_config(features: &Features) -> VerifierConfig {
    let use_signature_checker_v2 = features.is_enabled(FeatureFlag::SIGNATURE_CHECKER_V2);
    let sig_checker_v2_fix_script_ty_param_count =
        features.is_enabled(FeatureFlag::SIGNATURE_CHECKER_V2_SCRIPT_FIX);

    VerifierConfig {
        max_loop_depth: Some(5),
        max_generic_instantiation_length: Some(32),
        max_function_parameters: Some(128),
        max_basic_blocks: Some(1024),
        max_value_stack_size: 1024,
        max_type_nodes: Some(256),
        max_dependency_depth: Some(256),
        max_push_size: Some(10000),
        max_struct_definitions: None,
        max_fields_in_struct: None,
        max_function_definitions: None,
        max_back_edges_per_function: None,
        max_back_edges_per_module: None,
        max_basic_blocks_in_script: None,
        max_per_fun_meter_units: Some(1000 * 80000),
        max_per_mod_meter_units: Some(1000 * 80000),
        use_signature_checker_v2,
        sig_checker_v2_fix_script_ty_param_count,
    }
}
