// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::deserializer::DeserializerConfig;
use move_bytecode_verifier::VerifierConfig;
use move_vm_types::{
    loaded_data::runtime_types::TypeBuilder, values::DEFAULT_MAX_VM_VALUE_NESTED_DEPTH,
};
use serde::Serialize;

/// Dynamic config options for the Move VM. Always add new fields to the end, as we rely on the
/// hash or serialized bytes of config to detect if it has changed (e.g., new feature flag was
/// enabled). Also, do not delete existing fields, or change the type of existing field.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct VMConfig {
    pub verifier_config: VerifierConfig,
    pub deserializer_config: DeserializerConfig,
    /// When this flag is set to true, MoveVM will perform type checks at every instruction
    /// execution to ensure that type safety cannot be violated at runtime. Note: these
    /// are more than type checks, for example, stack balancing, visibility, but the name
    /// is kept for historical reasons.
    pub paranoid_type_checks: bool,
    /// Always set to false, no longer used, kept for compatibility.
    pub legacy_check_invariant_in_swap_loc: bool,
    /// Maximum value nest depth for structs.
    pub max_value_nest_depth: Option<u64>,
    /// Maximum allowed number of nodes in a type layout. This includes the types of fields for
    /// struct types.
    pub layout_max_size: u64,
    /// Maximum depth (in number of nodes) of the type layout tree.
    pub layout_max_depth: u64,
    pub type_max_cost: u64,
    pub type_base_cost: u64,
    pub type_byte_cost: u64,
    pub delayed_field_optimization_enabled: bool,
    pub ty_builder: TypeBuilder,
    pub enable_function_caches: bool,
    pub enable_lazy_loading: bool,
    pub enable_depth_checks: bool,
    /// Whether trusted code should be optimized, for example, excluding it from expensive
    /// paranoid checks. Checks may still not be done in place, and instead delayed to later time.
    /// Instead, a trace can be recorded which is sufficient for type checking.
    pub optimize_trusted_code: bool,
    /// When this flag is set to true, Move VM will perform additional checks to ensure that
    /// reference safety is maintained during execution. Note that the checks might be delayed and
    /// instead execution trace can be recorded (so that checks are done based on the trace later).
    pub paranoid_ref_checks: bool,
    pub enable_capture_option: bool,
    pub enable_enum_option: bool,
    /// If true, Move VM will try to fetch layout from remote cache.
    pub enable_layout_caches: bool,
    pub propagate_dependency_limit_error: bool,
    pub enable_framework_for_option: bool,
    /// Same as enable_function_caches, but gates missed gating for native dynamic dispatch.
    pub enable_function_caches_for_native_dynamic_dispatch: bool,
    /// Whether this VM should support debugging. If set, environment variables
    /// `MOVE_VM_TRACE` and `MOVE_VM_STEP` will be recognized.
    pub enable_debugging: bool,
}

impl Default for VMConfig {
    fn default() -> Self {
        Self {
            verifier_config: VerifierConfig::default(),
            deserializer_config: DeserializerConfig::default(),
            paranoid_type_checks: false,
            legacy_check_invariant_in_swap_loc: false,
            max_value_nest_depth: Some(DEFAULT_MAX_VM_VALUE_NESTED_DEPTH),
            layout_max_size: 512,
            layout_max_depth: 128,
            type_max_cost: 0,
            type_base_cost: 0,
            type_byte_cost: 0,
            delayed_field_optimization_enabled: false,
            ty_builder: TypeBuilder::with_limits(128, 20),
            enable_function_caches: true,
            enable_lazy_loading: true,
            enable_depth_checks: true,
            optimize_trusted_code: false,
            paranoid_ref_checks: false,
            enable_capture_option: true,
            enable_enum_option: true,
            enable_layout_caches: true,
            propagate_dependency_limit_error: true,
            enable_framework_for_option: true,
            enable_function_caches_for_native_dynamic_dispatch: true,
            enable_debugging: false,
        }
    }
}

impl VMConfig {
    /// Set the paranoid reference checks flag.
    pub fn set_paranoid_ref_checks(self, enable: bool) -> Self {
        Self {
            paranoid_ref_checks: enable,
            ..self
        }
    }

    /// Default for use in tests.
    pub fn default_for_test() -> Self {
        Self {
            enable_debugging: true,
            ..Self::default()
        }
    }
}
