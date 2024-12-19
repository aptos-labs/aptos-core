// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module checks if non-special-address modules have native functions or structs,
//! which are disallowed.

use codespan_reporting::diagnostic::Severity;
use move_model::model::GlobalEnv;

/// Check whether a non-special address module has native functions or structs.
pub fn check_for_native_functions_and_structs(env: &mut GlobalEnv) {
    for module in env
        .get_modules()
        .filter(|m| m.is_primary_target() && !m.self_address().expect_numerical().is_special())
    {
        for fun in module.get_functions().filter(|f| f.is_native()) {
            env.diag(
                Severity::Error,
                &fun.get_loc(),
                "Only special-address modules can have native functions",
            );
        }
        for struct_ in module.get_structs().filter(|s| s.is_native()) {
            env.diag(
                Severity::Error,
                &struct_.get_loc(),
                "Only special-address modules can have native structs",
            );
        }
    }
}
