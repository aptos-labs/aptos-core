// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::file_format_common::VERSION_MAX;
use move_bytecode_verifier::VerifierConfig;

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
}

impl Default for VMConfig {
    fn default() -> Self {
        Self {
            verifier: VerifierConfig::default(),
            max_binary_format_version: VERSION_MAX,
            paranoid_type_checks: false,
            enable_invariant_violation_check_in_swap_loc: true,
            type_size_limit: false,
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
