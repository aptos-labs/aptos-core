// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Config {
    pub max_num_modules: usize,
    pub max_num_members_in_module: usize,
    pub max_num_stmt_in_func: usize,
    pub max_num_params_in_func: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_num_modules: 3,
            max_num_members_in_module: 5,
            max_num_stmt_in_func: 10,
            max_num_params_in_func: 4,
        }
    }
}
