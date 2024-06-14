// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Configuration for the MoveSmith fuzzer.

/// The configuration for the MoveSmith fuzzer.
/// MoveSmith will randomly pick within [0..max_num_XXX] during generation.
#[derive(Debug)]
pub struct Config {
    /// The number of `//# run 0xCAFE::ModuleX::funX` to invoke
    pub num_runs_per_func: usize,

    pub max_num_modules: usize,
    pub max_num_functions_in_module: usize,
    pub max_num_structs_in_module: usize,
    pub max_num_uses_in_module: usize,
    pub max_num_friends_in_module: usize,
    pub max_num_constants_in_module: usize,
    pub max_num_specs_in_module: usize,

    pub max_num_fields_in_struct: usize,

    pub max_num_stmts_in_func: usize,
    pub max_num_params_in_func: usize,

    // This has lowest priority
    // i.e. if the block is a function body
    // max_num_stmts_in_func will override this
    pub max_num_stmts_in_block: usize,

    pub max_num_calls_in_script: usize,

    // Maximum depth of nested expression
    pub max_expr_depth: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            num_runs_per_func: 10,

            max_num_modules: 3,
            max_num_functions_in_module: 5,
            max_num_structs_in_module: 5,
            max_num_uses_in_module: 5,
            max_num_friends_in_module: 5,
            max_num_constants_in_module: 5,
            max_num_specs_in_module: 5,

            max_num_fields_in_struct: 5,

            max_num_stmts_in_func: 10,
            max_num_params_in_func: 3,

            max_num_stmts_in_block: 10,

            max_num_calls_in_script: 20,

            max_expr_depth: 5,
        }
    }
}
