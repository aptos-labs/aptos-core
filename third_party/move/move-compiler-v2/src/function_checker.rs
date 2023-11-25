// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Do a few checks of functions and function calls:
// - check that calls to or from inline functions are accessible;
//   - non-inline functions are handled in the VisibilityChecker pass, but
//     inline functions will be either gone or inlined by then.
// - check that non-inline function parameters do not have function type
// - check that all private functions in target modules have uses (if -Wunused flag is set)
// - check that there is exactly one function in scripts

use crate::Options;
use codespan_reporting::diagnostic::Severity;
use move_binary_format::file_format::Visibility;
use move_model::{
    model::{FunId, GlobalEnv, Loc, QualifiedId},
    ty::Type
};
use std::{
    collections::{BTreeMap, BTreeSet},
    iter::Iterator,
    ops::Deref,
    vec::Vec
};

type QualifiedFunId = QualifiedId<FunId>;

// Run checks for all functions in all target modules.
pub fn check_functions(env: &mut GlobalEnv) {
    let options = env
        .get_extension::<Options>()
        .expect("Options is available");
    //let debug = options.debug;
    let warn_about_unused = options.warn_unused;

    let mut callees_seen: BTreeMap<QualifiedFunId, BTreeSet<QualifiedFunId>> = BTreeMap::new();
    let mut private_funcs: BTreeSet<QualifiedFunId> = BTreeSet::new();
    for caller_module in env.get_modules() {
        if caller_module.is_target() {
            let caller_module_id = caller_module.get_id();
            let caller_module_name = caller_module.get_name();
            let caller_module_has_friends = !caller_module.get_friend_modules().is_empty();
            let caller_module_is_script = caller_module.get_name().is_script();
            for caller_func in caller_module.get_functions() {
                let caller_name_str = caller_func.get_full_name_with_address();
                let caller_qfid = caller_func.get_qualified_id();
                let caller_is_inline = caller_func.is_inline();

                // Check that non-inline function parameters don't have function type
                if !caller_is_inline {
                    let parameters = caller_func.get_parameters();
                    for param in parameters
                        .iter()
                        .filter(|param| matches!(param.1, Type::Fun(_, _)))
                    {
                        let type_ctx = caller_func.get_type_display_ctx();
                        let msg = format!(
                            "Non-inlined function `{}` parameter `{}` has a function type `{}`",
                            caller_module_name.display_full(env),
                            param.0.display(env.symbol_pool()),
                            param.1.display(&type_ctx)
                        );
                        env.error(&caller_func.get_loc(), &msg);
                    }
                }

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

                callees_seen.entry(caller_qfid).or_insert(BTreeSet::new());

                // Check that inline functions being called are accessible
                if let Some(def) = caller_func.get_def().deref() {
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
                                            "inline function `{}` cannot be called from a script, \
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
                                }
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
                                                    "called here".to_owned()
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
                            callees_seen
                                .entry(callee)
                                .and_modify(|curr | {
                                    curr.insert(caller_qfid);
                                })
                                .or_insert(BTreeSet::from([caller_qfid]));
                        }
                    }
                }
            }
        }
    }
    // Check for Unused functions: private (or friendless public(friend)) funs with no callers.
    for callee in private_funcs {
        if callees_seen
            .get(&callee)
            .map(|s| s.is_empty())
            .unwrap_or(true)
        {
            // We saw no uses of private/friendless function `callee`.
            let callee_env = env.get_function(callee);
            let callee_loc = callee_env.get_loc();
            let callee_is_script = callee_env.module_env.get_name().is_script();

            // Entry functions in a script don't need any uses.
            // Check others which are private.
            if !callee_is_script {
                let is_private = matches!(callee_env.visibility(), Visibility::Private);
                if warn_about_unused {
                    let msg = format!(
                        "Function `{}` is unused: it has no current uses and {} so it can have no future uses.",
                        callee_env.get_full_name_with_address(),
                        if is_private {
                            "is `private`"
                        } else {
                            "is `public(friend)` but module has no friends"
                        }
                    );
                    env.diag(Severity::Warning, &callee_loc, &msg);
                }
            }
        }
    }
}
