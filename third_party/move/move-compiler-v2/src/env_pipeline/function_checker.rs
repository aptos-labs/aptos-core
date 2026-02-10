// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Do a few checks of functions and function calls.

use crate::{experiments::Experiment, Options};
use codespan_reporting::diagnostic::Severity;
use move_binary_format::file_format::Visibility;
use move_model::{
    ast::{ExpData, Operation, Pattern},
    metadata::{lang_feature_versions::LANGUAGE_VERSION_FOR_UNUSED_CHECK, LanguageVersion},
    model::{
        FunId, FunctionEnv, GlobalEnv, Loc, ModuleEnv, ModuleId, NodeId, Parameter, QualifiedId,
        StructEnv,
    },
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
    extra_msg: Option<String>,
    module_env: &ModuleEnv,
) {
    let call_details: Vec<_> = [*id]
        .iter()
        .map(|node_id| (env.get_node_loc(*node_id), format!("{} here", oper)))
        .collect();
    let msg = format!(
        "Invalid operation: {} can only be done within the defining module `{}` {}",
        msg,
        module_env.get_full_name_str(),
        extra_msg.unwrap_or_default()
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

/// Check for access error or warning for a struct operation.
/// storage operations include `exists`, `move_to`, `move_from`, `borrow_global`, `borrow_global_mut`
fn check_for_access_error_or_warning<F>(
    env: &GlobalEnv,
    fun_env: &FunctionEnv,
    struct_env: &StructEnv,
    caller_module_id: &ModuleId,
    storage_operation: bool,
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
        let mut err_msg = None;
        // storage operations cannot be cross-module, even for public structs
        if !storage_operation && env.language_version().language_version_for_public_struct() {
            match struct_env.get_visibility() {
                Visibility::Public => {
                    return;
                },
                Visibility::Friend => {
                    if struct_env.module_env.has_friend(caller_module_id) {
                        return;
                    }
                    let friend_str = if struct_env.has_package_visibility() {
                        "modules in the same package".to_string()
                    } else {
                        "friend modules".to_string()
                    };
                    err_msg = Some(format!("or {}", friend_str));
                },
                Visibility::Private => {},
            }
        }
        access_error(env, fun_env, id, oper, msg_maker(), err_msg, module_env);
    } else if caller_is_inline_non_private {
        if !storage_operation
            && env.language_version().language_version_for_public_struct()
            && struct_env.get_visibility() == Visibility::Public
        {
            return;
        }
        access_warning(env, fun_env, id, oper, msg_maker(), module_env);
    }
}

/// Check for privileged operations on a struct/enum that can only be performed
/// within the module that defines it.
///
/// This function walks the AST and checks:
/// - Storage operations (exists, borrow_global, move_from, move_to)
/// - Field access operations (select, select_variants)
/// - Struct construction (pack) and destruction (unpack)
/// - Enum operations (test_variants, match)
fn check_privileged_operations_on_structs(env: &GlobalEnv, fun_env: &FunctionEnv) {
    let Some(fun_body) = fun_env.get_def() else {
        return;
    };

    let caller_module_id = fun_env.module_env.get_id();
    let caller_is_inline_non_private =
        fun_env.is_inline() && fun_env.visibility() != Visibility::Private;

    // Track nesting depth of spec blocks - we skip checks inside spec blocks
    let mut spec_blocks_seen = 0;

    fun_body.visit_pre_post(&mut |post, exp: &ExpData| {
        if post {
            // Post-visit: decrement spec block counter
            if matches!(exp, ExpData::SpecBlock(..)) {
                debug_assert!(spec_blocks_seen > 0, "should match in pre and post");
                spec_blocks_seen -= 1;
            }
            return true;
        }

        // Pre-visit: track spec blocks and skip checks inside them
        if matches!(exp, ExpData::SpecBlock(..)) {
            spec_blocks_seen += 1;
        }
        if spec_blocks_seen > 0 {
            return true; // Skip checks inside spec blocks
        }

        // Check different expression types for privileged operations
        match exp {
            ExpData::Call(id, oper, _) => {
                check_operation(
                    env,
                    fun_env,
                    oper,
                    id,
                    &caller_module_id,
                    caller_is_inline_non_private,
                );
            },
            ExpData::Assign(_, pat, _)
            | ExpData::Block(_, pat, _, _)
            | ExpData::Lambda(_, pat, _, _, _) => {
                check_pattern_unpacks(
                    env,
                    fun_env,
                    pat,
                    &caller_module_id,
                    caller_is_inline_non_private,
                );
            },
            ExpData::Match(_, discriminator, _) => {
                check_match_discriminator(
                    env,
                    fun_env,
                    discriminator,
                    &caller_module_id,
                    caller_is_inline_non_private,
                );
            },
            ExpData::SpecBlock(_, _) => {
                unreachable!("should have been handled above");
            },
            _ => {
                // Other expression types don't involve privileged struct operations
            },
        }
        true
    });
}

/// Check operations on structs (storage ops, field access, pack, etc.)
fn check_operation(
    env: &GlobalEnv,
    fun_env: &FunctionEnv,
    oper: &Operation,
    id: &NodeId,
    caller_module_id: &ModuleId,
    caller_is_inline_non_private: bool,
) {
    match oper {
        // Storage operations: only allowed within the struct's module
        Operation::Exists(_)
        | Operation::BorrowGlobal(_)
        | Operation::MoveFrom
        | Operation::MoveTo => {
            let inst = env.get_node_instantiation(*id);
            debug_assert!(!inst.is_empty());
            if let Some((struct_env, _)) = inst[0].get_struct(env) {
                let mid = struct_env.module_env.get_id();
                let sid = struct_env.get_id();
                check_struct_operation(
                    env,
                    fun_env,
                    mid,
                    sid,
                    caller_module_id,
                    true, // storage operation
                    id,
                    "called",
                    |s| format!("storage operation on type `{}`", s.get_full_name_str()),
                    caller_is_inline_non_private,
                );
            }
        },

        Operation::Select(mid, sid, fid) => {
            check_struct_operation(
                env,
                fun_env,
                *mid,
                *sid,
                caller_module_id,
                false,
                id,
                "accessed",
                |s| {
                    format!(
                        "access of the field `{}` on type `{}`",
                        fid.symbol().display(s.symbol_pool()),
                        s.get_full_name_str(),
                    )
                },
                caller_is_inline_non_private,
            );
        },

        Operation::SelectVariants(mid, sid, fids) => {
            check_struct_operation(
                env,
                fun_env,
                *mid,
                *sid,
                caller_module_id,
                false,
                id,
                "accessed",
                |s| {
                    let field = s.get_field(fids[0]);
                    format!(
                        "access of the field `{}` on enum type `{}`",
                        field.get_name().display(s.symbol_pool()),
                        s.get_full_name_str(),
                    )
                },
                caller_is_inline_non_private,
            );
        },

        Operation::TestVariants(mid, sid, _) => {
            check_struct_operation(
                env,
                fun_env,
                *mid,
                *sid,
                caller_module_id,
                false,
                id,
                "tested",
                |s| format!("variant test on enum type `{}`", s.get_full_name_str()),
                caller_is_inline_non_private,
            );
        },

        Operation::Pack(mid, sid, _) => {
            check_struct_operation(
                env,
                fun_env,
                *mid,
                *sid,
                caller_module_id,
                false,
                id,
                "packed",
                |s| format!("pack of `{}`", s.get_full_name_str()),
                caller_is_inline_non_private,
            );
        },

        _ => {
            // Other operations don't involve privileged struct access
        },
    }
}

/// Common helper for checking struct operations
fn check_struct_operation<F>(
    env: &GlobalEnv,
    fun_env: &FunctionEnv,
    mid: ModuleId,
    sid: move_model::model::StructId,
    caller_module_id: &ModuleId,
    storage_operation: bool,
    id: &NodeId,
    oper: &str,
    msg_maker: F,
    caller_is_inline_non_private: bool,
) where
    F: Fn(&StructEnv) -> String,
{
    let struct_env = env.get_struct(mid.qualified(sid));
    let cross_module = mid != *caller_module_id;
    check_for_access_error_or_warning(
        env,
        fun_env,
        &struct_env,
        caller_module_id,
        storage_operation,
        id,
        oper,
        || msg_maker(&struct_env),
        &struct_env.module_env,
        cross_module,
        caller_is_inline_non_private,
    );
}

/// Check patterns for struct unpacks
fn check_pattern_unpacks(
    env: &GlobalEnv,
    fun_env: &FunctionEnv,
    pat: &Pattern,
    caller_module_id: &ModuleId,
    caller_is_inline_non_private: bool,
) {
    pat.visit_pre_post(&mut |_, pat| {
        if let Pattern::Struct(id, str, _, _) = pat {
            check_struct_operation(
                env,
                fun_env,
                str.module_id,
                str.id,
                caller_module_id,
                false,
                id,
                "unpacked",
                |s| format!("unpack of `{}`", s.get_full_name_str()),
                caller_is_inline_non_private,
            );
        }
    });
}

/// Check match expression discriminators
fn check_match_discriminator(
    env: &GlobalEnv,
    fun_env: &FunctionEnv,
    discriminator: &Exp,
    caller_module_id: &ModuleId,
    caller_is_inline_non_private: bool,
) {
    let discriminator_node_id = discriminator.node_id();
    if let Type::Struct(mid, sid, _) = env.get_node_type(discriminator_node_id).drop_reference() {
        check_struct_operation(
            env,
            fun_env,
            mid,
            sid,
            caller_module_id,
            false,
            &discriminator_node_id,
            "matched",
            |s| format!("match on enum type `{}`", s.get_full_name_str()),
            caller_is_inline_non_private,
        );
    }
}

/// Check function accessibility and report unused entities. Called twice during compilation.
///
/// **Before inlining:**
/// - Check visibility of calls to/from inline functions:
///   - Public: callable from any module
///   - Friend: callable only from friend modules
///   - Private: callable only within the same module
/// - Report unused private functions
/// - Report unused private structs
/// - Report unused constants
///
/// **After inlining:**
/// - Check visibility of non-inline function calls (same rules as above)
/// - Check visibility of struct operations (pack, unpack, field access):
///   - Public structs: operations allowed from any module
///   - Friend structs: operations allowed only from friend modules
///   - Private structs: operations allowed only within the defining module
/// - Check storage operations (exists, borrow_global, move_from, move_to):
///   - Always restricted to the struct's defining module (even for public structs)
/// - Check visibility of enum operations (match, variant test): same rules as struct operations
/// - Warn when inline functions call private helpers that won't be accessible after inlining
pub fn check_access_and_use(env: &mut GlobalEnv, before_inlining: bool) {
    if before_inlining {
        check_before_inlining(env);
    } else {
        check_after_inlining(env);
    }
}

/// Check accessibility and usage before inlining.
/// - Checks that inline function calls are accessible (before they disappear)
/// - Collects and reports unused entity warnings (functions, structs, constants)
fn check_before_inlining(env: &mut GlobalEnv) {
    let unused_warnings_enabled = env
        .get_extension::<Options>()
        .expect("Options is available");
    let language_version = options.language_version.unwrap_or_default();
    let unused_warnings_enabled = before_inlining
        && options.experiment_on(Experiment::UNUSED_CHECK)
        && language_version >= LANGUAGE_VERSION_FOR_UNUSED_CHECK;

    // Track function usage for unused function detection
    let mut functions_with_callers = BTreeSet::new();
    let mut functions_with_inaccessible_callers = BTreeSet::new();
    let mut private_funcs = BTreeSet::new();

    // Collect module IDs to avoid borrow checker issues
    let module_ids: Vec<_> = env
        .get_modules()
        .filter(|m| m.is_primary_target())
        .map(|m| m.get_id())
        .collect();

    // Process each module
    for module_id in module_ids {
        let module = env.get_module(module_id);

        // Check inline function call accessibility and track function usage
        for caller_func in module.get_functions() {
            // Track private and friendless public(friend) functions for unused detection
            match caller_func.visibility() {
                Visibility::Public => {},
                Visibility::Friend if !module.has_no_friends() => {},
                _ => {
                    private_funcs.insert(caller_func.get_qualified_id());
                },
            }

            // Check cross-module inline calls and track all function calls
            check_and_track_function_calls(
                env,
                &caller_func,
                module_id,
                &mut functions_with_callers,
                &mut functions_with_inaccessible_callers,
            );
        }

        // Check for unused entities if warnings are enabled
        if unused_warnings_enabled {
            check_unused_structs(env, module_id);
            check_unused_constants(env, module_id);
        }
    }

    // Report unused functions
    if unused_warnings_enabled {
        report_unused_functions(
            env,
            &private_funcs,
            &functions_with_callers,
            &functions_with_inaccessible_callers,
        );
    }
}

/// Check cross-module inline function calls for accessibility and track all function calls.
///
/// For each function call:
/// - If cross-module with inline functions: check visibility now (before inlining)
/// - Otherwise: just track as called (for unused detection or check after inlining)
fn check_and_track_function_calls(
    env: &GlobalEnv,
    caller_func: &FunctionEnv,
    caller_module_id: ModuleId,
    functions_with_callers: &mut BTreeSet<QualifiedFunId>,
    functions_with_inaccessible_callers: &mut BTreeSet<QualifiedFunId>,
) {
    let Some(def) = caller_func.get_def() else {
        return;
    };

    let caller_is_inline = caller_func.is_inline();
    let caller_is_script = caller_func.module_env.get_name().is_script();

    for (callee_id, sites) in &def.used_funs_with_uses() {
        let callee_func = env.get_function(*callee_id);

        // Script functions cannot be called - always error
        if callee_func.module_env.is_script_module() {
            calling_script_function_error(env, sites, &callee_func);
            continue;
        }

        let callee_is_inline = callee_func.is_inline();
        let same_module = callee_func.module_env.get_id() == caller_module_id;
        let involves_inline = caller_is_inline || callee_is_inline;

        // Determine which calls to check now (before inlining)
        match (same_module, involves_inline) {
            // Same module: always accessible, no check needed
            (true, _) => {
                functions_with_callers.insert(*callee_id);
            },

            // Cross-module without inline: don't check now, will be checked in second pass after inlining
            // (inline functions disappear after inlining, so we must check them before;
            //  non-inline functions remain, so we check them in a second pass after inlining)
            (false, false) => {
                functions_with_callers.insert(*callee_id);
            },

            // Cross-module with inline: check now (before inline functions disappear)
            (false, true) => {
                let is_accessible = check_inline_call_visibility(
                    env,
                    caller_func,
                    &callee_func,
                    caller_module_id,
                    caller_is_script,
                    sites,
                );

                if is_accessible {
                    functions_with_callers.insert(*callee_id);
                } else {
                    functions_with_inaccessible_callers.insert(*callee_id);
                }
            },
        }
    }
}

/// Check if an inline function call is accessible.
fn check_inline_call_visibility(
    env: &GlobalEnv,
    caller_func: &FunctionEnv,
    callee_func: &FunctionEnv,
    caller_module_id: ModuleId,
    caller_is_script: bool,
    sites: &BTreeSet<NodeId>,
) -> bool {
    // Scripts can only call public functions
    if caller_is_script && callee_func.visibility() != Visibility::Public {
        generic_error(env, "a script ", "it is not public", sites, callee_func);
        return false;
    }

    match callee_func.visibility() {
        Visibility::Public => true,

        Visibility::Friend => {
            check_friend_call_accessibility(env, caller_func, callee_func, caller_module_id, sites)
        },

        Visibility::Private => {
            private_to_module_error(env, sites, caller_func, callee_func);
            false
        },
    }
}

/// Check friend visibility for function calls.
///
/// Friend functions can be called in two ways:
/// 1. Explicit friend declaration: `friend module_name;`
/// 2. Package visibility: `public(package)` - callable within the same package
fn check_friend_call_accessibility(
    env: &GlobalEnv,
    caller_func: &FunctionEnv,
    callee_func: &FunctionEnv,
    caller_module_id: ModuleId,
    sites: &BTreeSet<NodeId>,
) -> bool {
    // Case 1: Explicit friend declaration
    if callee_func.module_env.has_friend(&caller_module_id) {
        return true; // Caller is an explicit friend
    }

    // Case 2: Package visibility
    if !callee_func.has_package_visibility() {
        // Not a friend and not package-visible: error
        not_a_friend_error(env, sites, caller_func, callee_func);
        return false;
    }

    // Has package visibility: check if caller is in the same package
    check_package_visibility(env, caller_func, callee_func, sites)
}

/// Check if a package-visible function can be called (must be same package).
fn check_package_visibility(
    env: &GlobalEnv,
    caller_func: &FunctionEnv,
    callee_func: &FunctionEnv,
    sites: &BTreeSet<NodeId>,
) -> bool {
    let caller_addr = caller_func.module_env.self_address();
    let callee_addr = callee_func.module_env.self_address();

    // Must have same address to be in the same package
    if caller_addr != callee_addr {
        call_package_fun_from_diff_addr_error(env, sites, caller_func, callee_func);
        return false;
    }

    // Same address: check if both are in the same package
    // TODO(#13745): improve package detection - currently only works for primary targets
    if callee_func.module_env.is_primary_target() {
        // Both caller and callee are primary targets with the same address.
        // This means they are in the same package, so the compiler should have already
        // inferred a friend declaration between them.
        //
        // We should never reach this code path because:
        // - If they're in the same package, friend declaration should exist
        // - If friend declaration exists, we would have returned true earlier
        //
        // If we're here, it's a compiler bug - friend inference failed.
        panic!(
            "{} should have friend {}",
            callee_func.module_env.get_full_name_str(),
            caller_func.module_env.get_full_name_str()
        );
    }

    // Callee is not a primary target: use experimental package visibility check
    let options = env
        .get_extension::<Options>()
        .expect("Options is available");
    if options.experiment_on(Experiment::UNSAFE_PACKAGE_VISIBILITY) {
        true // Experiment enabled: allow same-address calls
    } else {
        call_package_fun_from_diff_package_error(env, sites, caller_func, callee_func);
        false
    }
}

/// Check for unused private structs in a module.
fn check_unused_structs(env: &mut GlobalEnv, module_id: ModuleId) {
    let module = env.get_module(module_id);
    for struct_env in module.get_structs() {
        if struct_env.get_visibility() == Visibility::Private && struct_env.get_users().is_empty()
        {
            let msg = format!(
                "Struct `{}` is unused: it has no current users and is private to its module.",
                struct_env.get_full_name_with_address(),
            );
            env.diag(Severity::Warning, &struct_env.get_loc(), &msg);
        }
    }
}

/// Check for unused constants in a module.
fn check_unused_constants(env: &mut GlobalEnv, module_id: ModuleId) {
    let module = env.get_module(module_id);
    for const_env in module.get_named_constants() {
        if const_env.get_using_functions().is_empty() {
            let msg = format!(
                "Constant `{}` is unused.",
                const_env.get_name().display(env.symbol_pool()),
            );
            env.diag(Severity::Warning, &const_env.get_loc(), &msg);
        }
    }
}

/// Report unused private and friendless public(friend) functions.
fn report_unused_functions(
    env: &mut GlobalEnv,
    private_funcs: &BTreeSet<QualifiedFunId>,
    functions_with_callers: &BTreeSet<QualifiedFunId>,
    functions_with_inaccessible_callers: &BTreeSet<QualifiedFunId>,
) {
    for &func_id in private_funcs {
        if functions_with_callers.contains(&func_id) {
            continue; // Function has accessible callers
        }

        let func = env.get_function(func_id);

        // Entry functions in scripts don't need callers
        if func.module_env.get_name().is_script() {
            continue;
        }

        let msg = if functions_with_inaccessible_callers.contains(&func_id) {
            format!(
                "Function `{}` may be unused: it has callers, but none with access.",
                func.get_full_name_with_address(),
            )
        } else {
            let reason = if matches!(func.visibility(), Visibility::Private) {
                "is private to its module"
            } else {
                "is `public(friend)` but its module has no friends"
            };
            format!(
                "Function `{}` is unused: it has no current callers and {}.",
                func.get_full_name_with_address(),
                reason
            )
        };

        env.diag(Severity::Warning, &func.get_id_loc(), &msg);
    }
}

/// Check accessibility after inlining.
///
/// After inlining, inline function calls have been replaced with their bodies.
/// This pass checks:
/// 1. Regular (non-inline) function calls respect visibility rules
/// 2. Struct operations (pack/unpack/field access/storage) respect visibility rules
///    (including operations that came from inlined function bodies)
/// 3. Inline function bodies don't contain calls that would violate accessibility
///    when inlined into different contexts
fn check_after_inlining(env: &mut GlobalEnv) {
    // Collect module IDs first to avoid borrow checker issues
    let module_ids: Vec<_> = env
        .get_modules()
        .filter(|m| m.is_primary_target())
        .map(|m| m.get_id())
        .collect();

    for caller_module_id in module_ids {
        let caller_module = env.get_module(caller_module_id);

        for caller_func in caller_module.get_functions() {
            // 1. Check struct operations (pack/unpack/field access/storage ops)
            //    These operations may have come from inlined function bodies
            check_privileged_operations_on_structs(env, &caller_func);

            // 2. Check inline function bodies for problematic calls
            //    (calls that would be inaccessible when inlined into certain contexts)
            check_inline_bodies_with_inaccessible_calls(env, &caller_func);

            // 3. Check regular (non-inline) function call accessibility
            //    Inline calls were already checked before inlining and no longer exist
            let func_id = caller_func.get_qualified_id();
            check_non_inline_call_visibility(env, func_id, caller_module_id);
        }
    }
}

/// Check visibility of non-inline function calls.
///
/// Inline calls are skipped - they were already checked before inlining and no longer exist.
fn check_non_inline_call_visibility(
    env: &GlobalEnv,
    caller_id: QualifiedId<FunId>,
    caller_module_id: ModuleId,
) {
    let caller_func = env.get_function(caller_id);
    let Some(def) = caller_func.get_def() else {
        return;
    };
    let caller_is_inline = caller_func.is_inline();
    let caller_is_script = caller_func.module_env.get_name().is_script();

    let callees_with_sites = def.used_funs_with_uses();
    for (callee_id, call_sites) in &callees_with_sites {
        let callee_func = env.get_function(*callee_id);

        // Special case: script functions can never be called
        if callee_func.module_env.is_script_module() {
            calling_script_function_error(env, call_sites, &callee_func);
            continue;
        }

        // Skip if same module (always accessible) or involves inline functions
        // (inline calls were already checked and are now gone)
        let same_module = callee_func.module_env.get_id() == caller_module_id;
        let involves_inline = callee_func.is_inline() || caller_is_inline;
        if same_module || involves_inline {
            continue;
        }

        // Check visibility rules
        match callee_func.visibility() {
            // Public functions are always accessible
            Visibility::Public => {},

            // Scripts can only call public functions
            _ if caller_is_script => {
                generic_error(
                    env,
                    "a script ",
                    "it is not public",
                    call_sites,
                    &callee_func,
                );
            },

            // Friend visibility: check friend relationship or package visibility
            Visibility::Friend => {
                check_friend_call_accessibility(
                    env,
                    &caller_func,
                    &callee_func,
                    caller_module_id,
                    call_sites,
                );
            },

            // Private functions are only accessible within their module
            Visibility::Private => {
                private_to_module_error(env, call_sites, &caller_func, &callee_func);
            },
        }
    }
}

/// Check inline function bodies for calls that become inaccessible when inlined.
///
/// When an inline function is inlined, its body is copied into the caller. If the inline function
/// calls a less-visible helper, that call executes in the caller's context and may fail.
///
/// Example:
/// ```move
/// module A {
///   public inline fun foo() { private_helper() }
///   fun private_helper() {}
/// }
/// module B {
///   fun bar() { A::foo() }  // After inlining: bar calls private_helper - ERROR
/// }
/// ```
fn check_inline_bodies_with_inaccessible_calls(env: &GlobalEnv, caller_func: &FunctionEnv) {
    if !caller_func.is_inline() {
        return;
    }

    let Some(def) = caller_func.get_def() else {
        return;
    };

    // Private inline functions only callable within their module - no issue
    if caller_func.visibility() == Visibility::Private {
        return;
    }

    for (callee_id, sites) in &def.used_funs_with_uses() {
        let callee_func = env.get_function(*callee_id);
        if let Some(problem_context) = check_inlined_call_accessibility(caller_func, &callee_func) {
            warn_inline_accessibility_issue(
                env,
                caller_func,
                &callee_func,
                sites,
                &problem_context,
            );
        }
    }
}

/// Check if a call within an inline function becomes inaccessible when inlined.
///
/// Returns Some(context) describing where the problem occurs, or None if no problem.
fn check_inlined_call_accessibility(
    inline_func: &FunctionEnv,
    callee_func: &FunctionEnv,
) -> Option<String> {
    let inline_vis = inline_func.visibility();
    let callee_vis = callee_func.visibility();

    match (inline_vis, callee_vis) {
        // Public callee: accessible everywhere
        (_, Visibility::Public) => None,

        // Public inline calling non-public: callee not accessible everywhere inline is
        (Visibility::Public, Visibility::Friend | Visibility::Private) => Some(String::new()),

        // Friend inline calling private: callee not accessible in friend modules
        (Visibility::Friend, Visibility::Private) => {
            let module = &inline_func.module_env;
            Some(format!(
                ", such as in a {}friend module of `{}`",
                if module.has_no_friends() {
                    "(future) "
                } else {
                    ""
                },
                module.get_full_name_str()
            ))
        },

        // Friend inline calling friend: check if friend sets are compatible
        (Visibility::Friend, Visibility::Friend) => {
            check_friend_inline_calling_friend(inline_func, callee_func)
        },

        (Visibility::Private, _) => unreachable!("filtered out earlier"),
    }
}

/// Check friend inline calling friend - requires friend set compatibility.
///
/// The inline function can be called from its friends. When inlined, those same friends
/// must be able to call the callee.
fn check_friend_inline_calling_friend(
    inline_func: &FunctionEnv,
    callee_func: &FunctionEnv,
) -> Option<String> {
    let inline_has_package_vis = inline_func.has_package_visibility();
    let callee_has_package_vis = callee_func.has_package_visibility();

    match (inline_has_package_vis, callee_has_package_vis) {
        // Callee has package visibility: check same package
        (_, true) => {
            let same_package = callee_func.module_env.is_primary_target()
                && callee_func.module_env.self_address() == inline_func.module_env.self_address();
            if same_package {
                None
            } else {
                Some(String::new())
            }
        },

        // Inline has package visibility, callee has explicit friends:
        // some package modules may not be friends of callee
        (true, false) => Some(format!(
            ", such as from a module in this package that is not a friend of `{}`",
            callee_func.module_env.get_full_name_str()
        )),

        // Both have explicit friends: inline's friends must be subset of callee's friends
        (false, false) => {
            let inline_friends = inline_func.module_env.get_friend_modules();
            let callee_friends = callee_func.module_env.get_friend_modules();

            if inline_friends.is_subset(&callee_friends) {
                None
            } else {
                Some(format!(
                    ", such as from a module that is a friend of `{}` but not a friend of `{}`",
                    inline_func.module_env.get_full_name_str(),
                    callee_func.module_env.get_full_name_str()
                ))
            }
        },
    }
}

/// Emit warning for inline function calling inaccessible function.
fn warn_inline_accessibility_issue(
    env: &GlobalEnv,
    inline_func: &FunctionEnv,
    callee_func: &FunctionEnv,
    sites: &BTreeSet<NodeId>,
    problem_context: &str,
) {
    let label: Vec<_> = sites
        .first()
        .map(|node_id| {
            (
                env.get_node_loc(*node_id),
                format!(
                    "inline expansion calls {} function that may not be accessible in all locations that `{}` can be called",
                    function_visibility_description(callee_func),
                    inline_func.get_full_name_str()
                ),
            )
        })
        .into_iter()
        .collect();

    env.diag_with_primary_and_labels(
        Severity::Warning,
        &inline_func.get_id_loc(),
        &format!(
            "{} inline function `{}` cannot be called from all locations it is accessible",
            function_visibility_description(inline_func),
            inline_func.get_name_str()
        ),
        &format!(
            "if called from a location where `{}` is not accessible{}",
            callee_func.get_full_name_str(),
            problem_context
        ),
        label,
    );
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
