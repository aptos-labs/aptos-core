// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Lint check for unused functions (private, package, and friend).
//!
//! Detects:
//! - Private functions with no callers
//! - Package functions with no callers
//! - Friend functions with no callers
//!
//! TODO(#18830): Add separate checkers for:
//! - functions only reachable from inaccessible callers
//! - a group of private/package/friend functions that only call each other

use super::unused_common::{has_users, should_skip_function};
use move_binary_format::file_format::Visibility;
use move_compiler_v2::external_checks::FunctionChecker;
use move_model::model::FunctionEnv;

const CHECKER_NAME: &str = "unused_function";

#[derive(Default)]
pub struct UnusedFunction;

impl FunctionChecker for UnusedFunction {
    fn get_name(&self) -> String {
        CHECKER_NAME.to_string()
    }

    fn check_function(&self, func: &FunctionEnv) {
        if let Some(msg) = check_unused(func) {
            let note = format!(
                "Remove it, or suppress this warning with `#[test_only]` (if for test-only) \
                 or `#[lint::skip({})]` (if for spec-only or otherwise needed).",
                CHECKER_NAME
            );
            func.module_env
                .env
                .lint_diag_with_notes(&func.get_id_loc(), &msg, vec![note]);
        }
    }
}

/// Returns a warning message if the function is unused, or None.
fn check_unused(func: &FunctionEnv) -> Option<String> {
    if should_skip_function(func) {
        return None;
    }

    let visibility_prefix = match func.visibility() {
        Visibility::Public => return None,
        Visibility::Private => "",
        Visibility::Friend if func.has_package_visibility() => "package ",
        Visibility::Friend => "friend ",
    };

    if has_users(func) {
        return None;
    }

    Some(format!(
        "{}function `{}` is unused",
        visibility_prefix,
        func.get_name_str()
    ))
}
