// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module checks whether closure expressions are valid, which is done after type inference
//! and lambda lifting. Current checks:
//!
//! - The closure satisfies the ability requirements of it's inferred type. For the
//!   definition of closure abilities, see
//!   [AIP-112](https://github.com/aptos-foundation/AIPs/blob/main/aips/aip-112.md).
//! - The closure does not capture references, as this is currently not allowed.
//! - In a script, the closure cannot have a lambda lifted function.
//! ```

use crate::env_pipeline::lambda_lifter;
use move_binary_format::file_format::Visibility;
use move_core_types::ability::Ability;
use move_model::{
    ast::{Exp, ExpData, Operation, Pattern, TempIndex},
    model::{FunctionEnv, GlobalEnv, QualifiedInstId, StructId},
    symbol::Symbol,
    ty::{ReferenceKind, Type},
    well_known,
};
use std::collections::{BTreeMap, BTreeSet};

/// Checks various properties of lambda expressions in all target module functions.
pub fn check_closures(env: &GlobalEnv) {
    // In verify mode, lambdas are lifted in every module of the transitive target
    // closure and those functions can be inlined into verified targets, so all
    // checks cover that closure. In regular compilation, dependencies are checked
    // when they are compiled themselves.
    let modules = if env.is_verify_mode() {
        env.get_target_modules_transitive_closure()
    } else {
        env.get_primary_target_modules()
    };
    for module_env in modules {
        let is_script_module = module_env.is_script_module();
        for fun_env in module_env.get_functions() {
            if let Some(def) = fun_env.get_def() {
                // In verify mode, closures capturing references are admitted, but only
                // when constructed directly as arguments of calls to retained
                // inline-opaque functions: there, the captured locations are statically
                // visible to the prover's spec instrumentation, which models their
                // effects (havoc and `ensures_of` constraints). `ref_capture_allowed`
                // collects the closure expressions in such positions; it is populated
                // and consumed in one traversal, since a `Call(MoveFunction, ..)` is
                // visited before its `Closure` argument in pre-order.
                if env.is_verify_mode() {
                    check_retained_call_view_conflicts(env, &fun_env, def);
                }
                let mut ref_capture_allowed = std::collections::BTreeSet::new();
                def.visit_pre_order(&mut |e| {
                    if let ExpData::Call(_, Operation::MoveFunction(mid, fid), args) = e {
                        if env.is_verify_mode()
                            && env
                                .get_function(mid.qualified(*fid))
                                .is_inline_opaque_retained()
                        {
                            for arg in args {
                                if matches!(
                                    arg.as_ref(),
                                    ExpData::Call(_, Operation::Closure(..), _)
                                ) {
                                    ref_capture_allowed.insert(arg.node_id());
                                }
                            }
                        }
                    }
                    if let ExpData::Call(id, Operation::Closure(mid, fid, _), args) = e {
                        let mut context_ty = env.get_node_type(*id);
                        let mut function_wrapper_ty = None;
                        if let Some(ty) = context_ty.get_function_wrapper_ty(env) {
                            function_wrapper_ty = Some(context_ty);
                            context_ty = ty;
                        }
                        let required_abilities =
                            env.type_abilities(&context_ty, fun_env.get_type_parameters_ref());
                        let fun_env = env.get_function(mid.qualified(*fid));
                        let is_lambda_lifted = lambda_lifter::is_lambda_lifted_fun(&fun_env);
                        // The function itself has all abilities except `store`, which it only
                        // has if it is public. Notice that since required_abilities is derived
                        // from the function type of the closure, it cannot have `key` ability.
                        if required_abilities.has_ability(Ability::Store)
                            && fun_env.visibility() != Visibility::Public
                            && !fun_env.has_attribute(|attr| {
                                env.symbol_pool().string(attr.name()).as_str()
                                    == well_known::PERSISTENT_ATTRIBUTE
                            })
                        {
                            env.error_with_notes(
                                &env.get_node_loc(*id),
                                &format!(
                                    "function {} is missing the `store` ability",
                                    if is_lambda_lifted {
                                        "resulting from lambda lifting".to_string()
                                    } else {
                                        format!("`{}`", fun_env.get_full_name_str())
                                    },
                                ),
                                vec![
                                    if is_lambda_lifted {
                                        "lambda cannot be reduced to partial application of \
                                        existing function"
                                            .to_string()
                                    } else {
                                        "only public functions or functions with the \
                                        `#[persistent]` attribute can be stored"
                                            .to_string()
                                    },
                                    format!(
                                        "expected function type: `{}`",
                                        context_ty.display(&fun_env.get_type_display_ctx())
                                    ),
                                ],
                            );
                        }

                        // All captured arguments must (a) have least the required abilities
                        // (b) must not be references
                        let wrapper_msg = || {
                            if let Some(ty) = &function_wrapper_ty {
                                format!(
                                    " (wrapped type of `{}`)",
                                    ty.display(&fun_env.get_type_display_ctx())
                                )
                            } else {
                                "".to_owned()
                            }
                        };
                        for captured in args {
                            let captured_ty = env.get_node_type(captured.node_id());
                            if captured_ty.is_reference() && !ref_capture_allowed.contains(id) {
                                // In verify mode, reference captures are admitted for
                                // closures passed directly to retained inline-opaque
                                // functions: the bytecode is never executed but only
                                // translated for the prover (the VM rejects them in
                                // regular compilation), and the prover models the
                                // captured locations at those call sites.
                                let mut msg = format!(
                                    "captured value cannot be a reference, but has type `{}`{}",
                                    captured_ty.display(&fun_env.get_type_display_ctx()),
                                    wrapper_msg()
                                );
                                if env.is_verify_mode() {
                                    msg += "; in verification, lambdas over references are \
                                            only supported as direct arguments of calls to \
                                            inline functions with `pragma opaque`";
                                }
                                env.error(&env.get_node_loc(captured.node_id()), &msg)
                            }
                            let arg_ty_abilities =
                                env.type_abilities(&captured_ty, fun_env.get_type_parameters_ref());
                            let missing = required_abilities.setminus(arg_ty_abilities);
                            if !missing.is_empty() {
                                env.error_with_notes(
                                    &env.get_node_loc(captured.node_id()),
                                    &format!("captured value is missing abilities `{}`", missing,),
                                    vec![format!(
                                        "expected function type: `{}`{}",
                                        context_ty.display(&fun_env.get_type_display_ctx()),
                                        wrapper_msg()
                                    )],
                                )
                            }
                        }

                        // (d) Scripts cannot have closures with lambda lifted functions.
                        if is_script_module && is_lambda_lifted {
                            env.error_with_notes(
                                &env.get_node_loc(*id),
                                "lambda lifting is not allowed in scripts",
                                vec!["lambda cannot be reduced to partial application of an existing function".to_string()],
                            );
                        }
                    }

                    // Continue visiting
                    true
                });
            }
        }
    }
}

