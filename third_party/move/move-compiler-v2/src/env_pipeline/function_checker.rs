// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Do a few checks of functions and function calls.

use crate::{experiments::Experiment, Options};
use codespan_reporting::diagnostic::Severity;
use legacy_move_compiler::shared::known_attributes::LintAttribute;
use move_binary_format::file_format::Visibility;
use move_model::{
    ast::{Attribute, ExpData, Operation, Pattern},
    metadata::LanguageVersion,
    model::{
        FunId, FunctionEnv, GlobalEnv, Loc, NamedConstantEnv, NodeId, Parameter, QualifiedId,
        StructEnv, StructId,
    },
    ty::Type,
};
use std::{collections::BTreeSet, iter::Iterator, vec::Vec};

/// Attribute names that suppress unused warnings in general
/// - `deprecated`: Marks items that are deprecated but may not be removed
const SHARED_SUPPRESSION_ATTRS: &[&str] = &["deprecated"];

/// Additional attribute names that suppress unused warnings for functions only.
/// - `persistent`: Marks a function as being persistent on upgrade (behave like a public function)
const FUNC_ONLY_SUPPRESSION_ATTRS: &[&str] = &["persistent"];

/// Additional attribute names that suppress unused warnings for structs only.
/// - `resource_group`: Empty marker structs used by VM for storage optimization
/// - `resource_group_member`: Structs belonging to a resource group, used by VM verifier
const STRUCT_ONLY_SUPPRESSION_ATTRS: &[&str] = &["resource_group", "resource_group_member"];

/// Functions excluded from unused checks, format: (address, module, function).
/// - `None` for address or module means "any" (wildcard).
/// - `init_module`: VM hook called automatically when module is published
const EXCLUDED_FUNCTIONS: &[(Option<&str>, Option<&str>, &str)] = &[(None, None, "init_module")];

/// Checker name for unused warnings
/// Suppressing using linter syntax #[lint::skip(unused)]
const UNUSED_CHECK_NAME: &str = "unused";

// ===============================================================================================
// Access checking

struct StructOp {
    func: QualifiedId<FunId>,
    struct_id: QualifiedId<StructId>,
    kind: StructOpKind,
    site: NodeId,
}

#[derive(Clone, Copy)]
enum StructOpKind {
    Pack,
    Unpack,
    FieldAccess,
    VariantTest,
    StorageOp,
}

impl std::fmt::Display for StructOpKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StructOpKind::Pack => write!(f, "pack"),
            StructOpKind::Unpack => write!(f, "unpack"),
            StructOpKind::FieldAccess => write!(f, "field access"),
            StructOpKind::VariantTest => write!(f, "variant test"),
            StructOpKind::StorageOp => write!(f, "storage operation"),
        }
    }
}

/// Check function call access rules (before inlining).
pub fn check_access_before_inlining(env: &GlobalEnv) {
    for module in env.get_modules() {
        if !module.is_primary_target() {
            continue;
        }

        for caller in module.get_functions() {
            let Some(def) = caller.get_def() else {
                continue;
            };
            let caller_is_inline = caller.is_inline();
            for (callee_id, sites) in def.used_funs_with_uses() {
                let callee = env.get_function(callee_id);

                // Only check calls involving inline functions
                if !caller_is_inline && !callee.is_inline() {
                    continue;
                }

                check_function_call(&caller, &callee, &sites);
            }
        }
    }
}

/// Check struct operation and function call access rules (after inlining).
pub fn check_access_after_inlining(env: &GlobalEnv) {
    for module in env.get_modules() {
        if !module.is_primary_target() {
            continue;
        }

        for caller in module.get_functions() {
            let Some(def) = caller.get_def() else {
                continue;
            };

            // Check 1: struct operations
            for op in collect_struct_ops(&caller) {
                check_struct_op(env, &op);
            }

            // Check 2: calls in inline functions that may be inaccessible when expanded
            check_inline_function_calls(&caller);

            // Check 3: regular function calls (skip inline-related, already checked)
            if caller.is_inline() {
                continue;
            }

            for (callee_id, sites) in def.used_funs_with_uses() {
                let callee = env.get_function(callee_id);

                if callee.is_inline() {
                    for site in &sites {
                        env.diag(
                            Severity::Bug,
                            &env.get_node_loc(*site),
                            &format!(
                                "call to inline function `{}` should have been expanded",
                                callee.get_name_str(),
                            ),
                        );
                    }
                }

                check_function_call(&caller, &callee, &sites);
            }
        }
    }
}

