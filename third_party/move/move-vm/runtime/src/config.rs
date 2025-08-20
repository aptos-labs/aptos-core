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
    /// execution to ensure that type safety cannot be violated at runtime.
    pub paranoid_type_checks: bool,
    pub check_invariant_in_swap_loc: bool,
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
    pub use_call_tree_and_instruction_cache: bool,
    pub enable_lazy_loading: bool,
    pub enable_depth_checks: bool,
}

impl Default for VMConfig {
    fn default() -> Self {
        Self {
            verifier_config: VerifierConfig::default(),
            deserializer_config: DeserializerConfig::default(),
            paranoid_type_checks: false,
            check_invariant_in_swap_loc: true,
            max_value_nest_depth: Some(DEFAULT_MAX_VM_VALUE_NESTED_DEPTH),
            layout_max_size: 512,
            layout_max_depth: 128,
            type_max_cost: 0,
            type_base_cost: 0,
            type_byte_cost: 0,
            delayed_field_optimization_enabled: false,
            ty_builder: TypeBuilder::with_limits(128, 20),
            use_call_tree_and_instruction_cache: true,
            enable_lazy_loading: true,
            enable_depth_checks: true,
        }
    }
}
