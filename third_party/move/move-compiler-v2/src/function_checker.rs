// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Do a few checks of functions and function calls.

use crate::Options;
use codespan_reporting::diagnostic::Severity;
use move_binary_format::file_format::Visibility;
use move_model::{
    model::{FunId, FunctionEnv, GlobalEnv, Loc, NodeId, QualifiedId},
    ty::Type,
};
use std::{collections::BTreeSet, iter::Iterator, vec::Vec};

type QualifiedFunId = QualifiedId<FunId>;

/// check that non-inline function parameters do not have function type.
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
                        let caller_name = caller_func.get_full_name_str();
                        let reasons: Vec<(Loc, String)> = bad_params
                            .iter()
                            .map(|param| {
                                (
                                    param.2.clone(),
                                    format!(
                                        "Parameter `{}` has a function type.",
                                        param.0.display(env.symbol_pool()),
                                    ),
                                )
                            })
                            .collect();
                        env.diag_with_labels(
                            Severity::Error,
                            &caller_func.get_id_loc(),
                            &format!("Only inline functions may have function-typed parameters, but non-inline function `{}` has {}:",
                                     caller_name,
                                     if reasons.len() > 1 { "function parameters" } else { "a function parameter" },
                            ),
                            reasons,
                        );
                    }
                }
            }
        }
    }
}

/// For all function in target modules:
///
/// If `before_inlining`, then
/// - check that all function calls involving inline functions are accessible;
/// - warn about unused private functions
/// Otherwise  (`!before_inlining`):
/// - check that all function calls *not* involving inline functions are accessible.
pub fn check_access_and_use(env: &mut GlobalEnv, before_inlining: bool) {
    // For each function seen, we record whether it has an accessible caller.
    let mut functions_with_callers: BTreeSet<QualifiedFunId> = BTreeSet::new();
    // For each function seen, we record whether it has an inaccessible caller.
    let mut functions_with_inaccessible_callers: BTreeSet<QualifiedFunId> = BTreeSet::new();
    // Record all private and friendless public(friend) functions to check for uses.
    let mut private_funcs: BTreeSet<QualifiedFunId> = BTreeSet::new();

    for caller_module in env.get_modules() {
        if caller_module.is_target() {
            let caller_module_id = caller_module.get_id();
            let caller_module_has_friends = !caller_module.has_no_friends();
            let caller_module_is_script = caller_module.get_name().is_script();
            for caller_func in caller_module.get_functions() {
                let caller_qfid = caller_func.get_qualified_id();

                // During first pass, record private functions for later
                if before_inlining {
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
                }

                // Check that functions being called are accessible.
                if let Some(def) = caller_func.get_def() {
                    let callees_with_sites = def.called_funs_with_callsites();
                    for (callee, sites) in &callees_with_sites {
                        let callee_func = env.get_function(*callee);
                        // Check visibility.

                        // Same module is always visible
                        let same_module = callee_func.module_env.get_id() == caller_module_id;
                        let call_involves_inline_function =
                            callee_func.is_inline() || caller_func.is_inline();

                        // SKIP check if same_module or
                        // if before inlining and the call doesn't involve inline function.
                        let skip_check =
                            same_module || (before_inlining && !call_involves_inline_function);

                        let callee_is_accessible = if skip_check {
                            true
                        } else {
                            match callee_func.visibility() {
                                Visibility::Public => true,
                                _ if caller_module_is_script => {
                                    // Only public functions are visible from scripts.
                                    generic_error(
                                        env,
                                        "a script ",
                                        "it is not public",
                                        sites,
                                        &callee_func,
                                    );
                                    false
                                },
                                Visibility::Friend => {
                                    if callee_func.module_env.has_friend(&caller_module_id) {
                                        true
                                    } else {
                                        not_a_friend_error(env, sites, &caller_func, &callee_func);
                                        false
                                    }
                                },
                                Visibility::Private => {
                                    private_to_module_error(env, sites, &caller_func, &callee_func);
                                    false
                                },
                            }
                        };
                        // Only record and warn about unused functions before inlining:
                        if before_inlining {
                            // Record called functions for Unused check below.
                            if callee_is_accessible {
                                functions_with_callers.insert(*callee);
                            } else {
                                functions_with_inaccessible_callers.insert(*callee);
                            }
                        }
                    }
                }
            }
        }
    }

    if before_inlining {
        // Check for Unused functions: private (or friendless public(friend)) funs with no callers.
        let options = env
            .get_extension::<Options>()
            .expect("Options is available");
        if options.warn_unused {
            for callee in private_funcs {
                if !functions_with_callers.contains(&callee) {
                    // We saw no uses of private/friendless function `callee`.
                    let callee_func = env.get_function(callee);
                    let callee_loc = callee_func.get_id_loc();
                    let callee_is_script = callee_func.module_env.get_name().is_script();

                    // Entry functions in a script don't need any uses.
                    // Check others which are private.
                    if !callee_is_script {
                        let is_private = matches!(callee_func.visibility(), Visibility::Private);
                        if functions_with_inaccessible_callers.contains(&callee) {
                            let msg = format!(
                                "Function `{}` may be unused: it has callers, but none with access.",
                                callee_func.get_full_name_with_address(),
                            );
                            env.diag(Severity::Warning, &callee_loc, &msg);
                        } else {
                            let msg = format!(
                                "Function `{}` is unused: it has no current callers and {}.",
                                callee_func.get_full_name_with_address(),
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
}

fn generic_error(
    env: &GlobalEnv,
    called_from: &str,
    why: &str,
    sites: &BTreeSet<NodeId>,
    callee: &FunctionEnv,
) {
    let call_details: Vec<_> = sites
        .iter()
        .map(|node_id| (env.get_node_loc(*node_id), "called here".to_owned()))
        .collect();
    let callee_name = callee.get_full_name_with_address();
    let msg = format!(
        "{}function `{}` cannot be called from {}\
         because {}",
        if callee.is_inline() { "inline " } else { "" },
        callee_name,
        called_from,
        why,
    );
    env.diag_with_primary_and_labels(
        Severity::Error,
        &callee.get_id_loc(),
        &msg,
        "callee",
        call_details,
    );
}

fn cannot_call_error(
    env: &GlobalEnv,
    why: &str,
    sites: &BTreeSet<NodeId>,
    caller: &FunctionEnv,
    callee: &FunctionEnv,
) {
    let called_from = format!(
        "{}function `{}` ",
        if caller.is_inline() { "inline " } else { "" },
        caller.get_full_name_with_address()
    );
    generic_error(env, &called_from, why, sites, callee);
}

fn private_to_module_error(
    env: &GlobalEnv,
    sites: &BTreeSet<NodeId>,
    caller: &FunctionEnv,
    callee: &FunctionEnv,
) {
    let why = format!(
        "it is private to module `{}`",
        callee.module_env.get_full_name_str()
    );
    cannot_call_error(env, &why, sites, caller, callee);
}

fn not_a_friend_error(
    env: &GlobalEnv,
    sites: &BTreeSet<NodeId>,
    caller: &FunctionEnv,
    callee: &FunctionEnv,
) {
    let why = format!(
        "module `{}` is not a `friend` of `{}`",
        caller.module_env.get_full_name_str(),
        callee.module_env.get_full_name_str()
    );
    cannot_call_error(env, &why, sites, caller, callee);
}
