// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Configuration for the MoveSmith fuzzer.

use move_compiler_v2::Experiment;

/// The configuration for the MoveSmith fuzzer.
/// MoveSmith will randomly pick within [0..max_num_XXX] during generation.
pub struct Config {
    // The list of known errors to ignore
    // This is aggresive: if the diff contains any of these strings,
    // the report will be ignored
    pub known_error: Vec<String>,

    pub experiment_combos: Vec<(String, Vec<(String, bool)>)>,

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
    // Maximum depth of nested type instantiation
    pub max_type_depth: usize,

    pub max_num_type_params_in_func: usize,
    pub max_num_type_params_in_struct: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            known_error: vec![
                "exceeded maximal local count".to_string(),
                "MOVELOC_UNAVAILABLE_ERROR".to_string(),
                // unassigned variable should be removed
                // after copy is properly inserted 
                "unassigned variable".to_string(),
            ],

            experiment_combos: vec![
                ("optimize".to_string(), vec![
                    (Experiment::OPTIMIZE.to_string(), true),
                    (Experiment::ACQUIRES_CHECK.to_string(), false),
                ]),
                ("no-optimize".to_string(), vec![
                    (Experiment::OPTIMIZE.to_string(), false),
                    (Experiment::ACQUIRES_CHECK.to_string(), false),
                ]),
                // ("optimize-no-simplify".to_string(), vec![
                //     (Experiment::OPTIMIZE.to_string(), true),
                //     (Experiment::AST_SIMPLIFY.to_string(), false),
                //     (Experiment::ACQUIRES_CHECK.to_string(), false),
                // ]),
            ],

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
            max_type_depth: 5,
            max_num_type_params_in_func: 3,
            max_num_type_params_in_struct: 3,
        }
    }
}
