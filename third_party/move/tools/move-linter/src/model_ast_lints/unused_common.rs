// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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

/// Returns true if the function has at least one non-self caller, and all
/// callers are within the same module.
///
/// Direct callers that are inline functions are replaced by their (transitive)
/// callers, because inline bodies are expanded at call sites during compilation:
/// after inlining, this function is effectively called from wherever the inline
/// caller is called.
pub fn has_same_module_users_only(func: &FunctionEnv) -> bool {
    let using_funs = func.get_using_functions_with_transitive_inline();
    let func_qfid = func.get_qualified_id();
    let func_module_id = func.module_env.get_id();

    let mut has_non_self_user = false;
    for user in using_funs.iter() {
        if *user == func_qfid {
            continue;
        }
        has_non_self_user = true;
        if user.module_id != func_module_id {
            return false;
        }
    }
    has_non_self_user
}
