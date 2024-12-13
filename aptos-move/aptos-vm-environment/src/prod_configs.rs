// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_schedule::{gas_feature_versions::RELEASE_V1_15, AptosGasParameters};
use aptos_types::{
    on_chain_config::{
        randomness_api_v0_config::{AllowCustomMaxGasFlag, RequiredGasDeposit},
        FeatureFlag, Features, OnChainConfig, TimedFeatureFlag, TimedFeatureOverride,
        TimedFeatures,
    },
    state_store::StateView,
};
use move_binary_format::deserializer::DeserializerConfig;
use move_bytecode_verifier::VerifierConfig;
use move_vm_runtime::config::VMConfig;
use move_vm_types::loaded_data::runtime_types::TypeBuilder;
use once_cell::sync::OnceCell;

static PARANOID_TYPE_CHECKS: OnceCell<bool> = OnceCell::new();
static TIMED_FEATURE_OVERRIDE: OnceCell<TimedFeatureOverride> = OnceCell::new();

/// Set the paranoid type check flag.
pub fn set_paranoid_type_checks(enable: bool) {
    PARANOID_TYPE_CHECKS.set(enable).ok();
}

/// Returns the paranoid type check flag if already set, and true otherwise.
pub fn get_paranoid_type_checks() -> bool {
    PARANOID_TYPE_CHECKS.get().cloned().unwrap_or(true)
}

/// Set the timed feature override.
pub fn set_timed_feature_override(profile: TimedFeatureOverride) {
    TIMED_FEATURE_OVERRIDE.set(profile).ok();
}

/// Returns the timed feature override, and [None] if not set.
pub fn get_timed_feature_override() -> Option<TimedFeatureOverride> {
    TIMED_FEATURE_OVERRIDE.get().cloned()
}

/// Returns [TypeBuilder] used by the Aptos blockchain in production.
pub fn aptos_prod_ty_builder(
    gas_feature_version: u64,
    gas_params: &AptosGasParameters,
) -> TypeBuilder {
    if gas_feature_version >= RELEASE_V1_15 {
        let max_ty_size = gas_params.vm.txn.max_ty_size;
        let max_ty_depth = gas_params.vm.txn.max_ty_depth;
        TypeBuilder::with_limits(max_ty_size.into(), max_ty_depth.into())
    } else {
        aptos_default_ty_builder()
    }
}

/// Returns default [TypeBuilder], used only when:
///  1. Type size gas parameters are not yet in gas schedule (before 1.15).
///   2. No gas parameters are found on-chain.
pub fn aptos_default_ty_builder() -> TypeBuilder {
    TypeBuilder::with_limits(128, 20)
}

/// Returns [DeserializerConfig] used by the Aptos blockchain in production.
pub fn aptos_prod_deserializer_config(features: &Features) -> DeserializerConfig {
    DeserializerConfig::new(
        features.get_max_binary_format_version(),
        features.get_max_identifier_size(),
    )
}

/// Returns [VerifierConfig] used by the Aptos blockchain in production.
pub fn aptos_prod_verifier_config(features: &Features) -> VerifierConfig {
    let use_signature_checker_v2 = features.is_enabled(FeatureFlag::SIGNATURE_CHECKER_V2);
    let sig_checker_v2_fix_script_ty_param_count =
        features.is_enabled(FeatureFlag::SIGNATURE_CHECKER_V2_SCRIPT_FIX);
    let enable_enum_types = features.is_enabled(FeatureFlag::ENABLE_ENUM_TYPES);
    let enable_resource_access_control =
        features.is_enabled(FeatureFlag::ENABLE_RESOURCE_ACCESS_CONTROL);

    VerifierConfig {
        max_loop_depth: Some(5),
        max_generic_instantiation_length: Some(32),
        max_function_parameters: Some(128),
        max_basic_blocks: Some(1024),
        max_value_stack_size: 1024,
        max_type_nodes: Some(256),
        max_push_size: Some(10000),
        max_struct_definitions: None,
        max_struct_variants: None,
        max_fields_in_struct: None,
        max_function_definitions: None,
        max_back_edges_per_function: None,
        max_back_edges_per_module: None,
        max_basic_blocks_in_script: None,
        max_per_fun_meter_units: Some(1000 * 80000),
        max_per_mod_meter_units: Some(1000 * 80000),
        use_signature_checker_v2,
        sig_checker_v2_fix_script_ty_param_count,
        enable_enum_types,
        enable_resource_access_control,
    }
}

/// Returns [VMConfig] used by the Aptos blockchain in production, based on the set of feature
/// flags.
pub fn aptos_prod_vm_config(
    features: &Features,
    timed_features: &TimedFeatures,
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

    // Compatibility checker v2 is enabled either by its own flag or if enum types are enabled.
    let use_compatibility_checker_v2 = verifier_config.enable_enum_types
        || features.is_enabled(FeatureFlag::USE_COMPATIBILITY_CHECKER_V2);

    VMConfig {
        verifier_config,
        deserializer_config,
        paranoid_type_checks,
        check_invariant_in_swap_loc,
        max_value_nest_depth: Some(128),
        type_max_cost,
        type_base_cost,
        type_byte_cost,
        // By default, do not use delayed field optimization. Instead, clients should enable it
        // manually where applicable.
        delayed_field_optimization_enabled: false,
        ty_builder,
        disallow_dispatch_for_native: features.is_enabled(FeatureFlag::DISALLOW_USER_NATIVES),
        use_compatibility_checker_v2,
        use_loader_v2: features.is_loader_v2_enabled(),
    }
}

/// A collection of on-chain randomness API configs that VM needs to be aware of.
pub struct RandomnessConfig {
    pub randomness_api_v0_required_deposit: Option<u64>,
    pub allow_rand_contract_custom_max_gas: bool,
}

impl RandomnessConfig {
    /// Returns randomness config based on the current state.
    pub fn fetch(state_view: &impl StateView) -> Self {
        let randomness_api_v0_required_deposit = RequiredGasDeposit::fetch_config(state_view)
            .unwrap_or_else(RequiredGasDeposit::default_if_missing)
            .gas_amount;
        let allow_rand_contract_custom_max_gas = AllowCustomMaxGasFlag::fetch_config(state_view)
            .unwrap_or_else(AllowCustomMaxGasFlag::default_if_missing)
            .value;
        Self {
            randomness_api_v0_required_deposit,
            allow_rand_contract_custom_max_gas,
        }
    }
}
