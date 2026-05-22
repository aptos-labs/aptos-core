// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lint check for improper qualifiers on test functions.
//!
//! Rules:
//! 1. A `#[test]` function must not have any visibility qualifier and must
//!    not be `entry`.
//! 2. A `#[test_only]` function (or any function inside a `#[test_only]`
//!    module) must not be `entry`.

use move_binary_format::file_format::Visibility;
use move_compiler_v2::external_checks::FunctionChecker;
use move_model::model::FunctionEnv;

const CHECKER_NAME: &str = "improper_test_function_qualifiers";

#[derive(Default)]
pub struct ImproperTestFunctionQualifiers;

impl FunctionChecker for ImproperTestFunctionQualifiers {
    fn get_name(&self) -> String {
        CHECKER_NAME.to_string()
    }

    fn runs_on_test_code(&self) -> bool {
        true
    }

    fn check_function(&self, func: &FunctionEnv) {
        // `is_test_only()` covers `#[test]`, `#[test_only]`, and functions in
        // a `#[test_only]` module.
        if !func.is_test_only() {
            return;
        }
        let has_visibility = func.visibility() != Visibility::Private;
        let has_entry = func.is_entry();
        if !has_visibility && !has_entry {
            return;
        }
        let test_attr = func.module_env.env.symbol_pool().make("test");
        let is_test = func.get_attributes().iter().any(|a| a.name() == test_attr);

        let msg: &str = if is_test {
            match (has_visibility, has_entry) {
                (true, true) => {
                    "`#[test]` functions should not have visibility qualifiers and should not be `entry`"
                },
                (true, false) => "`#[test]` functions should not have visibility qualifiers",
                (false, true) => "`#[test]` functions should not be `entry`",
                (false, false) => unreachable!("early-returned above when neither qualifier is set"),
            }
        } else if has_entry {
            "`#[test_only]` functions should not be `entry`"
        } else {
            // `#[test_only]` allows visibility qualifiers.
            return;
        };

        self.report(func.module_env.env, &func.get_id_loc(), msg);
    }
}