/// Check visibility rules for a cross-module function call.
/// Reports errors for calls to script functions, private functions,
/// and friend/package-visible functions from non-authorized callers.
fn check_function_call(caller: &FunctionEnv, callee: &FunctionEnv, sites: &BTreeSet<NodeId>) {
    let env = caller.module_env.env;
    let report = |msg: &str, primary: &str| {
        let call_sites: Vec<_> = sites
            .iter()
            .map(|id| (env.get_node_loc(*id), "called here".to_owned()))
            .collect();
        env.diag_with_primary_and_labels(
            Severity::Error,
            &callee.get_id_loc(),
            msg,
            primary,
            call_sites,
        );
    };

    let caller_desc = || {
        let caller_name = caller.get_full_name_with_address();
        if caller.is_inline() {
            format!("called from inline function `{caller_name}`")
        } else {
            format!("called from function `{caller_name}`")
        }
    };

    // 1. Callee is script → error
    if callee.module_env.is_script_module() {
        report(
            &format!(
                "script function `{}` cannot be called from Move code",
                callee.get_name_str()
            ),
            "script function",
        );
        return;
    }

    // Now: callee is not a script
    // 2. Same module → allowed
    if caller.module_env.get_id() == callee.module_env.get_id() {
        return;
    }

    // Now: callee is not a script; cross-module
    // 3. Callee is public → allowed
    if callee.visibility() == Visibility::Public {
        return;
    }

    // Now: callee is not a script; cross-module; callee is not public
    // 4. Caller is script → error (scripts can only call public functions)
    if caller.module_env.is_script_module() {
        report(
            &format!(
                "function `{}` cannot be called from a script because it is not public",
                callee.get_full_name_with_address()
            ),
            "called from a script",
        );
        return;
    }

    // Now: callee is not a script; cross-module; callee is not public; caller is not a script
    let callee_name = callee.get_full_name_with_address();

    // 5. Callee is private → error
    if callee.visibility() == Visibility::Private {
        report(
            &format!(
                "function `{callee_name}` is private to module `{}`",
                callee.module_env.get_full_name_str()
            ),
            &caller_desc(),
        );
        return;
    }

    // Now: cross-module; callee is package or friend visible
    // 6. Caller is explicit friend → allowed
    if callee.module_env.has_friend(&caller.module_env.get_id()) {
        return;
    }

    // Now: cross-module; callee is package or friend visible; caller is not a friend
    if callee.has_package_visibility() {
        // 7. Callee has package visibility → check address and package for more informative messages
        if callee.module_env.self_address() != caller.module_env.self_address() {
            // Now: different address
            report(
                &format!(
                    "package function `{callee_name}` cannot be called from a different address"
                ),
                &caller_desc(),
            );
            return;
        }
        // Now: same address; callee is from a dependency package
        let options = env
            .get_extension::<Options>()
            .expect("Options is available");
        if options.experiment_on(Experiment::UNSAFE_PACKAGE_VISIBILITY) {
            return;
        }
        report(
            &format!("package function `{callee_name}` cannot be called from a different package"),
            &caller_desc(),
        );
    } else {
        // 8. Callee has friend but not package visibility
        report(
            &format!(
                "friend function `{callee_name}` cannot be called from `{}` (not a friend of `{}`)",
                caller.module_env.get_full_name_str(),
                callee.module_env.get_full_name_str()
            ),
            &caller_desc(),
        );
    }
}

// ===============================================================================================
// Access checking: struct operations

/// Get the struct type from a storage operation's type instantiation.
fn get_storage_op_struct(env: &GlobalEnv, node_id: NodeId) -> Option<QualifiedId<StructId>> {
    let inst = env.get_node_instantiation(node_id);
    let (s, _) = inst.first().and_then(|t| t.get_struct(env))?;
    Some(s.get_qualified_id())
}

