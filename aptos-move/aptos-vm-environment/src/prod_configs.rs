// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub use aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION;
use aptos_gas_schedule::{
    gas_feature_versions::{RELEASE_V1_15, RELEASE_V1_30, RELEASE_V1_34, RELEASE_V1_38},
    AptosGasParameters,
};
use aptos_types::{
    chain_id::ChainId,
    on_chain_config::{
        randomness_api_v0_config::{AllowCustomMaxGasFlag, RequiredGasDeposit},
        FeatureFlag, Features, OnChainConfig, TimedFeatureFlag, TimedFeatureOverride,
        TimedFeatures,
    },
    state_store::StateView,
};
use move_binary_format::deserializer::DeserializerConfig;
use move_bytecode_verifier::{verifier::VerificationScope, VerifierConfig};
use move_vm_runtime::config::VMConfig;
use move_vm_types::{
    loaded_data::runtime_types::TypeBuilder, values::DEFAULT_MAX_VM_VALUE_NESTED_DEPTH,
};
use once_cell::sync::OnceCell;
use std::sync::OnceLock;

static PARANOID_TYPE_CHECKS: OnceCell<bool> = OnceCell::new();
static PARANOID_REF_CHECKS: OnceCell<bool> = OnceCell::new();

/// Controls when additional checks (such as paranoid type checks) are performed. If set to true,
/// the trace may be collected during execution and Block-STM may perform the checks during post
/// commit processing once (instead of for every speculative execution). Note that there are other
/// factors that influence if checks are done async, such as block size, available workers, etc. If
/// not set - always performs the checks in-place at runtime.
static ASYNC_RUNTIME_CHECKS: OnceCell<bool> = OnceCell::new();
static TIMED_FEATURE_OVERRIDE: OnceCell<TimedFeatureOverride> = OnceCell::new();

/// Controls whether debugging is enabled. This is thread safe.
static DEBUGGING_ENABLED: OnceLock<bool> = OnceLock::new();

/// If enabled, types layouts are cached in a global long-living cache. Caches ensure the behavior
/// is the same as without caches, and so, using node config suffices.
static LAYOUT_CACHES: OnceCell<bool> = OnceCell::new();

/// Set the paranoid type check flag.
pub fn set_paranoid_type_checks(enable: bool) {
    PARANOID_TYPE_CHECKS.set(enable).ok();
}

/// Returns the paranoid type check flag if already set, and true otherwise.
pub fn get_paranoid_type_checks() -> bool {
    PARANOID_TYPE_CHECKS.get().cloned().unwrap_or(true)
}

/// Sets the async check flag.
pub fn set_async_runtime_checks(enable: bool) {
    ASYNC_RUNTIME_CHECKS.set(enable).ok();
}

/// Returns the async check flag if already set, and false otherwise.
pub fn get_async_runtime_checks() -> bool {
    ASYNC_RUNTIME_CHECKS.get().cloned().unwrap_or(false)
}

/// Set the paranoid reference check flag.
pub fn set_paranoid_ref_checks(enable: bool) {
    PARANOID_REF_CHECKS.set(enable).ok();
}

/// Returns the paranoid reference check flag if already set, and false otherwise.
pub fn get_paranoid_ref_checks() -> bool {
    PARANOID_REF_CHECKS.get().cloned().unwrap_or(false)
}

/// Set whether debugging is enabled. This can be called from multiple threads. If there
/// are multiple sets, all must have the same value. Notice that enabling debugging can
/// make execution slower.
pub fn set_debugging_enabled(enable: bool) {
    match DEBUGGING_ENABLED.set(enable) {
        Err(old) if old != enable => panic!(
            "tried to set \
        enable_debugging to {}, but was already set to {}",
            enable, old
        ),
        _ => {},
    }
}

/// Returns whether debugging is enabled. Only accessed privately to construct
/// VMConfig.
fn get_debugging_enabled() -> bool {
    DEBUGGING_ENABLED.get().cloned().unwrap_or(false)
}

/// Set the timed feature override.
pub fn set_timed_feature_override(profile: TimedFeatureOverride) {
    TIMED_FEATURE_OVERRIDE.set(profile).ok();
}

/// Returns the timed feature override, and [None] if not set.
pub fn get_timed_feature_override() -> Option<TimedFeatureOverride> {
    TIMED_FEATURE_OVERRIDE.get().cloned()
}

/// Set the layout cache flag.
pub fn set_layout_caches(enable: bool) {
    LAYOUT_CACHES.set(enable).ok();
}

