// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::file_format_common::VERSION_MAX;
use move_bytecode_verifier::VerifierConfig;

pub const DEFAULT_MAX_VALUE_NEST_DEPTH: u64 = 128;

/// Dynamic config options for the Move VM.
pub struct VMConfig {
    pub verifier: VerifierConfig,
    pub max_binary_format_version: u32,
    // When this flag is set to true, MoveVM will perform type check at every instruction
    // execution to ensure that type safety cannot be violated at runtime.
    pub paranoid_type_checks: bool,
    // When this flag is set to true, MoveVM will check invariant violation in swap_loc
    pub enable_invariant_violation_check_in_swap_loc: bool,
    pub type_size_limit: bool,
    /// Maximum value nest depth for structs
    pub max_value_nest_depth: Option<u64>,
    pub type_max_cost: u64,
    pub type_base_cost: u64,
    pub type_byte_cost: u64,
}

impl Default for VMConfig {
    fn default() -> Self {
        Self {
            verifier: VerifierConfig::default(),
            max_binary_format_version: VERSION_MAX,
            paranoid_type_checks: false,
            enable_invariant_violation_check_in_swap_loc: true,
            type_size_limit: false,
            max_value_nest_depth: Some(DEFAULT_MAX_VALUE_NEST_DEPTH),
            type_max_cost: 0,
            type_base_cost: 0,
            type_byte_cost: 0,
        }
    }
}

impl VMConfig {
    pub fn production() -> Self {
        Self {
            verifier: VerifierConfig::production(),
            ..Self::default()
        }
    }
}
