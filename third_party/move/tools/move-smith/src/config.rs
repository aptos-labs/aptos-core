// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Config {
    pub max_num_modules: usize,
    pub max_num_functions_in_module: usize,
    pub max_num_structs_in_module: usize,
    pub max_num_uses_in_module: usize,
    pub max_num_friends_in_module: usize,
    pub max_num_constants_in_module: usize,
    pub max_num_specs_in_module: usize,

    pub max_num_fields_in_struct: usize,

    pub max_num_stmt_in_func: usize,
    pub max_num_params_in_func: usize,

    pub max_num_calls_in_script: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_num_modules: 3,
            max_num_functions_in_module: 10,
            max_num_structs_in_module: 10,
            max_num_uses_in_module: 10,
            max_num_friends_in_module: 10,
            max_num_constants_in_module: 10,
            max_num_specs_in_module: 10,

            max_num_fields_in_struct: 5,

            max_num_stmt_in_func: 20,
            max_num_params_in_func: 5,

            max_num_calls_in_script: 20,
        }
    }
}
