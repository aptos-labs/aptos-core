// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::deserializer::DeserializerConfig;
use move_bytecode_verifier::VerifierConfig;
use move_vm_types::loaded_data::runtime_types::TypeBuilder;
use serde::Serialize;

pub const DEFAULT_MAX_VALUE_NEST_DEPTH: u64 = 128;

/// Dynamic config options for the Move VM.
#[derive(Clone, Serialize)]
pub struct VMConfig {
    pub verifier_config: VerifierConfig,
    pub deserializer_config: DeserializerConfig,
    /// When this flag is set to true, MoveVM will perform type checks at every instruction
    /// execution to ensure that type safety cannot be violated at runtime.
    pub paranoid_type_checks: bool,
    pub check_invariant_in_swap_loc: bool,
    /// Maximum value nest depth for structs.
    pub max_value_nest_depth: Option<u64>,
    pub type_max_cost: u64,
    pub type_base_cost: u64,
    pub type_byte_cost: u64,
    pub delayed_field_optimization_enabled: bool,
    pub ty_builder: TypeBuilder,
    pub disallow_dispatch_for_native: bool,
    pub use_compatibility_checker_v2: bool,
    pub use_loader_v2: bool,
}

impl Default for VMConfig {
    fn default() -> Self {
        Self {
            verifier_config: VerifierConfig::default(),
            deserializer_config: DeserializerConfig::default(),
            paranoid_type_checks: false,
            check_invariant_in_swap_loc: true,
            max_value_nest_depth: Some(DEFAULT_MAX_VALUE_NEST_DEPTH),
            type_max_cost: 0,
            type_base_cost: 0,
            type_byte_cost: 0,
            delayed_field_optimization_enabled: false,
            ty_builder: TypeBuilder::with_limits(128, 20),
            disallow_dispatch_for_native: true,
            use_compatibility_checker_v2: true,
            use_loader_v2: true,
        }
    }
}
