// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Questions about types: what a value of a type can contain, and whether a
//! type hides what is behind it.

use super::cells::Cells;
use rustc_data_structures::fx::FxHashSet;
use rustc_hir::def_id::DefId;
use rustc_middle::ty::{self, Ty, TyCtxt, TypingEnv};

pub(super) fn type_of<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId) -> Ty<'tcx> {
    tcx.type_of(def_id)
        .instantiate_identity()
        .skip_normalization()
}

/// How far one walk of a type goes before the lint gives up on it. A type
/// that nests itself with a new argument at each level
/// (`struct Node<T> { next: *const Node<(T,)> }`) never ends otherwise.
const MAX_DEPTH: usize = 64;
const MAX_TYPES: usize = 1000;

/// The walk of a type hit [`MAX_DEPTH`] or [`MAX_TYPES`]. The caller must
/// assume the worst about the type.
pub(super) struct TooLarge;

/// Visits `ty` and everything a value of it can contain, stopping at the
/// first `Some` from `f`. Does not enter a cell struct of `cells`. With
/// `owned_only`, does not follow references and raw pointers.
///
/// Gives up with [`TooLarge`] on a type past the limits above, and records
/// it in `cells` to be reported.
pub(super) fn find_in_ty<'tcx, R>(
    tcx: TyCtxt<'tcx>,
    cells: &Cells<'tcx>,
    owned_only: bool,
    ty: Ty<'tcx>,
    mut f: impl FnMut(Ty<'tcx>) -> Option<R>,
) -> Result<Option<R>, TooLarge> {
    let root = ty;
    let mut seen: FxHashSet<Ty<'tcx>> = FxHashSet::default();
    let mut stack = vec![(ty, 0)];
    while let Some((ty, depth)) = stack.pop() {
        if !seen.insert(ty) {
            continue;
        }
        if depth > MAX_DEPTH || seen.len() > MAX_TYPES {
            cells.too_large.borrow_mut().insert(root);
            return Err(TooLarge);
        }
        if let Some(found) = f(ty) {
            return Ok(Some(found));
        }
        let mut push = |inner: Ty<'tcx>| stack.push((inner, depth + 1));
        match ty.kind() {
            ty::Ref(_, inner, _) | ty::RawPtr(inner, _) if !owned_only => push(*inner),
            ty::Array(inner, _) | ty::Slice(inner) => push(*inner),
            ty::Adt(adt, args) => {
                args.types().for_each(&mut push);
                if !cells.is_struct(ty) {
                    for field in adt.all_fields() {
                        // Resolve an associated type (`F::Out`) where `F` is
                        // known. One left unresolved counts as opaque.
                        let field_ty = field.ty(tcx, args);
                        push(
                            tcx.try_normalize_erasing_regions(
                                TypingEnv::fully_monomorphized(),
                                field_ty,
                            )
                            .unwrap_or(field_ty.skip_normalization()),
                        );
                    }
                }
            },
            ty::Closure(_, args) => args.as_closure().upvar_tys().iter().for_each(&mut push),
            ty::Tuple(tys) => tys.iter().for_each(&mut push),
            _ => {},
        }
    }
    Ok(None)
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