/// Check access rules for struct operations (pack, unpack, field access, variant test, storage).
/// Reports errors/warnings for visibility violations and inline expansion issues.
fn check_struct_op(env: &GlobalEnv, op: &StructOp) {
    let func = env.get_function(op.func);
    let struct_env = env.get_struct(op.struct_id);
    let same_module = struct_env.module_env.get_id() == func.module_env.get_id();
    let may_expand_outside = func.is_inline() && func.visibility() != Visibility::Private;

    let struct_name = struct_env.get_full_name_str();
    let struct_module = struct_env.module_env.get_full_name_str();
    let op_desc = format!("{} on `{struct_name}`", op.kind);
    let label = vec![(env.get_node_loc(op.site), format!("{} here", op.kind))];

    // helper to report an access error
    let report_error = |extra: &str| {
        env.diag_with_labels(
            Severity::Error,
            &func.get_id_loc(),
            &format!("Invalid operation: {op_desc} can only be done within module `{struct_module}`{extra}"),
            label.clone(),
        );
    };

    // helper to report an access warning
    let report_warning = || {
        env.diag_with_labels(
            Severity::Warning,
            &func.get_id_loc(),
            &format!(
                "{op_desc} can only be done within module `{struct_module}`, but `{}` could be called (and expanded) outside",
                func.get_full_name_str()
            ),
            label.clone(),
        );
    };

    // 1. Same module and won't expand outside → allowed
    if same_module && !may_expand_outside {
        return;
    }

    match op.kind {
        // 2. Storage ops → error (cross-module) or warning (inline expansion)
        StructOpKind::StorageOp => {
            if same_module {
                // Now: same module, but user function can be expanded outside → warning
                report_warning();
            } else {
                // Now: cross-module; not allowed on storage operations
                report_error("");
            }
        },
        // 3. Pack/unpack/field access/variant test → check struct visibility
        StructOpKind::Pack
        | StructOpKind::Unpack
        | StructOpKind::FieldAccess
        | StructOpKind::VariantTest => {
            let struct_visibility_supported =
                env.language_version().language_version_for_public_struct();

            let visibility = if struct_visibility_supported {
                struct_env.get_visibility()
            } else {
                // Before public struct support, treat all structs as private
                Visibility::Private
            };

            if visibility == Visibility::Public {
                // Now: public struct → allowed
                return;
            }

            if same_module
                || (visibility == Visibility::Friend
                    && struct_env.module_env.has_friend(&func.module_env.get_id()))
            {
                // Now: same module, or caller is friend
                // -> warning if user function can be expanded outside
                if may_expand_outside {
                    report_warning();
                }
                // -> allowed otherwise
                return;
            }

            // Now: cross-module; private or friend without friend relationship → error
            let extra = if visibility == Visibility::Friend {
                let scope = if struct_env.has_package_visibility() {
                    "modules in the same package"
                } else {
                    "friend modules"
                };
                format!(" or {scope}")
            } else {
                String::new()
            };
            report_error(&extra);
        },
    }
}

/// Collect all struct operations in a function body (excluding spec blocks).
///
/// Collects:
/// - Storage ops: exists, borrow_global, move_from, move_to
/// - Field access: select, select_variants, test_variants
/// - Pack: struct construction
/// - Unpack: struct patterns in let/match/lambda
fn collect_struct_ops(func: &FunctionEnv) -> Vec<StructOp> {
    let Some(body) = func.get_def() else {
        return Vec::new();
    };

    let mut ops = Vec::new();
    // Track nesting depth in spec blocks. Spec blocks cannot currently be nested,
    // but we use depth counting defensively in case this changes.
    let mut spec_depth = 0usize;

    body.visit_pre_post(&mut |post, exp: &ExpData| {
        // Skip spec blocks
        if matches!(exp, ExpData::SpecBlock(..)) {
            spec_depth = if post {
                spec_depth.saturating_sub(1)
            } else {
                spec_depth + 1
            };
        }

        // We skip `post` since the exp has been visited during `pre`
        if post || spec_depth > 0 {
            return true;
        }

        // Collect operation from this expression
        collect_struct_op_from_exp(func, exp, &mut ops);
        true
    });

    ops
}

