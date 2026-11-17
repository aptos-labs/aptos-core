// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Questions about types: what a value of a type can contain, and whether a
//! type hides what is behind it.

use super::cells::Cells;
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_middle::ty::{self, Ty, TyCtxt, TypingEnv};

pub(super) fn type_of<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId) -> Ty<'tcx> {
    tcx.type_of(def_id)
        .instantiate_identity()
        .skip_normalization()
}

/// Visits `ty` and everything a value of it can contain, stopping at the
/// first `Some` from `f`. Does not enter a cell struct of `cells`. With
/// `owned_only`, does not follow references and raw pointers.
pub(super) fn find_in_ty<'tcx, R>(
    tcx: TyCtxt<'tcx>,
    cells: &Cells<'tcx>,
    owned_only: bool,
    ty: Ty<'tcx>,
    mut f: impl FnMut(Ty<'tcx>) -> Option<R>,
) -> Option<R> {
    let mut seen: Vec<Ty<'tcx>> = Vec::new();
    let mut stack = vec![ty];
    while let Some(ty) = stack.pop() {
        if seen.contains(&ty) {
            continue;
        }
        seen.push(ty);
        if let Some(found) = f(ty) {
            return Some(found);
        }
        match ty.kind() {
            ty::Ref(_, inner, _) | ty::RawPtr(inner, _) if !owned_only => stack.push(*inner),
            ty::Array(inner, _) | ty::Slice(inner) => stack.push(*inner),
            ty::Adt(adt, args) => {
                stack.extend(args.types());
                if !cells.is_struct(ty) {
                    stack.extend(adt.all_fields().map(|field| {
                        // Resolve an associated type (`F::Out`) where `F` is
                        // known. One left unresolved counts as opaque.
                        let field_ty = field.ty(tcx, args);
                        tcx.try_normalize_erasing_regions(
                            TypingEnv::fully_monomorphized(),
                            field_ty,
                        )
                        .unwrap_or(field_ty.skip_normalization())
                    }));
                }
            },
            ty::Closure(_, args) => stack.extend(args.as_closure().upvar_tys().iter()),
            ty::Tuple(tys) => stack.extend(tys.iter()),
            _ => {},
        }
    }
    None
}

/// What `ty` is, if it is a trait object, an opaque type or an associated
/// type that is not resolved.
pub(super) fn erased_kind<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<&'static str> {
    match ty.kind() {
        ty::Dynamic(..) => Some("a trait object"),
        ty::Alias(_, alias) if matches!(alias.kind(tcx), ty::AliasTyKind::Opaque { .. }) => {
            Some("an opaque type")
        },
        ty::Alias(..) => Some("an associated type that is not resolved"),
        _ => None,
    }
}

/// Whether `ty` mentions a type the lint cannot see through: a type
/// parameter, a trait object or an opaque type.
pub(super) fn is_erased<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    ty.walk().any(|arg| {
        arg.as_type()
            .is_some_and(|ty| matches!(ty.kind(), ty::Param(_)) || erased_kind(tcx, ty).is_some())
    })
}

/// Whether `owner` has a bound that mentions the type parameter `param`
/// (`T: Trait`, `&T: Trait`, ...) on a trait that satisfies `f`.
pub(super) fn has_bound<'tcx>(
    tcx: TyCtxt<'tcx>,
    owner: LocalDefId,
    param: Ty<'tcx>,
    f: impl Fn(DefId) -> bool,
) -> bool {
    let bounds = tcx.param_env(owner).caller_bounds();
    bounds.iter().any(|clause| {
        clause.as_trait_clause().is_some_and(|pred| {
            let mut bounded = pred.self_ty().skip_binder().walk();
            bounded.any(|arg| arg.as_type() == Some(param)) && f(pred.def_id())
        })
    })
}
