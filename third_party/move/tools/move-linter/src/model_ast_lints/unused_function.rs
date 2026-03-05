// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Lint check for unused private functions.
//!
//! TODO(#18830): Add separate checkers for:
//! - friend functions in modules without friends
//! - functions only reachable from inaccessible callers
//! - a group of private functions that only call each other

use super::unused_common::SHARED_SUPPRESSION_ATTRS;
use move_binary_format::file_format::Visibility;
use move_compiler_v2::external_checks::FunctionChecker;
use move_model::{
    ast::Attribute,
    model::{FunctionEnv, GlobalEnv, Loc},
};

const CHECKER_NAME: &str = "unused_function";

/// Additional attribute names that suppress unused warnings for functions only.
/// - `persistent`: Marks a function as being persistent on upgrade (behaves like a public function).
/// - `view`: View functions are callable externally via the Aptos REST API.
const FUNC_ONLY_SUPPRESSION_ATTRS: &[&str] = &["persistent", "view"];

/// Functions excluded from unused checks, format: (address, module, function).
/// - `None` for address or module means "any" (wildcard).
/// - `init_module`: VM hook called automatically when module is published.
const EXCLUDED_FUNCTIONS: &[(Option<&str>, Option<&str>, &str)] = &[(None, None, "init_module")];

#[derive(Default)]
pub struct UnusedFunction;

impl FunctionChecker for UnusedFunction {
    fn get_name(&self) -> String {
        CHECKER_NAME.to_string()
    }

    fn check_function(&self, func: &FunctionEnv) {
        if should_warn_unused_function(func) {
            let msg = format!("function `{}` is unused", func.get_name_str());
            self.report(func.module_env.env, &func.get_id_loc(), &msg);
        }
    }

    fn report(&self, env: &GlobalEnv, loc: &Loc, msg: &str) {
        env.lint_diag_with_notes(loc, msg, vec![format!(
            "Remove it, or suppress this warning with `#[test_only]` (if for test-only) \
             or `#[lint::skip({})]` (if for spec-only or otherwise needed).",
            CHECKER_NAME
        )]);
    }
}

/// Returns true if function should be warned as unused.
fn should_warn_unused_function(func: &FunctionEnv) -> bool {
    let env = func.module_env.env;

    let is_suppression_attr = |attr: &Attribute| {
        SHARED_SUPPRESSION_ATTRS
            .iter()
            .chain(FUNC_ONLY_SUPPRESSION_ATTRS.iter())
            .any(|&s| attr.name() == env.symbol_pool().make(s))
    };

    if func.visibility() != Visibility::Private
        || func.is_script_or_entry()
        || func.is_test_or_verify_only()
        || is_excluded_function(func)
        || func.has_attribute(is_suppression_attr)
        || has_users(func)
    {
        return false;
    }

    true
}

/// Check if a function should be excluded from unused checks.
fn is_excluded_function(func: &FunctionEnv) -> bool {
    let env = func.module_env.env;
    let func_name = env.symbol_pool().string(func.get_name());

    EXCLUDED_FUNCTIONS.iter().any(|(ex_addr, ex_mod, ex_func)| {
        if func_name.as_ref() != *ex_func {
            return false;
        }
        if let Some(m) = ex_mod {
            let module_name = env.symbol_pool().string(func.module_env.get_name().name());
            if module_name.as_ref() != *m {
                return false;
            }
        }
        if let Some(a) = ex_addr {
            let addr = func.module_env.get_name().addr().expect_numerical();
            if addr.to_hex_literal() != *a {
                return false;
            }
        }
        true
    })
}

/// Check if function has any users (excluding self-recursive use).
fn has_users(func: &FunctionEnv) -> bool {
    if let Some(using_funs) = func.get_using_functions() {
        let func_qfid = func.get_qualified_id();
        using_funs.iter().any(|user| *user != func_qfid)
    } else {
        false
    }
}
