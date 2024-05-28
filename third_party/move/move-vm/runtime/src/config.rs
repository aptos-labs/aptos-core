// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;

pub const DEFAULT_MAX_VALUE_NEST_DEPTH: u64 = 128;

/// Dynamic config options for the Move VM.
#[derive(Clone, Serialize)]
pub struct VMConfig {
    /// When this flag is set to true, MoveVM will perform type checks at every instruction
    /// execution to ensure that type safety cannot be violated at runtime.
    pub paranoid_type_checks: bool,
    pub check_invariant_in_swap_loc: bool,
    pub type_size_limit: bool,
    /// Maximum value nest depth for structs.
    pub max_value_nest_depth: Option<u64>,
    pub type_max_cost: u64,
    pub type_base_cost: u64,
    pub type_byte_cost: u64,
    pub delayed_field_optimization_enabled: bool,
}

impl Default for VMConfig {
    fn default() -> Self {
        Self {
            paranoid_type_checks: false,
            check_invariant_in_swap_loc: true,
            type_size_limit: false,
            max_value_nest_depth: Some(DEFAULT_MAX_VALUE_NEST_DEPTH),
            type_max_cost: 0,
            type_base_cost: 0,
            type_byte_cost: 0,
            delayed_field_optimization_enabled: false,
        }
    }
}
