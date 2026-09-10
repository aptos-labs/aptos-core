// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module checks for native declarations which the VM rejects at publish or
//! load time, mirroring its rules: native structs and native entry functions are
//! never allowed, and other native functions are allowed only in modules at
//! special addresses.

use codespan_reporting::diagnostic::Severity;
use move_model::model::GlobalEnv;

/// Check for disallowed native declarations in primary target modules.
pub fn check_for_native_functions_and_structs(env: &mut GlobalEnv) {
    for module in env.get_modules().filter(|m| m.is_primary_target()) {
        let is_special = module.self_address().expect_numerical().is_special();
        for fun in module.get_functions().filter(|f| f.is_native()) {
            if fun.is_entry() {
                env.diag(
                    Severity::Error,
                    &fun.get_loc(),
                    "native functions cannot be entry functions",
                );
            } else if !is_special {
                env.diag(
                    Severity::Error,
                    &fun.get_loc(),
                    "only special-address modules can have native functions",
                );
            }
        }
        for struct_ in module.get_structs().filter(|s| s.is_native()) {
            env.diag(
                Severity::Error,
                &struct_.get_loc(),
                "native structs are not supported",
            );
        }
    }
}
