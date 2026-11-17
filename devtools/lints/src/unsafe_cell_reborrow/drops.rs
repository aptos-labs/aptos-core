// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Destructors: which bodies own a value whose destructor reaches a cell
//! struct. A body that holds a cell must not; any other body counts as
//! possibly borrowing every cell.

use super::{body::BodyScan, types::find_in_ty, util::report};
use clippy_utils::visitors::for_each_expr_without_closures;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::{self as hir, HirId};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;
use std::ops::ControlFlow;

impl<'tcx> BodyScan<'_, 'tcx> {
    /// Whether the body owns a value whose `Drop` impl reaches a cell
    /// struct, going by the type of every parameter and expression. A
    /// destructor is a call the body never spells.
    ///
    /// Check: a body that holds a cell owns no such value.
    pub(super) fn check_drops(&self, body: &'tcx hir::Body<'tcx>) -> bool {
        let tcx = self.cx.tcx;
        let holds = !self.info.sites.is_empty();
        let mut seen: FxHashMap<Ty<'tcx>, Option<Ty<'tcx>>> = FxHashMap::default();
        let mut reported: Vec<Ty<'tcx>> = Vec::new();
        let mut check = |hir_id: HirId, span: Span, ty: Ty<'tcx>| {
            let dropped = *seen
                .entry(ty)
                .or_insert_with(|| self.drop_reaching_struct(ty));
            if let Some(dropped) = dropped
                && !reported.contains(&dropped)
            {
                reported.push(dropped);
                if holds {
                    report(
                        self.cx,
                        hir_id,
                        span,
                        format!(
                            "a value of `{dropped}`, whose `Drop` can reach a cell struct, is \
                             owned here while {} is held",
                            self.cells.names(tcx, &self.info.sites)
                        ),
                        &[],
                    );
                }
            }
        };
        for param in body.params {
            check(param.hir_id, param.span, self.typeck.pat_ty(param.pat));
        }
        for_each_expr_without_closures(body, |expr| {
            check(expr.hir_id, expr.span, self.typeck.expr_ty(expr));
            ControlFlow::<()>::Continue(())
        });
        !reported.is_empty()
    }

    /// A type that a value of `ty` owns, that has a `Drop` impl and that
    /// reaches a cell struct. For a type of another crate, whose fields are
    /// not visible here, that means one of its type arguments does, as in
    /// `Guard<Callback>` or `Vec<&Ctx>`.
    pub(super) fn drop_reaching_struct(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let tcx = self.cx.tcx;
        find_in_ty(tcx, self.cells, true, ty, |ty| {
            matches!(ty.kind(), ty::Adt(adt, _)
                if tcx.adt_destructor(adt.did()).is_some() && self.cells.reaches(tcx, ty))
            .then_some(ty)
        })
    }
}
