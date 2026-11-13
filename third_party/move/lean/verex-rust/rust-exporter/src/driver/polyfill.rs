// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Temporary queries for rust-lang/rust#161892 on nightly-2026-07-23.
//!
//! This is the only mapper module allowed to use compiler internals. All values
//! crossing its boundary belong to rustc_public. Queries borrow rustc's scoped
//! implicit context during `run_with_tcx!`; no context or internal value escapes
//! the callback, and no additional TLS or lifetime erasure is needed here.

use rustc_middle::{mir, ty};
use rustc_public::{
    crate_def::DefId,
    rustc_internal::{internal, stable},
    ty::{ClosureDef, FnDef, GenericArgs, GenericDef, MirConst, PredicateKind, Span, Ty},
};

pub(super) const PRODUCER: &str =
    "verex-rust-export via rustc Public (+ rustc_internal polyfill for rust-lang/rust#161892)";

pub(super) fn source_callsite(span: Span) -> Span {
    ty::tls::with(|tcx| stable(internal(tcx, span).source_callsite()))
}

pub(super) fn const_param_ty(parameter: GenericDef) -> Ty {
    ty::tls::with(|tcx| {
        stable(
            tcx.type_of(internal(tcx, parameter.0))
                .instantiate_identity()
                .skip_norm_wip(),
        )
    })
}

pub(super) struct ItemPredicates {
    pub parent: Option<DefId>,
    // Keep binders: dropping a higher-ranked quantifier would change the bound.
    pub predicates: Vec<(rustc_public::ty::Binder<PredicateKind>, Span)>,
}

pub(super) fn explicit_predicates_of(definition: DefId) -> ItemPredicates {
    ty::tls::with(|tcx| {
        let predicates = tcx.explicit_predicates_of(internal(tcx, definition));
        ItemPredicates {
            parent: predicates.parent.map(stable),
            predicates: predicates
                .predicates
                .iter()
                .filter_map(|(clause, span)| {
                    // The pinned bridge cannot convert these compiler-only clauses.
                    if matches!(
                        clause.kind().skip_binder(),
                        ty::ClauseKind::HostEffect(..) | ty::ClauseKind::UnstableFeature(..)
                    ) {
                        None
                    } else {
                        Some((stable(clause.as_predicate().kind()), stable(*span)))
                    }
                })
                .collect(),
        }
    })
}

/// Compiler identities for language-defined operations. Keep identity
/// checks here rather than recognizing a user-controlled trait name.
pub(super) fn copy_trait() -> Option<DefId> {
    ty::tls::with(|tcx| tcx.lang_items().copy_trait().map(stable))
}

pub(super) fn drop_trait() -> Option<DefId> {
    ty::tls::with(|tcx| tcx.lang_items().drop_trait().map(stable))
}

pub(super) fn drop_method() -> Option<DefId> {
    drop_trait().and_then(|id| rustc_public::ty::TraitDef(id).associated_items().into_iter()
        .find(|item| matches!(&item.kind, rustc_public::ty::AssocKind::Fn { name, .. } if name == "drop"))
        .map(|item| item.def_id.0))
}

pub(super) fn is_process_abort(definition: FnDef) -> bool {
    ty::tls::with(|tcx| {
        let id = internal(tcx, definition.0);
        tcx.diagnostic_items(id.krate)
            .name_to_id
            .iter()
            .any(|(name, item)| name.as_str() == "process_abort" && *item == id)
    })
}

pub(super) fn fn_trait() -> Option<rustc_public::ty::TraitDef> {
    ty::tls::with(|tcx| {
        tcx.lang_items()
            .fn_trait()
            .map(|id| rustc_public::ty::TraitDef(stable(id)))
    })
}

pub(super) fn fn_mut_trait() -> Option<rustc_public::ty::TraitDef> {
    ty::tls::with(|tcx| {
        tcx.lang_items()
            .fn_mut_trait()
            .map(|id| rustc_public::ty::TraitDef(stable(id)))
    })
}

