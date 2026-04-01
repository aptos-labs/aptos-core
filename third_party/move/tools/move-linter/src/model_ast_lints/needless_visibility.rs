// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lint check for functions with unnecessarily wide visibility.
//!
//! Detects:
//! - Package functions called only from the same module (package visibility has
//!   no effect)
//! - Friend functions called only from the same module (friend visibility has
//!   no effect)
//! - Friend functions in modules that declare no friends (friend visibility has
//!   no effect)

use super::unused_common::{has_same_module_users_only, should_skip_function};
use move_binary_format::file_format::Visibility;
use move_compiler_v2::external_checks::FunctionChecker;
use move_model::model::FunctionEnv;

const CHECKER_NAME: &str = "needless_visibility";

#[derive(Default)]
pub struct NeedlessVisibility;

impl FunctionChecker for NeedlessVisibility {
    fn get_name(&self) -> String {
        CHECKER_NAME.to_string()
    }

    fn check_function(&self, func: &FunctionEnv) {
        if let Some((msg, note)) = check_needless(func) {
            func.module_env
                .env
                .lint_diag_with_notes(&func.get_id_loc(), &msg, vec![note]);
        }
    }
}

/// Returns (message, note) if the function has needlessly wide visibility.
fn check_needless(func: &FunctionEnv) -> Option<(String, String)> {
    if should_skip_function(func) {
        return None;
    }

    let name = func.get_name_str();

    match func.visibility() {
        Visibility::Public | Visibility::Private => None,
        Visibility::Friend => {
            if func.has_package_visibility() && has_same_module_users_only(func) {
                Some(same_module_warning("package", &name))
            } else if !func.has_package_visibility() && func.module_env.has_no_friends() {
                Some(no_friends_warning(&name))
            } else if !func.has_package_visibility() && has_same_module_users_only(func) {
                Some(same_module_warning("friend", &name))
            } else {
                None
            }
        },
    }
}

fn same_module_warning(visibility: &str, name: &str) -> (String, String) {
    (
        format!(
            "{} function `{}` is only called from the same module: \
             {} visibility is not needed",
            visibility, name, visibility
        ),
        format!(
            "Consider removing the visibility modifier, \
             or suppress with `#[lint::skip({})]`.",
            CHECKER_NAME
        ),
    )
}

fn no_friends_warning(name: &str) -> (String, String) {
    (
        format!(
            "friend function `{}` has needless visibility: \
             module declares no friends",
            name
        ),
        format!(
            "This module declares no friends, so friend visibility is not needed. \
             Remove this visibility, or add friend declarations. \
             Suppress with `#[lint::skip({})]` if appropriate.",
            CHECKER_NAME
        ),
    )
}