/// The root variable of a reference view in the enclosing function.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RootVar {
    Local(Symbol),
    Param(TempIndex),
}

/// Checks that no location is mutated through more than one argument of a call to a
/// retained inline-opaque function, and that a mutated location is not also referenced
/// or captured by another argument. Such programs can be legal Move — after inline
/// expansion the accesses happen sequentially, and compilation including reference
/// safety is assumed to have succeeded — but they cannot be modeled by the callee's
/// opaque spec:
/// - behavioral predicates relate the call's entry and exit states, while the actual
///   effects compose through intermediate states, so the prover's call-site model
///   (one pre-state value and one havoced post-state value per location) would be
///   unsound;
/// - value and reference captures snapshot the location when the closure is
///   constructed, while inline expansion reads the variable at its use sites, which
///   may observe a sibling argument's mutation.
///
/// Views are collected from direct `&`/`&mut` arguments, from the operands of closure
/// arguments (borrows as produced by lambda lifting for modified captures, captured
/// values, and the variables used in curried capture expressions), and from
/// reference-typed locals with a statically visible borrow binding (`let r = &x;`).
/// Direct non-reference arguments need no tracking (they are evaluated at call entry
/// under both closure and expansion semantics), and neither do reference-typed
/// parameters of the enclosing function (they cannot refer to its own locals).
fn check_retained_call_view_conflicts(env: &GlobalEnv, fun_env: &FunctionEnv, def: &Exp) {
    // Track simple reference bindings `let r = <borrow of root>` in a single
    // program-order pass, so each retained call below is checked against the
    // bindings in effect at that point. Bindings are resolved against the
    // carriers collected so far, so chains like `let rs = &s; let rx = &rs.x;`
    // resolve to the root `s`; reassignments and rebindings with a different
    // view conservatively invalidate. (Branch-local invalidations leak to
    // join points, which can only cause missed views, never false errors;
    // carrier-based conflicts are additionally rejected by reference safety,
    // since a carrier binding always lives across the sibling closure pack.)
    let mut carriers: BTreeMap<Symbol, Option<(ReferenceKind, RootVar)>> = BTreeMap::new();
    def.visit_pre_order(&mut |e| {
        match e {
            ExpData::Block(_, Pattern::Var(_, sym), Some(binding), _) => {
                let view = root_view(env, &carriers, binding.as_ref());
                let entry = carriers.entry(*sym).or_insert(view);
                if *entry != view {
                    *entry = None;
                }
            },
            ExpData::Assign(_, pat, _) => {
                for (_, sym) in pat.vars() {
                    carriers.insert(sym, None);
                }
            },
            _ => {},
        }
        if let ExpData::Call(id, Operation::MoveFunction(mid, fid), args) = e {
            if env
                .get_function(mid.qualified(*fid))
                .is_inline_opaque_retained()
            {
                let mut views: BTreeMap<RootVar, Vec<ReferenceKind>> = BTreeMap::new();
                let mut has_ref_captures = false;
                for arg in args {
                    if let ExpData::Call(_, Operation::Closure(..), operands) = arg.as_ref() {
                        for op in operands {
                            if env.get_node_type(op.node_id()).is_reference() {
                                has_ref_captures = true;
                            }
                            // Within captured operands, all variable uses are views:
                            // borrows by their kind, values (captured directly or used
                            // in a curried capture expression) as immutable snapshots.
                            op.visit_pre_order(&mut |sub| match sub {
                                ExpData::Call(_, Operation::Borrow(_), _) => {
                                    if let Some((kind, root)) = root_view(env, &carriers, sub) {
                                        views.entry(root).or_default().push(kind);
                                    }
                                    false // root accounted for, do not descend
                                },
                                ExpData::LocalVar(id, sym) => {
                                    if env.get_node_type(*id).is_reference() {
                                        if let Some(Some((kind, root))) = carriers.get(sym) {
                                            views.entry(*root).or_default().push(*kind);
                                        }
                                    } else {
                                        views
                                            .entry(RootVar::Local(*sym))
                                            .or_default()
                                            .push(ReferenceKind::Immutable);
                                    }
                                    true
                                },
                                ExpData::Temporary(id, idx) => {
                                    if !env.get_node_type(*id).is_reference() {
                                        views
                                            .entry(RootVar::Param(*idx))
                                            .or_default()
                                            .push(ReferenceKind::Immutable);
                                    }
                                    true
                                },
                                _ => true,
                            });
                        }
                    } else if let Some((kind, root)) = root_view(env, &carriers, arg.as_ref()) {
                        views.entry(root).or_default().push(kind);
                    } else if env.get_node_type(arg.node_id()).is_reference() {
                        // A reference-typed argument computed by a call (e.g.
                        // `vector::borrow_mut(&mut v, i)`, table/map helpers)
                        // derives from the references used inside it; collect
                        // their root views. Value subexpressions are evaluated
                        // at call entry and need no views.
                        arg.visit_pre_order(&mut |sub| match sub {
                            ExpData::Call(_, Operation::Borrow(_), _) => {
                                if let Some((kind, root)) = root_view(env, &carriers, sub) {
                                    views.entry(root).or_default().push(kind);
                                }
                                false
                            },
                            ExpData::LocalVar(id, sym) if env.get_node_type(*id).is_reference() => {
                                if let Some(Some((kind, root))) = carriers.get(sym) {
                                    views.entry(*root).or_default().push(*kind);
                                }
                                true
                            },
                            _ => true,
                        });
                    }
                }
                for (root, kinds) in views {
                    let mut_count = kinds
                        .iter()
                        .filter(|k| **k == ReferenceKind::Mutable)
                        .count();
                    if mut_count >= 1 && kinds.len() >= 2 {
                        let name = match root {
                            RootVar::Local(sym) => {
                                format!("local `{}`", sym.display(env.symbol_pool()))
                            },
                            RootVar::Param(idx) => format!(
                                "parameter `{}`",
                                fun_env.get_local_name(idx).display(env.symbol_pool())
                            ),
                        };
                        let msg = if mut_count == kinds.len() {
                            format!(
                                "{} is mutated through more than one argument of this call",
                                name
                            )
                        } else {
                            format!(
                                "{} is mutated through one argument of this call and \
                                 referenced or captured through another",
                                name
                            )
                        };
                        env.error_with_notes(&env.get_node_loc(*id), &msg, vec![
                            "the behavioral predicates in the callee's opaque spec relate \
                                 the call's entry and exit states and cannot express sequential \
                                 effects on the same location; combine the accesses in one lambda"
                                .to_string(),
                        ]);
                    }
                }

                // A closure capturing references must not outlive the call: the
                // captured locations are modeled at this call site only, and the
                // Boogie dispatcher treats invocations of such closures elsewhere
                // as unreachable. The callee cannot store it globally (reference
                // captures lack the `store` ability), but it could leak it through
                // its result or a `&mut` parameter; reject those channels.
                if has_ref_captures {
                    let callee = env.get_function(mid.qualified(*fid));
                    let targs = env.get_node_instantiation(*id);
                    let mut visited = BTreeSet::new();
                    let mut leaks = type_contains_function(
                        env,
                        &callee.get_result_type().instantiate(&targs),
                        &mut visited,
                    );
                    if !leaks {
                        for ty in callee.get_parameter_types() {
                            if let Type::Reference(ReferenceKind::Mutable, target) = &ty {
                                visited.clear();
                                if type_contains_function(
                                    env,
                                    &target.instantiate(&targs),
                                    &mut visited,
                                ) {
                                    leaks = true;
                                    break;
                                }
                            }
                        }
                    }
                    if leaks {
                        env.error_with_notes(
                            &env.get_node_loc(*id),
                            "a closure capturing references cannot be passed to this call: \
                             the callee may leak function values through its result or a \
                             `&mut` parameter",
                            vec![
                                "such a closure must not outlive the call, since its captured \
                                 locations are only modeled at this call site"
                                    .to_string(),
                            ],
                        );
                    }
                }
            }
        }
        true
    });
}

