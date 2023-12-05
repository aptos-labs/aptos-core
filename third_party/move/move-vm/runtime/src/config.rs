// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{
    deserializer::DeserializerConfig,
    file_format_common::{IDENTIFIER_SIZE_MAX, VERSION_MAX},
};
use move_bytecode_verifier::VerifierConfig;
use serde::Serialize;

pub const DEFAULT_MAX_VALUE_NEST_DEPTH: u64 = 128;

/// Dynamic config options for the Move VM.
#[derive(Clone, Serialize)]
pub struct VMConfig {
    pub verifier: VerifierConfig,
    pub deserializer_config: DeserializerConfig,
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
    pub aggregator_v2_type_tagging: bool,
}

impl Default for VMConfig {
    fn default() -> Self {
        Self {
            verifier: VerifierConfig::default(),
            deserializer_config: DeserializerConfig::new(VERSION_MAX, IDENTIFIER_SIZE_MAX),
            paranoid_type_checks: false,
            enable_invariant_violation_check_in_swap_loc: true,
            type_size_limit: false,
            max_value_nest_depth: Some(DEFAULT_MAX_VALUE_NEST_DEPTH),
            type_max_cost: 0,
            type_base_cost: 0,
            type_byte_cost: 0,
            aggregator_v2_type_tagging: true,
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