pub(super) fn fn_once_trait() -> Option<rustc_public::ty::TraitDef> {
    ty::tls::with(|tcx| {
        tcx.lang_items()
            .fn_once_trait()
            .map(|id| rustc_public::ty::TraitDef(stable(id)))
    })
}

pub(super) fn fn_once_output() -> Option<DefId> {
    ty::tls::with(|tcx| tcx.lang_items().fn_once_output().map(stable))
}

/// What rustc normalizes a type naming an associated type to in `owner`'s
/// environment: the type the selected impl gives it, where a bound of the
/// environment determines the impl. A projection standing on a type parameter
/// rustc cannot normalize, and a type naming no associated type is already the
/// type rustc gave us, with its regions: both are returned as they are.
pub(super) fn normalize_type(owner: DefId, value: Ty) -> Ty {
    ty::tls::with(|tcx| {
        let environment = ty::TypingEnv::non_body_analysis(tcx, internal(tcx, owner));
        let internal = internal(tcx, value);
        match tcx.try_normalize_erasing_regions(environment, ty::Unnormalized::new(internal)) {
            Ok(normalized) => stable(normalized),
            Err(_) => value,
        }
    })
}

/// Substitute a declared evidence signature at a checked Rust call site.
pub(super) fn instantiate_type(value: Ty, arguments: &GenericArgs) -> Ty {
    ty::tls::with(|tcx| {
        stable(
            ty::EarlyBinder::bind(tcx, internal(tcx, value))
                .instantiate(tcx, internal(tcx, arguments.clone()))
                .skip_norm_wip(),
        )
    })
}

