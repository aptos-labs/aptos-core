// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::{
    randomness_api_v0_config::{AllowCustomMaxGasFlag, RequiredGasDeposit},
    ConfigStorage, FeatureFlag, Features, OnChainConfig, TimedFeatureFlag, TimedFeatureOverride,
    TimedFeatures,
};
use move_binary_format::deserializer::DeserializerConfig;
use move_bytecode_verifier::VerifierConfig;
use move_vm_runtime::config::VMConfig;
use move_vm_types::loaded_data::runtime_types::TypeBuilder;
use once_cell::sync::OnceCell;

static PARANOID_TYPE_CHECKS: OnceCell<bool> = OnceCell::new();
static TIMED_FEATURE_OVERRIDE: OnceCell<TimedFeatureOverride> = OnceCell::new();

pub fn set_paranoid_type_checks(enable: bool) {
    PARANOID_TYPE_CHECKS.set(enable).ok();
}

/// Get the paranoid type check flag if already set, otherwise default to true.
pub fn get_paranoid_type_checks() -> bool {
    PARANOID_TYPE_CHECKS.get().cloned().unwrap_or(true)
}

pub fn set_timed_feature_override(profile: TimedFeatureOverride) {
    TIMED_FEATURE_OVERRIDE.set(profile).ok();
}

pub fn get_timed_feature_override() -> Option<TimedFeatureOverride> {
    TIMED_FEATURE_OVERRIDE.get().cloned()
}

pub fn aptos_prod_deserializer_config(features: &Features) -> DeserializerConfig {
    DeserializerConfig::new(
        features.get_max_binary_format_version(),
        features.get_max_identifier_size(),
    )
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

pub fn aptos_prod_vm_config(
    features: &Features,
    timed_features: &TimedFeatures,
    delayed_field_optimization_enabled: bool,
    ty_builder: TypeBuilder,
) -> VMConfig {
    let check_invariant_in_swap_loc =
        !timed_features.is_enabled(TimedFeatureFlag::DisableInvariantViolationCheckInSwapLoc);
    let paranoid_type_checks = get_paranoid_type_checks();

    let mut type_max_cost = 0;
    let mut type_base_cost = 0;
    let mut type_byte_cost = 0;
    if timed_features.is_enabled(TimedFeatureFlag::LimitTypeTagSize) {
        // 5000 limits type tag total size < 5000 bytes and < 50 nodes
        type_max_cost = 5000;
        type_base_cost = 100;
        type_byte_cost = 1;
    }

    let deserializer_config = aptos_prod_deserializer_config(features);
    let verifier_config = aptos_prod_verifier_config(features);

    VMConfig {
        verifier_config,
        deserializer_config,
        paranoid_type_checks,
        check_invariant_in_swap_loc,
        max_value_nest_depth: Some(128),
        type_max_cost,
        type_base_cost,
        type_byte_cost,
        delayed_field_optimization_enabled,
        ty_builder,
    }
}

/// A collection of on-chain randomness API configs that VM needs to be aware of.
pub struct RandomnessConfig {
    pub randomness_api_v0_required_deposit: Option<u64>,
    pub allow_rand_contract_custom_max_gas: bool,
}

impl RandomnessConfig {
    pub fn fetch(storage: &impl ConfigStorage) -> Self {
        let randomness_api_v0_required_deposit = RequiredGasDeposit::fetch_config(storage)
            .unwrap_or_else(RequiredGasDeposit::default_if_missing)
            .gas_amount;
        let allow_rand_contract_custom_max_gas = AllowCustomMaxGasFlag::fetch_config(storage)
            .unwrap_or_else(AllowCustomMaxGasFlag::default_if_missing)
            .value;
        Self {
            randomness_api_v0_required_deposit,
            allow_rand_contract_custom_max_gas,
        }
    }
}
