// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Common code for unused item and needless visibility checks.

use move_model::model::FunctionEnv;

/// Attribute names that suppress unused warnings for all item types.
/// - `deprecated`: Marks items that are deprecated but may not be removed.
pub const SHARED_SUPPRESSION_ATTRS: &[&str] = &["deprecated"];

/// Function names excluded from unused/needless-visibility checks.
/// - `init_module`: VM hook called automatically when module is published.
const EXCLUDED_FUNCTION_NAMES: &[&str] = &["init_module"];

/// Attribute names that suppress unused warnings specifically for functions.
/// - `persistent`: Marks storage-related functions invoked by the runtime.
/// - `view`: Marks read-only query functions callable off-chain via the API.
const FUNCTION_SUPPRESSION_ATTRS: &[&str] = &["persistent", "view"];

/// Returns true if the function should be skipped by unused/needless-visibility
/// checkers (entry, test-only, excluded, or has suppression attributes).
pub fn should_skip_function(func: &FunctionEnv) -> bool {
    let env = func.module_env.env;
    func.is_script_or_entry()
        || func.is_test_or_verify_only()
        || EXCLUDED_FUNCTION_NAMES
            .iter()
            .any(|name| func.matches_name(name))
        || func.has_attribute(|attr| {
            SHARED_SUPPRESSION_ATTRS
                .iter()
                .chain(FUNCTION_SUPPRESSION_ATTRS.iter())
                .any(|&s| attr.name() == env.symbol_pool().make(s))
        })
}

/// Check if function has any users (excluding self-recursive use).
pub fn has_users(func: &FunctionEnv) -> bool {
    let Some(using_funs) = func.get_using_functions() else {
        return false;
    };
    let func_qfid = func.get_qualified_id();
    using_funs.iter().any(|user| *user != func_qfid)
}

/// Check if function has any users from a different module.
pub fn has_cross_module_users(func: &FunctionEnv) -> bool {
    let Some(using_funs) = func.get_using_functions() else {
        return false;
    };
    let func_module_id = func.module_env.get_id();

    using_funs
        .iter()
        .any(|user| user.module_id != func_module_id)
}