pub(super) enum ResolvedCallee {
    Item(FnDef, GenericArgs),
    Closure(ClosureDef, GenericArgs),
    /// Retained in public form for M2 dictionary selection.
    TraitParam {
        trait_method: FnDef,
        args: GenericArgs,
    },
    Unsupported(&'static str),
}

pub(super) fn resolve_in_caller(
    caller: DefId,
    callee: FnDef,
    arguments: &GenericArgs,
) -> Result<ResolvedCallee, String> {
    ty::tls::with(|tcx| {
        let environment = ty::TypingEnv::non_body_analysis(tcx, internal(tcx, caller));
        let arguments = tcx
            .try_normalize_erasing_regions(
                environment,
                ty::Unnormalized::new(internal(tcx, arguments.clone())),
            )
            .map_err(|error| format!("cannot normalize callee arguments: {error:?}"))?;
        let instance =
            ty::Instance::try_resolve(tcx, environment, internal(tcx, callee.0), arguments)
                .map_err(|error| format!("cannot resolve callee: {error:?}"))?;
        // On this pin a parameter-dependent selection is `None` (there is no
        // InstanceKind::Param). Do not turn it into an unchecked direct call.
        Ok(match instance {
            None => ResolvedCallee::TraitParam {
                trait_method: callee,
                args: GenericArgs(
                    arguments
                        .iter()
                        .map(|argument| stable(argument.kind()))
                        .collect(),
                ),
            },
            Some(instance) => match instance.def {
                ty::InstanceKind::Item(definition) => {
                    // Converting the instance itself assumes monomorphic types.
                    let args = GenericArgs(
                        instance
                            .args
                            .iter()
                            .map(|argument| stable(argument.kind()))
                            .collect(),
                    );
                    let def = stable(definition);
                    if tcx
                        .type_of(definition)
                        .instantiate_identity()
                        .skip_norm_wip()
                        .is_closure()
                    {
                        ResolvedCallee::Closure(ClosureDef(def), args)
                    } else {
                        ResolvedCallee::Item(FnDef(def), args)
                    }
                },
                ty::InstanceKind::Virtual(..) => {
                    ResolvedCallee::Unsupported("is dispatched dynamically")
                },
                ty::InstanceKind::Intrinsic(..) | ty::InstanceKind::LlvmIntrinsic(..) => {
                    ResolvedCallee::Unsupported("is a compiler intrinsic")
                },
                ty::InstanceKind::Shim(..) => {
                    ResolvedCallee::Unsupported("reaches compiler-generated glue")
                },
            },
        })
    })
}

pub(super) fn eval_promoted_const(constant: &MirConst, span: Span) -> Result<MirConst, String> {
    ty::tls::with(|tcx| {
        let mir::Const::Unevaluated(unevaluated, type_) = internal(tcx, constant.clone()) else {
            return Err("expected an unevaluated promoted constant".to_owned());
        };
        let environment = ty::TypingEnv::non_body_analysis(tcx, unevaluated.def);
        let value = tcx
            .const_eval_resolve(environment, unevaluated, internal(tcx, span))
            .map_err(|error| format!("cannot evaluate promoted constant: {error:?}"))?;
        Ok(stable(mir::Const::Val(value, type_)))
    })
}

/// Read the signature without substituting the erased lifetimes of `FnDef::ty`.
/// The public fn_sig query otherwise loses precisely the early-bound regions
/// referenced by explicit outlives clauses.
pub(super) fn declared_signature(function: FnDef) -> rustc_public::ty::PolyFnSig {
    ty::tls::with(|tcx| {
        stable(
            tcx.fn_sig(internal(tcx, function.0))
                .instantiate_identity()
                .skip_norm_wip(),
        )
    })
}

/// Includes inherited and supertrait Copy bounds, even when unused in the body.
pub(super) fn copy_parameters(definition: DefId) -> Vec<u32> {
    ty::tls::with(|tcx| {
        let definition = internal(tcx, definition);
        let environment = ty::TypingEnv::non_body_analysis(tcx, definition);
        ty::GenericArgs::identity_for_item(tcx, definition)
            .iter()
            .enumerate()
            .filter_map(|(index, arg)| {
                let ty::GenericArgKind::Type(type_) = arg.kind() else {
                    return None;
                };
                // Closure synthetic signature/upvar parameters are not source binders.
                if matches!(type_.kind(), ty::Param(_))
                    && tcx.type_is_copy_modulo_regions(environment, type_)
                {
                    Some(index as u32)
                } else {
                    None
                }
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_public::{
        crate_def::CrateDef,
        mir::{visit::Location, ConstOperand, MirVisitor, TerminatorKind},
        ty::{ClauseKind, ConstantKind, GenericArgKind, RigidTy, TyKind},
        CompilerError,
    };

    #[test]
    fn preserves_queries_needed_by_trait_evidence() {
        let source = format!(
            "{}/tests/fixtures/polyfill_api.rs",
            env!("CARGO_MANIFEST_DIR")
        );
        let arguments = vec![
            "rustc".to_owned(),
            source.clone(),
            "--crate-type=lib".to_owned(),
            "--edition=2024".to_owned(),
            "-Cpanic=abort".to_owned(),
        ];
        let result = rustc_public::run_with_tcx!(&arguments, |_tcx| {
            let krate = rustc_public::local_crate();
            let function = |name: &str| {
                krate
                    .fn_defs()
                    .into_iter()
                    .find(|f| f.name().ends_with(name))
                    .expect("fixture function")
            };
            let predicates = explicit_predicates_of(function("::predicates").0);
            assert!(predicates.predicates.iter().any(|(p, _)| matches!(
                p.value,
                rustc_public::ty::PredicateKind::Clause(ClauseKind::Trait(_))
            )));
            assert!(predicates.predicates.iter().any(|(p, _)| matches!(
                p.value,
                rustc_public::ty::PredicateKind::Clause(ClauseKind::Projection(_))
            )));
            assert!(predicates.predicates.iter().any(|(p, _)| matches!(
                p.value,
                rustc_public::ty::PredicateKind::Clause(ClauseKind::RegionOutlives(_))
            )));
            let higher = explicit_predicates_of(function("::higher_ranked").0);
            assert!(
                higher
                    .predicates
                    .iter()
                    .any(|(p, _)| !p.bound_vars.is_empty()),
                "higher-ranked binders must survive conversion"
            );
            let method = function("::method");
            let parent = explicit_predicates_of(method.0)
                .parent
                .expect("impl parent");
            assert!(explicit_predicates_of(parent)
                .predicates
                .iter()
                .any(|(p, _)| matches!(
                    p.value,
                    rustc_public::ty::PredicateKind::Clause(ClauseKind::Projection(_))
                )));
            let ordered = krate
                .adts()
                .into_iter()
                .find(|a| a.name().ends_with("::Ordered"))
                .unwrap();
            assert!(explicit_predicates_of(ordered.0)
                .predicates
                .iter()
                .any(|(p, _)| matches!(
                    p.value,
                    rustc_public::ty::PredicateKind::Clause(ClauseKind::RegionOutlives(_))
                )));
            let parameter = function("::constant").generics_of().params.remove(0);
            assert!(matches!(
                const_param_ty(parameter.def_id).kind(),
                TyKind::RigidTy(RigidTy::Uint(rustc_public::ty::UintTy::Usize))
            ));
            let body = rustc_public::CrateItem(method.0).body().unwrap();
            let (callee, args) = body
                .blocks
                .iter()
                .find_map(|block| match &block.terminator.kind {
                    TerminatorKind::Call { func, .. } => {
                        match func.ty(body.locals()).unwrap().kind() {
                            TyKind::RigidTy(RigidTy::FnDef(f, args)) => Some((f, args)),
                            _ => None,
                        }
                    },
                    _ => None,
                })
                .unwrap();
            match resolve_in_caller(method.0, callee, &args).unwrap() {
                ResolvedCallee::TraitParam { trait_method, args } => {
                    assert_eq!(trait_method, callee);
                    assert!(matches!(args.0[0], GenericArgKind::Type(_)));
                },
                _ => panic!("parameter-dependent selection must retain its method and arguments"),
            }
            let iterate = function("::iterate");
            let body = rustc_public::CrateItem(iterate.0).body().unwrap();
            let mut selected_next = false;
            for block in &body.blocks {
                if let TerminatorKind::Call { func, .. } = &block.terminator.kind
                    && let TyKind::RigidTy(RigidTy::FnDef(f, args)) =
                        func.ty(body.locals()).unwrap().kind()
                    && f.name().ends_with("::next")
                {
                    assert!(matches!(
                        resolve_in_caller(iterate.0, f, &args).unwrap(),
                        ResolvedCallee::Item(..)
                    ));
                    selected_next = true;
                }
            }
            assert!(selected_next);
            struct Promoted(bool);
            impl MirVisitor for Promoted {
                fn visit_const_operand(&mut self, constant: &ConstOperand, _: Location) {
                    if matches!(
                        constant.const_.kind(),
                        ConstantKind::Unevaluated(rustc_public::ty::UnevaluatedConst {
                            promoted: Some(_),
                            ..
                        })
                    ) {
                        let evaluated =
                            eval_promoted_const(&constant.const_, constant.span).unwrap();
                        assert!(matches!(evaluated.kind(), ConstantKind::Allocated(_)));
                        self.0 = true;
                    }
                }
            }
            let mut promoted = Promoted(false);
            promoted.visit_body(
                &rustc_public::CrateItem(function("::promoted").0)
                    .body()
                    .unwrap(),
            );
            assert!(promoted.0);
            let body = rustc_public::CrateItem(function("::macro_site").0)
                .body()
                .unwrap();
            let expanded = body
                .blocks
                .iter()
                .flat_map(|b| {
                    b.statements
                        .iter()
                        .map(|s| s.source_info.span)
                        .chain(std::iter::once(b.terminator.source_info.span))
                })
                .find(|s| s.get_filename() != source)
                .expect("external macro span");
            assert_eq!(source_callsite(expanded).get_filename(), source);
            std::ops::ControlFlow::<(), ()>::Break(())
        });
        assert!(matches!(result, Err(CompilerError::Interrupted(()))));
    }
}