/// Collect struct operation from a single expression.
fn collect_struct_op_from_exp(func: &FunctionEnv, exp: &ExpData, ops: &mut Vec<StructOp>) {
    let env = func.module_env.env;
    let func_id = func.get_qualified_id();

    let mut push = |struct_id, kind, site| {
        ops.push(StructOp {
            func: func_id,
            struct_id,
            kind,
            site,
        });
    };

    match exp {
        // Operations in Call expressions
        ExpData::Call(id, oper, _) => match oper {
            // Storage operations
            Operation::Exists(_)
            | Operation::BorrowGlobal(_)
            | Operation::MoveFrom
            | Operation::MoveTo => {
                if let Some(struct_id) = get_storage_op_struct(env, *id) {
                    push(struct_id, StructOpKind::StorageOp, *id);
                }
            },
            // Field access
            Operation::Select(mid, sid, _) | Operation::SelectVariants(mid, sid, _) => {
                push(mid.qualified(*sid), StructOpKind::FieldAccess, *id);
            },
            // Variant test
            Operation::TestVariants(mid, sid, _) => {
                push(mid.qualified(*sid), StructOpKind::VariantTest, *id);
            },
            // Pack
            Operation::Pack(mid, sid, _) => {
                push(mid.qualified(*sid), StructOpKind::Pack, *id);
            },
            // Non-struct operations (listed explicitly to catch future additions)
            Operation::MoveFunction(_, _)
            | Operation::Closure(_, _, _)
            | Operation::Tuple
            | Operation::SpecFunction(_, _, _)
            | Operation::UpdateField(_, _, _)
            | Operation::Behavior(_, _)
            | Operation::Result(_)
            | Operation::Index
            | Operation::Slice
            | Operation::Range
            | Operation::Implies
            | Operation::Iff
            | Operation::Identical
            | Operation::Add
            | Operation::Sub
            | Operation::Mul
            | Operation::Mod
            | Operation::Div
            | Operation::BitOr
            | Operation::BitAnd
            | Operation::Xor
            | Operation::Shl
            | Operation::Shr
            | Operation::And
            | Operation::Or
            | Operation::Eq
            | Operation::Neq
            | Operation::Lt
            | Operation::Gt
            | Operation::Le
            | Operation::Ge
            | Operation::Copy
            | Operation::Move
            | Operation::Not
            | Operation::Cast
            | Operation::Negate
            | Operation::Borrow(_)
            | Operation::Deref
            | Operation::Freeze(_)
            | Operation::Abort(_)
            | Operation::Vector
            | Operation::Len
            | Operation::TypeValue
            | Operation::TypeDomain
            | Operation::ResourceDomain
            | Operation::Global(_)
            | Operation::CanModify
            | Operation::Old
            | Operation::Trace(_)
            | Operation::EmptyVec
            | Operation::SingleVec
            | Operation::UpdateVec
            | Operation::ConcatVec
            | Operation::IndexOfVec
            | Operation::ContainsVec
            | Operation::InRangeRange
            | Operation::InRangeVec
            | Operation::RangeVec
            | Operation::MaxU8
            | Operation::MaxU16
            | Operation::MaxU32
            | Operation::MaxU64
            | Operation::MaxU128
            | Operation::MaxU256
            | Operation::Bv2Int
            | Operation::Int2Bv
            | Operation::AbortFlag
            | Operation::AbortCode
            | Operation::WellFormed
            | Operation::BoxValue
            | Operation::UnboxValue
            | Operation::EmptyEventStore
            | Operation::ExtendEventStore
            | Operation::EventStoreIncludes
            | Operation::EventStoreIncludedIn
            | Operation::NoOp => {},
        },

        // Match expression unpacks the discriminator
        ExpData::Match(_, discriminator, _) => {
            let id = discriminator.node_id();
            if let Type::Struct(mid, sid, _) = env.get_node_type(id).drop_reference() {
                push(mid.qualified(sid), StructOpKind::Unpack, id);
            }
        },

        // Patterns in let/assign/lambda can unpack structs
        ExpData::Assign(_, pat, _)
        | ExpData::Block(_, pat, _, _)
        | ExpData::Lambda(_, pat, _, _, _) => {
            pat.visit_pre_post(&mut |post, p| {
                if !post {
                    if let Pattern::Struct(id, sid, _, _) = p {
                        push(sid.to_qualified_id(), StructOpKind::Unpack, *id);
                    }
                }
            });
        },

        // Non-struct-op expressions (listed explicitly to catch future additions)
        ExpData::Invalid(_)
        | ExpData::Value(_, _)
        | ExpData::LocalVar(_, _)
        | ExpData::Temporary(_, _)
        | ExpData::Invoke(_, _, _)
        | ExpData::Quant(_, _, _, _, _, _)
        | ExpData::IfElse(_, _, _, _)
        | ExpData::Return(_, _)
        | ExpData::Sequence(_, _)
        | ExpData::Loop(_, _)
        | ExpData::LoopCont(_, _, _)
        | ExpData::Mutate(_, _, _)
        | ExpData::SpecBlock(_, _) => {},
    }
}

