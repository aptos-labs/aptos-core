// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Do a few checks of functions and function calls.

use crate::Options;
use codespan_reporting::diagnostic::Severity;
use move_binary_format::file_format::Visibility;
use move_model::{
    model::{FunId, GlobalEnv, QualifiedId},
    ty::Type,
};
use std::{collections::BTreeSet, iter::Iterator, vec::Vec};

type QualifiedFunId = QualifiedId<FunId>;

/// check that non-inline function parameters do not have function type
pub fn check_for_function_typed_parameters(env: &mut GlobalEnv) {
    for caller_module in env.get_modules() {
        if caller_module.is_target() {
            for caller_func in caller_module.get_functions() {
                // Check that non-inline function parameters don't have function type
                if !caller_func.is_inline() {
                    let parameters = caller_func.get_parameters();
                    let bad_params: Vec<_> = parameters
                        .iter()
                        .filter(|param| matches!(param.1, Type::Fun(_, _)))
                        .collect();
                    if !bad_params.is_empty() {
                        let type_ctx = caller_func.get_type_display_ctx();
                        let caller_name = caller_func.get_full_name_str();
                        let notes: Vec<String> = bad_params
                            .iter()
                            .map(|param| {
                                format!(
                                    "Parameter `{}` has a function type `{}`.",
                                    param.0.display(env.symbol_pool()),
                                    param.1.display(&type_ctx)
                                )
                            })
                            .collect();
                        env.error_with_notes(
                            &caller_func.get_loc(),
                            &format!("Only inline functions may have function-typed parameters, but non-inline function `{}` has some:",
                                     caller_name),
                            notes,
                        );
                    }
                }
            }
        }
    }
}

