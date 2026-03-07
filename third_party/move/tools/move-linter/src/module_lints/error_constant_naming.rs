// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Lint: error constants must follow `E_ERROR_NAME` or `EERROR_NAME` naming conventions.
//! A constant is considered an error constant if its name starts with `E` followed by
//! either `_` or an uppercase letter. This lint warns when neither convention is followed.

use move_compiler_v2::external_checks::ModuleChecker;
use move_model::model::{GlobalEnv, NamedConstantEnv};

pub struct ErrorConstantNaming;

impl ModuleChecker for ErrorConstantNaming {
    fn get_name(&self) -> String {
        "error_constant_naming".to_string()
    }

    fn visit_named_constant(&self, env: &GlobalEnv, constant: &NamedConstantEnv) {
        let name = env.symbol_pool().string(constant.get_name());
        let name = name.as_str();

        // Only check constants that start with 'E' (potential error constants).
        if !name.starts_with('E') || name.len() <= 1 {
            return;
        }

        let second_char = name.as_bytes()[1];

        // Accepted patterns:
        // - E_ followed by anything (e.g., E_NOT_FOUND)
        // - E followed by an uppercase letter (e.g., ENOT_FOUND)
        if second_char == b'_' || second_char.is_ascii_uppercase() {
            return;
        }

        // If we get here, it starts with E but doesn't follow either convention.
        self.report(
            env,
            &constant.get_loc(),
            "Error constant name does not follow the naming convention. Use `E_ERROR_NAME` or `EERROR_NAME`.",
        );
    }
}
