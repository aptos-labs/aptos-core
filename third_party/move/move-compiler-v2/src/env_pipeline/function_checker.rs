// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Do a few checks of functions and function calls.

use crate::{experiments::Experiment, Options};
use codespan_reporting::diagnostic::Severity;
use move_binary_format::file_format::Visibility;
use move_model::{
    ast::{ExpData, Operation, Pattern},
    metadata::LanguageVersion,
    model::{FunId, FunctionEnv, GlobalEnv, Loc, ModuleEnv, NodeId, Parameter, QualifiedId},
    ty::Type,
};
use std::{collections::BTreeSet, iter::Iterator, vec::Vec};

type QualifiedFunId = QualifiedId<FunId>;

// Takes a list of function types, returns those which have a function type in their argument type
fn identify_function_types_with_functions_in_args(func_types: Vec<Type>) -> Vec<Type> {
    func_types
        .into_iter()
        .filter_map(|ty| {
            if let Type::Fun(args, _, _) = &ty {
                if args.as_ref().has_function() {
                    Some(ty)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

// Takes a list of function-typed parameters, along with argument and result type
// Returns a list of any parameters whose result type has a function value, along with that result type.
fn identify_function_typed_params_with_functions_in_rets(
    func_types: Vec<&Parameter>,
) -> Vec<(&Parameter, &Type)> {
    func_types
        .iter()
        .filter_map(|param| {
            if let Type::Fun(_args, result, _) = &param.1 {
                let rest_unboxed = result.as_ref();
                if rest_unboxed.has_function() {
                    Some((*param, rest_unboxed))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

/// check that function parameters/results do not have function type unless allowed.
pub fn check_for_function_typed_parameters(env: &mut GlobalEnv) {
    let options = env
        .get_extension::<Options>()
        .expect("Options is available");

    let lambda_params_ok = options
        .language_version
        .unwrap_or_default()
        .is_at_least(LanguageVersion::V2_2);
    let lambda_return_ok = lambda_params_ok;
    if lambda_params_ok && lambda_return_ok {
        return;
    }

    for caller_module in env.get_modules() {
        if caller_module.is_target() {
            for caller_func in caller_module.get_functions() {
                if !lambda_params_ok || !lambda_return_ok {
                    let caller_name = caller_func.get_full_name_str();
                    let return_type = caller_func.get_result_type();
                    let func_returns: Vec<_> = return_type
                        .clone()
                        .flatten()
                        .into_iter()
                        .filter(|t| t.is_function())
                        .collect();
                    let type_display_ctx = caller_func.get_type_display_ctx();
                    if !func_returns.is_empty() {
                        // (2) is there a function type result at the top level?  This is allowed
                        // only for LAMBDA_IN_RETURNS
                        if !lambda_return_ok && !func_returns.is_empty() {
                            env.diag(
                                Severity::Error,
                                &caller_func.get_result_type_loc(),
                                &format!("Functions may not return function-typed values, but function `{}` return type is the function type `{}`:",
                                         &caller_name,
                                         return_type.display(&type_display_ctx)),
                            )
                        }
                        if !lambda_params_ok {
                            // (3) is there *any* function type with function type in an arg? This
                            // is allowed only for LAMBDA_IN_PARAMS
                            let bad_returns =
                                identify_function_types_with_functions_in_args(func_returns);
                            if !bad_returns.is_empty() {
                                env.diag(
                                    Severity::Error,
                                    &caller_func.get_result_type_loc(),
                                    &format!("Non-inline functions may not take function-typed parameters, but function `{}` return type is `{}`, which has a function type taking a function parameter:",
                                             &caller_name,
                                             return_type.display(&type_display_ctx)),
                                )
                            }
                        }
                    }

                    let parameters = caller_func.get_parameters_ref();
                    let func_params: Vec<_> = parameters
                        .iter()
                        .filter(|param| matches!(param.1, Type::Fun(..)))
                        .collect();
                    if !func_params.is_empty() {
                        // (1) is there a function type arg at the top level?  This is allowed for
                        // inline or LAMBDA_IN_PARAMS
                        if !caller_func.is_inline() && !lambda_params_ok {
                            let reasons: Vec<(Loc, String)> = func_params
                                .iter()
                                .map(|param| {
                                    (
                                        param.2.clone(),
                                        format!(
                                            "Parameter `{}` has function-valued type `{}`.",
                                            param.0.display(env.symbol_pool()),
                                            param.1.display(&type_display_ctx)
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
                        if !lambda_return_ok {
                            // (4) is there *any* function type with function type in its result? This is
                            // allowed only for LAMBDA_IN_RETURNS
                            let bad_params =
                                identify_function_typed_params_with_functions_in_rets(func_params);
                            if !bad_params.is_empty() {
                                let reasons: Vec<(Loc, String)> = bad_params
                                    .iter()
                                    .map(|(param, ty)| {
                                        (
                                            param.2.clone(),
                                            format!(
                                                "Parameter `{}` has type `{}`, which has function type `{}` as a function result type",
                                                param.0.display(env.symbol_pool()),
                                                param.1.display(&type_display_ctx),
                                                ty.display(&type_display_ctx),
                                            ),
                                        )
                                    })
                                    .collect();
                                env.diag_with_labels(
                                    Severity::Error,
                                    &caller_func.get_id_loc(),
                                    &format!("Functions may not return function-typed values, but function `{}` has {} of function type with function-typed result:",
                                             caller_name,
                                             if reasons.len() > 1 { "parameters" } else { "a parameter" },
                                    ),
                                    reasons,
                                );
                            }
                        }
                    }
                };
            }
        }
    }
}

fn access_error(
    env: &GlobalEnv,
    fun_env: &FunctionEnv,
    id: &NodeId,
    oper: &str,
    msg: String,
    module_env: &ModuleEnv,
) {
    let call_details: Vec<_> = [*id]
        .iter()
        .map(|node_id| (env.get_node_loc(*node_id), format!("{} here", oper)))
        .collect();
    let msg = format!(
        "Invalid operation: {} can only be done within the defining module `{}`",
        msg,
        module_env.get_full_name_str()
    );
    env.diag_with_labels(Severity::Error, &fun_env.get_id_loc(), &msg, call_details);
}

fn access_warning(
    env: &GlobalEnv,
    fun_env: &FunctionEnv,
    id: &NodeId,
    oper: &str,
    msg: String,
    module_env: &ModuleEnv,
) {
    let call_details: Vec<_> = [*id]
        .iter()
        .map(|node_id| (env.get_node_loc(*node_id), format!("{} here", oper)))
        .collect();
    let msg = format!(
        "{} can only be done within the defining module `{}`, but `{}` could be called (and expanded) outside the module",
        msg,
        module_env.get_full_name_str(),
        fun_env.get_full_name_str()
    );
    env.diag_with_labels(Severity::Warning, &fun_env.get_id_loc(), &msg, call_details);
}

fn check_for_access_error_or_warning<F>(
    env: &GlobalEnv,
    fun_env: &FunctionEnv,
    id: &NodeId,
    oper: &str,
    msg_maker: F,
    module_env: &ModuleEnv,
    cross_module: bool,
    caller_is_inline_non_private: bool,
) where
    F: Fn() -> String,
{
    if cross_module {
        access_error(env, fun_env, id, oper, msg_maker(), module_env);
    } else if caller_is_inline_non_private {
        access_warning(env, fun_env, id, oper, msg_maker(), module_env);
    }
}

/// Check for privileged operations on a struct/enum that can only be performed
/// within the module that defines it.
fn check_privileged_operations_on_structs(env: &GlobalEnv, fun_env: &FunctionEnv) {
    if let Some(fun_body) = fun_env.get_def() {
        let caller_module_id = fun_env.module_env.get_id();
        let caller_is_inline_non_private =
            fun_env.is_inline() && fun_env.visibility() != Visibility::Private;
        fun_body.visit_pre_order(&mut |exp: &ExpData| {
            match exp {
                ExpData::Call(id, oper, _) => match oper {
                    Operation::Exists(_)
                    | Operation::BorrowGlobal(_)
                    | Operation::MoveFrom
                    | Operation::MoveTo => {
                        let inst = env.get_node_instantiation(*id);
                        debug_assert!(!inst.is_empty());
                        if let Some((struct_env, _)) = inst[0].get_struct(env) {
                            let mid = struct_env.module_env.get_id();
                            let sid = struct_env.get_id();
                            let qualified_struct_id = mid.qualified(sid);
                            let struct_env = env.get_struct(qualified_struct_id);
                            let msg_maker = || {
                                format!(
                                    "storage operation on type `{}`",
                                    struct_env.get_full_name_str(),
                                )
                            };
                            check_for_access_error_or_warning(
                                env,
                                fun_env,
                                id,
                                "called",
                                msg_maker,
                                &struct_env.module_env,
                                mid != caller_module_id,
                                caller_is_inline_non_private,
                            );
                        }
                    },
                    Operation::Select(mid, sid, fid) => {
                        let qualified_struct_id = mid.qualified(*sid);
                        let struct_env = env.get_struct(qualified_struct_id);
                        let msg_maker = || {
                            format!(
                                "access of the field `{}` on type `{}`",
                                fid.symbol().display(struct_env.symbol_pool()),
                                struct_env.get_full_name_str(),
                            )
                        };
                        check_for_access_error_or_warning(
                            env,
                            fun_env,
                            id,
                            "accessed",
                            msg_maker,
                            &struct_env.module_env,
                            *mid != caller_module_id,
                            caller_is_inline_non_private,
                        );
                    },
                    Operation::SelectVariants(mid, sid, fids) => {
                        let qualified_struct_id = mid.qualified(*sid);
                        let struct_env = env.get_struct(qualified_struct_id);
                        // All field names are the same, so take one representative field id to report.
                        let field_env = struct_env.get_field(fids[0]);
                        let msg_maker = || {
                            format!(
                                "access of the field `{}` on enum type `{}`",
                                field_env.get_name().display(struct_env.symbol_pool()),
                                struct_env.get_full_name_str(),
                            )
                        };
                        check_for_access_error_or_warning(
                            env,
                            fun_env,
                            id,
                            "accessed",
                            msg_maker,
                            &struct_env.module_env,
                            *mid != caller_module_id,
                            caller_is_inline_non_private,
                        );
                    },
                    Operation::TestVariants(mid, sid, _) => {
                        let qualified_struct_id = mid.qualified(*sid);
                        let struct_env = env.get_struct(qualified_struct_id);
                        let msg_maker = || {
                            format!(
                                "variant test on enum type `{}`",
                                struct_env.get_full_name_str(),
                            )
                        };
                        check_for_access_error_or_warning(
                            env,
                            fun_env,
                            id,
                            "tested",
                            msg_maker,
                            &struct_env.module_env,
                            *mid != caller_module_id,
                            caller_is_inline_non_private,
                        );
                    },
                    Operation::Pack(mid, sid, _) => {
                        let qualified_struct_id = mid.qualified(*sid);
                        let struct_env = env.get_struct(qualified_struct_id);
                        let msg_maker = || format!("pack of `{}`", struct_env.get_full_name_str());
                        check_for_access_error_or_warning(
                            env,
                            fun_env,
                            id,
                            "packed",
                            msg_maker,
                            &struct_env.module_env,
                            *mid != caller_module_id,
                            caller_is_inline_non_private,
                        );
                    },
                    _ => {
                        // all the other operations are either:
                        // - not related to structs
                        // - spec-only
                    },
                },
                ExpData::Assign(_, pat, _)
                | ExpData::Block(_, pat, _, _)
                | ExpData::Lambda(_, pat, _, _, _) => {
                    pat.visit_pre_post(&mut |_, pat| {
                        if let Pattern::Struct(id, str, _, _) = pat {
                            let module_id = str.module_id;
                            let struct_env = env.get_struct(str.to_qualified_id());
                            let msg_maker =
                                || format!("unpack of `{}`", struct_env.get_full_name_str(),);
                            check_for_access_error_or_warning(
                                env,
                                fun_env,
                                id,
                                "unpacked",
                                msg_maker,
                                &struct_env.module_env,
                                module_id != caller_module_id,
                                caller_is_inline_non_private,
                            );
                        }
                    });
                },
                ExpData::Match(_, discriminator, _) => {
                    let discriminator_node_id = discriminator.node_id();
                    if let Type::Struct(mid, sid, _) =
                        env.get_node_type(discriminator_node_id).drop_reference()
                    {
                        let qualified_struct_id = mid.qualified(sid);
                        let struct_env = env.get_struct(qualified_struct_id);
                        let msg_maker =
                            || format!("match on enum type `{}`", struct_env.get_full_name_str(),);
                        check_for_access_error_or_warning(
                            env,
                            fun_env,
                            &discriminator_node_id,
                            "matched",
                            msg_maker,
                            &struct_env.module_env,
                            mid != caller_module_id,
                            caller_is_inline_non_private,
                        );
                    }
                },
                ExpData::Invalid(_)
                | ExpData::Value(..)
                | ExpData::LocalVar(..)
                | ExpData::Temporary(..)
                | ExpData::Invoke(..)
                | ExpData::Quant(..)
                | ExpData::IfElse(..)
                | ExpData::Return(..)
                | ExpData::Sequence(..)
                | ExpData::Loop(..)
                | ExpData::LoopCont(..)
                | ExpData::Mutate(..) => {},
                // access in specs is not restricted
                ExpData::SpecBlock(_, _) => {
                    return false;
                },
            }
            true
        });
    }
}

/// For all function in target modules:
///
/// If `before_inlining`, then
/// - check that all function calls involving inline functions are accessible;
/// - warn about unused private functions
/// Otherwise  (`!before_inlining`):
/// - check that all function calls *not* involving inline functions are accessible.
/// - check privileged operations on structs cannot be done across module boundary
pub fn check_access_and_use(env: &mut GlobalEnv, before_inlining: bool) {
    // For each function seen, we record whether it has an accessible caller.
    let mut functions_with_callers: BTreeSet<QualifiedFunId> = BTreeSet::new();
    // For each function seen, we record whether it has an inaccessible caller.
    let mut functions_with_inaccessible_callers: BTreeSet<QualifiedFunId> = BTreeSet::new();
    // Record all private and friendless public(friend) functions to check for uses.
    let mut private_funcs: BTreeSet<QualifiedFunId> = BTreeSet::new();

    for caller_module in env.get_modules() {
        // TODO(#13745): fix when we can tell in general if two modules are in the same package
        if caller_module.is_primary_target() {
            let caller_module_id = caller_module.get_id();
            let caller_module_has_friends = !caller_module.has_no_friends();
            let caller_module_is_script = caller_module.get_name().is_script();
            for caller_func in caller_module.get_functions() {
                if !before_inlining {
                    check_privileged_operations_on_structs(env, &caller_func);
                    check_inline_function_bodies_for_calls(env, &caller_func);
                }
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
                    let callees_with_sites = def.used_funs_with_uses();
                    for (callee, sites) in &callees_with_sites {
                        let callee_func = env.get_function(*callee);

                        // Script functions cannot be called.
                        if callee_func.module_env.is_script_module() {
                            calling_script_function_error(env, sites, &callee_func);
                        }

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
                                    } else if callee_func.has_package_visibility() {
                                        if callee_func.module_env.self_address()
                                            == caller_func.module_env.self_address()
                                        {
                                            // if callee is also a primary target, then they are in the same package
                                            // TODO(#13745): fix when we can tell in general if two modules are in the same package
                                            if callee_func.module_env.is_primary_target() {
                                                // we should've inferred the friend declaration
                                                panic!(
                                                    "{} should have friend {}",
                                                    callee_func.module_env.get_full_name_str(),
                                                    caller_func.module_env.get_full_name_str()
                                                );
                                            } else {
                                                // With "unsafe package visibility" experiment on, all package functions are made
                                                // visible in all modules with the same address. The prover uses this in filter mode
                                                // to get around the lack of package-based target filtering functionality.
                                                let options = env
                                                    .get_extension::<Options>()
                                                    .expect("Options is available");
                                                if options.experiment_on(
                                                    Experiment::UNSAFE_PACKAGE_VISIBILITY,
                                                ) {
                                                    true
                                                } else {
                                                    call_package_fun_from_diff_package_error(
                                                        env,
                                                        sites,
                                                        &caller_func,
                                                        &callee_func,
                                                    );
                                                    false
                                                }
                                            }
                                        } else {
                                            call_package_fun_from_diff_addr_error(
                                                env,
                                                sites,
                                                &caller_func,
                                                &callee_func,
                                            );
                                            false
                                        }
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

/// Check the body of inline functions (after inlining) to ensure they do not call
/// inaccessible functions.
fn check_inline_function_bodies_for_calls(env: &GlobalEnv, caller_func: &FunctionEnv) {
    if !caller_func.is_inline() {
        return;
    }
    let Some(def) = caller_func.get_def() else {
        return;
    };
    let caller_visibility = caller_func.visibility();
    if caller_visibility == Visibility::Private {
        // if caller is private inline function, it can only be called from the same module
        return;
    }
    let callees_with_sites = def.used_funs_with_uses();
    for (callee, sites) in &callees_with_sites {
        let callee_func = env.get_function(*callee);
        let callee_visibility = callee_func.visibility();
        let warn_info = match (caller_visibility, callee_visibility) {
            (_, Visibility::Public) => {
                // callee can be called from anywhere, so nothing to warn about
                None
            },
            (Visibility::Public, _) => Some("".to_string()),
            (Visibility::Friend, Visibility::Private) => {
                let caller_module = &caller_func.module_env;
                Some(format!(
                    ", such as in a {}friend module of `{}`",
                    if caller_module.has_no_friends() {
                        "(future) "
                    } else {
                        ""
                    },
                    caller_module.get_full_name_str()
                ))
            },
            (Visibility::Friend, Visibility::Friend) => {
                match (
                    caller_func.has_package_visibility(),
                    callee_func.has_package_visibility(),
                ) {
                    (_, true) => {
                        // TODO(#13745): fix when we can explicitly say if two modules are in the same package.
                        let same_package = callee_func.module_env.is_primary_target()
                            && callee_func.module_env.self_address()
                                == caller_func.module_env.self_address();
                        if !same_package {
                            Some("".to_string())
                        } else {
                            // caller and callee are in the same package, caller can only be called from
                            // friend contexts, so callee (package function) can also be called there.
                            None
                        }
                    },
                    (true, false) => Some(format!(
                        ", such as from a module in this package that is not a friend of `{}`",
                        callee_func.module_env.get_full_name_str()
                    )),
                    (false, false) => {
                        let caller_friends = caller_func.module_env.get_friend_modules();
                        let callee_friends = callee_func.module_env.get_friend_modules();
                        let covered = caller_friends.difference(&callee_friends).next().is_none();
                        if !covered {
                            Some(format!(
                                ", such as from a module that is a friend of `{}` but not a friend of `{}`",
                                caller_func.module_env.get_full_name_str(),
                                callee_func.module_env.get_full_name_str()
                            ))
                        } else {
                            // all of caller's friends are also callee's friends, so no warning
                            None
                        }
                    },
                }
            },
            (Visibility::Private, _) => {
                unreachable!("we return early")
            },
        };
        if let Some(additional_context) = warn_info {
            // We use just one call site as the warning label.
            let label: Vec<_> = sites
                    .first()
                    .map(|node_id| {
                        (
                            env.get_node_loc(*node_id),
                            format!(
                                "inline expansion calls {} function that may not be accessible in all locations that `{}` can be called",
                                function_visibility_description(&callee_func),
                                caller_func.get_full_name_str()
                            ),
                        )
                    })
                    .iter()
                    .cloned()
                    .collect::<Vec<(Loc, String)>>();
            env.diag_with_primary_and_labels(
                Severity::Warning,
                &caller_func.get_id_loc(),
                &format!(
                    "{} inline function `{}` cannot be called from all locations it is accessible",
                    function_visibility_description(caller_func),
                    caller_func.get_name_str()
                ),
                &format!(
                    "if called from a location where `{}` is not accessible{}",
                    callee_func.get_full_name_str(),
                    additional_context
                ),
                label,
            );
        }
    }
}

fn function_visibility_description(func: &FunctionEnv) -> String {
    match func.visibility() {
        Visibility::Public => "public".to_string(),
        Visibility::Friend => {
            if func.has_package_visibility() {
                "package".to_string()
            } else {
                "friend".to_string()
            }
        },
        Visibility::Private => "private".to_string(),
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
        if callee.is_inline() {
            "inline "
        } else if callee.has_package_visibility() {
            "public(package) "
        } else {
            ""
        },
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

fn call_package_fun_from_diff_package_error(
    env: &GlobalEnv,
    sites: &BTreeSet<NodeId>,
    caller: &FunctionEnv,
    callee: &FunctionEnv,
) {
    let why = "they are from different packages";
    cannot_call_error(env, why, sites, caller, callee);
}

fn call_package_fun_from_diff_addr_error(
    env: &GlobalEnv,
    sites: &BTreeSet<NodeId>,
    caller: &FunctionEnv,
    callee: &FunctionEnv,
) {
    let why = "they are from different addresses";
    cannot_call_error(env, why, sites, caller, callee);
}

fn calling_script_function_error(env: &GlobalEnv, sites: &BTreeSet<NodeId>, callee: &FunctionEnv) {
    let call_details: Vec<_> = sites
        .iter()
        .map(|node_id| (env.get_node_loc(*node_id), "used here".to_owned()))
        .collect();
    let callee_name = callee.get_name_str();
    let msg = format!(
        "script function `{}` cannot be used in Move code",
        callee_name
    );
    env.diag_with_labels(Severity::Error, &callee.get_id_loc(), &msg, call_details);
}