/// Returns the layout cache flag if already set, and false otherwise.
pub fn get_layout_caches() -> bool {
    LAYOUT_CACHES.get().cloned().unwrap_or(false)
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
pub fn aptos_prod_verifier_config(gas_feature_version: u64, features: &Features) -> VerifierConfig {
    let sig_checker_v2_fix_script_ty_param_count =
        features.is_enabled(FeatureFlag::SIGNATURE_CHECKER_V2_SCRIPT_FIX);
    let sig_checker_v2_fix_function_signatures = gas_feature_version >= RELEASE_V1_34;
    let enable_enum_types = features.is_enabled(FeatureFlag::ENABLE_ENUM_TYPES);
    let enable_resource_access_control =
        features.is_enabled(FeatureFlag::ENABLE_RESOURCE_ACCESS_CONTROL);
    let enable_function_values = features.is_enabled(FeatureFlag::ENABLE_FUNCTION_VALUES);
    // Note: we reuse the `enable_function_values` flag to set various stricter limits on types.

    VerifierConfig {
        scope: VerificationScope::Everything,
        max_loop_depth: Some(5),
        max_generic_instantiation_length: Some(32),
        max_function_parameters: Some(128),
        max_basic_blocks: Some(1024),
        max_value_stack_size: 1024,
        max_type_nodes: if enable_function_values {
            Some(128)
        } else {
            Some(256)
        },
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
        _use_signature_checker_v2: true,
        sig_checker_v2_fix_script_ty_param_count,
        sig_checker_v2_fix_function_signatures,
        enable_enum_types,
        enable_resource_access_control,
        enable_function_values,
        max_function_return_values: if enable_function_values {
            Some(128)
        } else {
            None
        },
        max_type_depth: if enable_function_values {
            Some(20)
        } else {
            None
        },
    }
}

/// Returns [VMConfig] used by the Aptos blockchain in production, based on the set of feature
/// flags.
pub fn aptos_prod_vm_config(
    chain_id: ChainId,
    gas_feature_version: u64,
    features: &Features,
    timed_features: &TimedFeatures,
    ty_builder: TypeBuilder,
) -> VMConfig {
    let paranoid_type_checks = get_paranoid_type_checks();
    let paranoid_ref_checks = get_paranoid_ref_checks();
    let enable_layout_caches = get_layout_caches();
    let enable_debugging = get_debugging_enabled();

    let deserializer_config = aptos_prod_deserializer_config(features);
    let verifier_config = aptos_prod_verifier_config(gas_feature_version, features);
    let enable_enum_option = features.is_enabled(FeatureFlag::ENABLE_ENUM_OPTION);
    let enable_framework_for_option = features.is_enabled(FeatureFlag::ENABLE_FRAMEWORK_FOR_OPTION);

    let layout_max_size = if gas_feature_version >= RELEASE_V1_30 {
        512
    } else {
        256
    };

    // Value runtime depth checks have been introduced together with function values and are only
    // enabled when the function values are enabled. Previously, checks were performed over types
    // to bound the value depth (checking the size of a packed struct type bounds the value), but
    // this no longer applies once function values are enabled. With function values, types can be
    // shallow while the value can be deeply nested, thanks to captured arguments not visible in a
    // type. Hence, depth checks have been adjusted to operate on values.
    let enable_depth_checks = features.is_enabled(FeatureFlag::ENABLE_FUNCTION_VALUES);
    let enable_capture_option = !timed_features.is_enabled(TimedFeatureFlag::DisabledCaptureOption)
        || features.is_enabled(FeatureFlag::ENABLE_CAPTURE_OPTION);

    // Some feature gating was missed, so for native dynamic dispatch the feature is always on for
    // testnet after 1.38 release.
    let enable_function_caches = features.is_call_tree_and_instruction_vm_cache_enabled();
    let enable_function_caches_for_native_dynamic_dispatch =
        enable_function_caches || (chain_id.is_testnet() && gas_feature_version >= RELEASE_V1_38);

    let config = VMConfig {
        verifier_config,
        deserializer_config,
        paranoid_type_checks,
        legacy_check_invariant_in_swap_loc: false,
        // Note: if updating, make sure the constant is in-sync.
        max_value_nest_depth: Some(DEFAULT_MAX_VM_VALUE_NESTED_DEPTH),
        layout_max_size,
        layout_max_depth: 128,
        // 5000 limits type tag total size < 5000 bytes and < 50 nodes.
        type_max_cost: 5000,
        type_base_cost: 100,
        type_byte_cost: 1,
        // By default, do not use delayed field optimization. Instead, clients should enable it
        // manually where applicable.
        delayed_field_optimization_enabled: false,
        ty_builder,
        enable_function_caches,
        enable_lazy_loading: features.is_lazy_loading_enabled(),
        enable_depth_checks,
        optimize_trusted_code: features.is_trusted_code_enabled(),
        paranoid_ref_checks,
        enable_capture_option,
        enable_enum_option,
        enable_layout_caches,
        propagate_dependency_limit_error: gas_feature_version >= RELEASE_V1_38,
        enable_framework_for_option,
        enable_function_caches_for_native_dynamic_dispatch,
        enable_debugging,
    };

    // Note: if max_value_nest_depth changed, make sure the constant is in-sync. Do not remove this
    // assertion as it ensures the constant value is set correctly.
    assert_eq!(
        config.max_value_nest_depth,
        Some(DEFAULT_MAX_VM_VALUE_NESTED_DEPTH)
    );

    config
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