/// For all function in target modules:
/// - check that non-inline function parameters do not have function type
/// - check that calls to or from inline functions are accessible;
///   - non-inline functions are handled in the VisibilityChecker pass, but
///     inline functions will be either gone or inlined by then.
pub fn check_access_and_use(env: &mut GlobalEnv) {
    // For each function seen, we record whether it has an accessible caller.
    let mut functions_with_callers: BTreeSet<QualifiedFunId> = BTreeSet::new();
    // For each function seen, we record whether it has an inaccessible caller.
    let mut functions_with_inaccessible_callers: BTreeSet<QualifiedFunId> = BTreeSet::new();
    // Record all private and friendless public(friend) functions to check for uses.
    let mut private_funcs: BTreeSet<QualifiedFunId> = BTreeSet::new();

    for caller_module in env.get_modules() {
        if caller_module.is_target() {
            let caller_module_id = caller_module.get_id();
            let caller_module_name = caller_module.get_name();
            let caller_module_has_friends = !caller_module.has_no_friends();
            let caller_module_is_script = caller_module.get_name().is_script();
            for caller_func in caller_module.get_functions() {
                let caller_name_str = caller_func.get_full_name_with_address();
                let caller_qfid = caller_func.get_qualified_id();
                let caller_is_inline = caller_func.is_inline();

                match caller_func.visibility() {
                    Visibility::Public => {},
                    Visibility::Friend => {
                        if !caller_module_has_friends {
                            // Function is essentially private
                            private_funcs.insert(caller_qfid);
                        }
                    },
                    Visibility::Private => {
                        private_funcs.insert(caller_qfid);
                    },
                };

                // Check that inline functions being called are accessible
                let optional_def = caller_func.get_def();
                if let Some(def) = optional_def {
                    let callees_with_sites = def.called_funs_with_callsites();
                    for (callee, sites) in callees_with_sites {
                        let callee_env = env.get_function(callee);
                        // check visibility if not in the same module
                        let callee_is_inline = callee_env.is_inline();
                        let callee_is_accessible = if callee_env.module_env.get_id()
                            == caller_module_id
                        {
                            true
                        } else {
                            match callee_env.visibility() {
                                Visibility::Public => true,
                                _ if caller_module_is_script => {
                                    // Only public functions are visible from scripts.
                                    if callee_is_inline {
                                        let call_details: Vec<_> = sites
                                            .iter()
                                            .map(|node_id| {
                                                (
                                                    env.get_node_loc(*node_id),
                                                    "called here".to_owned(),
                                                )
                                            })
                                            .collect();
                                        let msg = format!(
                                            "Inline function `{}` cannot be called from a script, \
                                             because it is not public",
                                            callee_env.get_full_name_with_address(),
                                        );
                                        env.diag_with_labels(
                                            Severity::Error,
                                            &callee_env.get_loc(),
                                            &msg,
                                            call_details,
                                        );
                                    }
                                    false
                                },
                                Visibility::Friend => {
                                    if callee_env.module_env.has_friend(&caller_module_id) {
                                        true
                                    } else {
                                        if caller_is_inline || callee_is_inline {
                                            let call_details: Vec<_> = sites
                                                .iter()
                                                .map(|node_id| {
                                                    (
                                                        env.get_node_loc(*node_id),
                                                        "called here".to_owned(),
                                                    )
                                                })
                                                .collect();
                                            let msg = format!(
                                                "`public(friend)` {}function `{}` cannot be called from {}function `{}` because module `{}` is not a `friend` of `{}`",
                                                if callee_is_inline { "inline " } else { "" },
                                                callee_env.get_full_name_with_address(),
                                                caller_name_str,
                                                if caller_is_inline { "inline " } else { "" },
                                                caller_module_name.display_full(env),
                                                callee_env.module_env.get_full_name_str());
                                            env.diag_with_labels(
                                                Severity::Error,
                                                &callee_env.get_loc(),
                                                &msg,
                                                call_details,
                                            );
                                        }
                                        false
                                    }
                                },
                                Visibility::Private => {
                                    if caller_is_inline || callee_is_inline {
                                        let call_details: Vec<_> = sites
                                            .iter()
                                            .map(|node_id| {
                                                (
                                                    env.get_node_loc(*node_id),
                                                    "called here".to_owned(),
                                                )
                                            })
                                            .collect();
                                        let msg = format!(
                                            "{}function `{}` cannot be called from {}function `{}` \
                                             because it is private to module `{}`",
                                            if callee_is_inline { "inline " } else { "" },
                                            callee_env.get_full_name_with_address(),
                                            if caller_is_inline { "inline " } else { "" },
                                            caller_name_str,
                                            callee_env.module_env.get_full_name_str());
                                        env.diag_with_labels(
                                            Severity::Error,
                                            &callee_env.get_loc(),
                                            &msg,
                                            call_details,
                                        );
                                    }
                                    false
                                },
                            }
                        };
                        if callee_is_accessible {
                            functions_with_callers.insert(callee);
                        } else {
                            functions_with_inaccessible_callers.insert(callee);
                        }
                    }
                }
            }
        }
    }

    // Check for Unused functions: private (or friendless public(friend)) funs with no callers.
    let options = env
        .get_extension::<Options>()
        .expect("Options is available");
    if options.warn_unused {
        for callee in private_funcs {
            if !functions_with_callers.contains(&callee) {
                // We saw no uses of private/friendless function `callee`.
                let callee_env = env.get_function(callee);
                let callee_loc = callee_env.get_loc();
                let callee_is_script = callee_env.module_env.get_name().is_script();

                // Entry functions in a script don't need any uses.
                // Check others which are private.
                if !callee_is_script {
                    let is_private = matches!(callee_env.visibility(), Visibility::Private);
                    if functions_with_inaccessible_callers.contains(&callee) {
                        let msg = format!(
                            "Function `{}` may be unused: it has callers, but none with access.",
                            callee_env.get_full_name_with_address(),
                        );
                        env.diag(Severity::Warning, &callee_loc, &msg);
                    } else {
                        let msg = format!(
                            "Function `{}` is unused: it has no current callers and {}.",
                            callee_env.get_full_name_with_address(),
                            if is_private {
                                "is private to its module"
                            } else {
                                "is `public(friend)` but its module has no friends"
                            }
                        );
                        env.diag(Severity::Warning, &callee_loc, &msg);
                    }
                }
            }
        }
    }
}