// ===============================================================================================
// Access checking: inline function calls

/// Check if a non-private inline function calls functions that may be inaccessible when expanded
fn check_inline_function_calls(func: &FunctionEnv) {
    // Only check non-private inline functions
    if !func.is_inline() || func.visibility() == Visibility::Private {
        return;
    }
    let env = func.module_env.env;
    let Some(def) = func.get_def() else {
        // Native functions have no definitions, but they cannot be inline.
        env.diag(
            Severity::Bug,
            &func.get_loc(),
            &format!(
                "inline function `{}` should have a definition",
                func.get_name_str()
            ),
        );
        return;
    };
    for (callee_id, sites) in &def.used_funs_with_uses() {
        let callee = env.get_function(*callee_id);
        check_inline_callee_visibility(env, func, &callee, sites);
    }
}

/// Warn when an inline function calls a callee that may be inaccessible after expansion.
fn check_inline_callee_visibility(
    env: &GlobalEnv,
    caller: &FunctionEnv,
    callee: &FunctionEnv,
    sites: &BTreeSet<NodeId>,
) {
    let caller_vis = caller.visibility();
    let callee_vis = callee.visibility();

    let report_warning = |context: &str| {
        let caller_vis_desc =
            visibility_description(caller.visibility(), caller.has_package_visibility());
        let callee_vis_desc =
            visibility_description(callee.visibility(), callee.has_package_visibility());

        let label: Vec<_> = sites
            .first()
            .map(|id| {
                (
                    env.get_node_loc(*id),
                    format!(
                        "{callee_vis_desc} function may not be accessible in all contexts where `{}` can be called",
                        caller.get_name_str()
                    ),
                )
            })
            .into_iter()
            .collect();

        env.diag_with_primary_and_labels(
            Severity::Warning,
            &caller.get_id_loc(),
            &format!(
                "{caller_vis_desc} inline function `{}` calls `{}` which may not be accessible when expanded",
                caller.get_name_str(),
                callee.get_full_name_str(),
            ),
            &format!("contains calls that may be inaccessible when expanded{context}"),
            label,
        );
    };

    // 1. Callee is public → safe
    if callee_vis == Visibility::Public {
        return;
    }

    // Now: callee is not public
    match caller_vis {
        Visibility::Public => {
            // Now: callee is not public; caller is public → warning
            report_warning("");
        },
        Visibility::Private => {
            // Now: callee is not public; caller is private → safe
        },
        Visibility::Friend => {
            if callee_vis == Visibility::Private {
                // Now: callee is private; caller is friend → warning
                let future = if caller.module_env.has_no_friends() {
                    "(future) "
                } else {
                    ""
                };
                report_warning(&format!(
                    ", such as in a {future}friend module of `{}`",
                    caller.module_env.get_full_name_str()
                ));
            } else {
                // Now: both are friend → check compatibility
                if let Some(context) = check_friend_visibility_compatibility(caller, callee) {
                    report_warning(&context);
                }
            }
        },
    }
}

