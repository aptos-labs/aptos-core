// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::{FeatureFlag, Features, TimedFeatureFlag, TimedFeatures};
use move_binary_format::deserializer::DeserializerConfig;
use move_bytecode_verifier::VerifierConfig;
use move_vm_runtime::config::VMConfig;

pub fn aptos_prod_deserializer_config(
    features: &Features,
    gas_feature_version: u64,
) -> DeserializerConfig {
    // Note: binary format v6 adds a few new integer types and their corresponding instructions.
    //       Therefore, it depends on a new version of the gas schedule and cannot be allowed if
    //       the gas schedule hasn't been updated yet.
    let max_binary_format_version =
        features.get_max_binary_format_version(Some(gas_feature_version));

    DeserializerConfig::new(
        max_binary_format_version,
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
    gas_feature_version: u64,
    aggregator_v2_type_tagging: bool,
    paranoid_type_checks: bool,
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

    let deserializer_config = aptos_prod_deserializer_config(features, gas_feature_version);
    let verifier = aptos_prod_verifier_config(features);

    VMConfig {
        verifier,
        deserializer_config,
        paranoid_type_checks,
        enable_invariant_violation_check_in_swap_loc,
        type_size_limit: true,
        max_value_nest_depth: Some(128),
        type_max_cost,
        type_base_cost,
        type_byte_cost,
        aggregator_v2_type_tagging,
    }
}