/// Returns true if the type transitively contains a function type, expanding struct
/// fields (`visited` guards against recursive struct definitions).
fn type_contains_function(
    env: &GlobalEnv,
    ty: &Type,
    visited: &mut BTreeSet<QualifiedInstId<StructId>>,
) -> bool {
    match ty {
        Type::Fun(..) => true,
        Type::Reference(_, t) | Type::Vector(t) => type_contains_function(env, t, visited),
        Type::Tuple(ts) => ts.iter().any(|t| type_contains_function(env, t, visited)),
        Type::Struct(mid, sid, inst) => {
            // Visited keys include the instantiation: different instances of a
            // generic wrapper contain different field types.
            if !visited.insert(mid.qualified_inst(*sid, inst.clone())) {
                return false;
            }
            let struct_env = env.get_struct(mid.qualified(*sid));
            struct_env.get_fields().any(|field| {
                type_contains_function(env, &field.get_type().instantiate(inst), visited)
            })
        },
        _ => false,
    }
}

/// Resolves the view of a reference-creating or reference-carrying expression: for a
/// borrow, the borrow kind and the root variable of its selection chain (resolving
/// reference-typed roots through `carriers`); for a reference-typed local, the view of
/// its carrier binding. Returns None for reference-typed parameters of the enclosing
/// function (which cannot refer to its own locals) and for unresolvable shapes.
fn root_view(
    env: &GlobalEnv,
    carriers: &BTreeMap<Symbol, Option<(ReferenceKind, RootVar)>>,
    e: &ExpData,
) -> Option<(ReferenceKind, RootVar)> {
    match e {
        ExpData::Call(_, Operation::Borrow(kind), args) => {
            match args[0].as_ref().selection_chain_root(true) {
                ExpData::LocalVar(id, sym) => {
                    if env.get_node_type(*id).is_reference() {
                        // Borrow through a reference carrier, e.g. `&mut (*r).x`:
                        // the access kind is that of the new borrow.
                        carriers
                            .get(sym)
                            .copied()
                            .flatten()
                            .map(|(_, root)| (*kind, root))
                    } else {
                        Some((*kind, RootVar::Local(*sym)))
                    }
                },
                ExpData::Temporary(id, idx) => {
                    if env.get_node_type(*id).is_reference() {
                        None
                    } else {
                        Some((*kind, RootVar::Param(*idx)))
                    }
                },
                _ => None,
            }
        },
        ExpData::LocalVar(id, sym) if env.get_node_type(*id).is_reference() => {
            carriers.get(sym).copied().flatten()
        },
        _ => None,
    }
}