/// Check if friend/package visibility is compatible between caller and callee.
/// Returns None if safe, Some(context) if warning needed.
fn check_friend_visibility_compatibility(
    caller: &FunctionEnv,
    callee: &FunctionEnv,
) -> Option<String> {
    let callee_is_package = callee.has_package_visibility();
    let caller_is_package = caller.has_package_visibility();

    // 1. Callee is package, same package → safe
    // 2. Callee is package, different package → warning
    if callee_is_package {
        // TODO(#13745): refine when we have package info in modules
        let same_package = callee.module_env.is_primary_target()
            && callee.module_env.self_address() == caller.module_env.self_address();
        return if same_package {
            None
        } else {
            Some(String::new())
        };
    }

    // Now: callee is friend (not package)
    // 3. Caller is package, callee is friend → warning
    if caller_is_package {
        return Some(format!(
            ", such as from a module in this package that is not a friend of `{}`",
            callee.module_env.get_full_name_str()
        ));
    }

    // Now: both are friend (not package)
    // 4. Both friend: caller's friends ⊆ callee's friends → safe
    // 5. Both friend: some caller friend is not callee friend → warning
    let caller_friends = caller.module_env.get_friend_modules();
    let callee_friends = callee.module_env.get_friend_modules();
    let all_covered = caller_friends.difference(&callee_friends).next().is_none();

    if all_covered {
        None
    } else {
        Some(format!(
            ", e.g., if called from a non-friend of `{}`",
            callee.module_env.get_full_name_str()
        ))
    }
}

fn visibility_description(vis: Visibility, has_package: bool) -> &'static str {
    match vis {
        Visibility::Public => "public",
        Visibility::Friend if has_package => "package",
        Visibility::Friend => "friend",
        Visibility::Private => "private",
    }
}

// ===============================================================================================
// Function type parameter checks

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

/// Check for unused private functions and emit warnings.
///
/// TODO(#18830): Add separate checkers for:
/// - friend functions in modules without friends
/// - functions only reachable from inaccessible callers
/// - a group of private functions that only call each other
pub fn check_unused_functions(env: &GlobalEnv) {
    for module in env.get_modules() {
        if module.is_primary_target() {
            for func in module.get_functions() {
                if should_warn_unused_function(&func) {
                    let msg = format!(
                        "function `{}` is unused. Remove it, or suppress warning with `#[test_only]` (if test-only), `#[verify_only]` (if verify-only), or `#[lint::skip(unused)]`.",
                        func.get_name_str(),
                    );
                    env.diag(Severity::Warning, &func.get_id_loc(), &msg);
                }
            }
        }
    }
}

/// Check for unused structs/enums and emit warnings.
pub fn check_unused_structs(env: &GlobalEnv) {
    for module in env.get_modules() {
        if module.is_primary_target() {
            for struct_env in module.get_structs() {
                if should_warn_unused_struct(&struct_env) {
                    let entity_type = if struct_env.has_variants() {
                        "enum"
                    } else {
                        "struct"
                    };
                    let msg = format!(
                        "{} `{}` is unused in current package. Remove it (if not published), or suppress warning with `#[test_only]` (if test-only), `#[verify_only]` (if verify-only), or `#[lint::skip(unused)]`.",
                        entity_type,
                        struct_env.get_name_str()
                    );
                    env.diag(Severity::Warning, &struct_env.get_loc(), &msg);
                }
            }
        }
    }
}

/// Check for unused constants and emit warnings.
pub fn check_unused_constants(env: &GlobalEnv) {
    for module in env.get_modules() {
        if module.is_primary_target() {
            for const_env in module.get_named_constants() {
                if should_warn_unused_constant(&const_env) {
                    let msg = format!(
                        "constant `{}` is unused. Remove it, or suppress warning with `#[test_only]` (if test-only), `#[verify_only]` (if verify-only), or `#[lint::skip(unused)]`.",
                        const_env.get_name().display(env.symbol_pool()),
                    );
                    env.diag(Severity::Warning, &const_env.get_loc(), &msg);
                }
            }
        }
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

    // Don't warn:
    // - non-private functions
    // - script or entry functions
    // - test_only or verify_only functions or modules
    // - excluded functions (e.g., init_module)
    // - functions with suppression attributes or #[lint::skip(unused)]
    // - functions with callers
    if func.visibility() != Visibility::Private
        || func.is_script_or_entry()
        || func.is_test_or_verify_only()
        || is_excluded_function(func)
        || func.has_attribute(is_suppression_attr)
        || skip_unused_check(env, func.get_attributes())
        || has_users(func)
    {
        return false;
    }

    true
}

/// Check if function has any users (excluding self-recursive use).
fn has_users(func: &FunctionEnv) -> bool {
    if let Some(using_funs) = func.get_using_functions() {
        let func_qfid = func.get_qualified_id();
        // Check if there's any user other than itself
        using_funs.iter().any(|user| *user != func_qfid)
    } else {
        false
    }
}

/// Returns true if struct should be warned as unused.
fn should_warn_unused_struct(struct_env: &StructEnv) -> bool {
    let env = struct_env.module_env.env;

    let is_suppression_attr = |attr: &Attribute| {
        SHARED_SUPPRESSION_ATTRS
            .iter()
            .chain(STRUCT_ONLY_SUPPRESSION_ATTRS.iter())
            .any(|&s| attr.name() == env.symbol_pool().make(s))
    };

    // Don't warn:
    // - non-private structs
    // - ghost memory structs
    // - test_only or verify_only structs or modules
    // - structs with suppression attributes or #[lint::skip(unused)]
    // - structs with users
    if struct_env.get_visibility() != Visibility::Private
        || struct_env.is_ghost_memory()
        || struct_env.is_test_or_verify_only()
        || struct_env.has_attribute(is_suppression_attr)
        || skip_unused_check(env, struct_env.get_attributes())
        || !struct_env.get_users().is_empty()
    {
        return false;
    }

    true
}

/// Returns true if constant should be warned as unused.
fn should_warn_unused_constant(const_env: &NamedConstantEnv) -> bool {
    let env = const_env.module_env.env;

    let is_suppression_attr = |attr: &Attribute| {
        SHARED_SUPPRESSION_ATTRS
            .iter()
            .any(|&s| attr.name() == env.symbol_pool().make(s))
    };

    // Don't warn:
    // - test_only or verify_only constants or modules
    // - constants with suppression attributes or #[lint::skip(unused)]
    // - constants with users
    if const_env.is_test_or_verify_only()
        || const_env.has_attribute(is_suppression_attr)
        || skip_unused_check(env, const_env.get_attributes())
        || !const_env.get_users().is_empty()
    {
        return false;
    }

    true
}

/// Check if attributes contain #[lint::skip(unused)].
fn skip_unused_check(env: &GlobalEnv, attrs: &[Attribute]) -> bool {
    let lint_skip = env.symbol_pool().make(LintAttribute::SKIP);
    let unused = env.symbol_pool().make(UNUSED_CHECK_NAME);
    Attribute::has(attrs, |attr| {
        attr.name() == lint_skip
            && matches!(attr, Attribute::Apply(_, _, args) if args.iter().any(|arg| arg.name() == unused))
    })
}

/// Check if a function should be excluded from unused checks.
fn is_excluded_function(func: &FunctionEnv) -> bool {
    let env = func.module_env.env;
    let func_name = env.symbol_pool().string(func.get_name());

    EXCLUDED_FUNCTIONS.iter().any(|(ex_addr, ex_mod, ex_func)| {
        // Check function name first (always required)
        if func_name.as_ref() != *ex_func {
            return false;
        }
        // Check module name only if specified in the exclusion rule
        if let Some(m) = ex_mod {
            let module_name = env.symbol_pool().string(func.module_env.get_name().name());
            if module_name.as_ref() != *m {
                return false;
            }
        }
        // Check address only if specified in the exclusion rule
        if let Some(a) = ex_addr {
            let addr = func.module_env.get_name().addr().expect_numerical();
            if addr.to_hex_literal() != *a {
                return false;
            }
        }
        true
    })
}
