// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The cells: the `UnsafeCell` fields of the structs listed in the
//! configuration. Finding them is the first pass.

use super::{
    types::{find_in_ty, type_of},
    util::{full_path, report, report_config},
    Config,
};
use rustc_data_structures::fx::{FxHashMap, FxIndexSet};
use rustc_hir::{
    self as hir,
    attrs::lang_items::LangItem,
    def_id::{DefId, CRATE_DEF_ID, LOCAL_CRATE},
};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::{Span, Symbol};
use std::cell::RefCell;

/// One `UnsafeCell` field of a listed struct.
pub(super) struct Cell {
    /// The struct that owns the field.
    pub(super) owner: DefId,
    pub(super) field: Symbol,
    /// Derefs between `.get()` and the reborrow: two through a stored
    /// reference (`&mut **`), one for an owned value (`&mut *`).
    pub(super) derefs: usize,
}

pub(super) type CellIdx = usize;

/// The cells of the crate, and which types reach the structs that own them.
pub(super) struct Cells<'tcx> {
    pub(super) list: Vec<Cell>,
    /// Memo for [`Cells::reaches`].
    pub(super) reach: RefCell<FxHashMap<Ty<'tcx>, bool>>,
    /// Types too large to walk, to be reported once each. See
    /// [`find_in_ty`].
    pub(super) too_large: RefCell<FxIndexSet<Ty<'tcx>>>,
}

impl<'tcx> Cells<'tcx> {
    pub(super) fn name(&self, tcx: TyCtxt<'_>, cell: CellIdx) -> String {
        let cell = &self.list[cell];
        format!("`{}.{}`", tcx.item_name(cell.owner), cell.field)
    }

    /// The cell that field `field` of the struct `owner` is, if any.
    pub(super) fn index_of(&self, owner: DefId, field: Symbol) -> Option<CellIdx> {
        self.list
            .iter()
            .position(|c| c.owner == owner && c.field == field)
    }

    /// The names of the cells of `sites`, for a message.
    pub(super) fn names(&self, tcx: TyCtxt<'_>, sites: &[(CellIdx, Span)]) -> String {
        sites
            .iter()
            .map(|(cell, _)| self.name(tcx, *cell))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// `S`, `&S`, `&&S`, ... for a cell struct `S`.
    pub(super) fn is_struct(&self, ty: Ty<'_>) -> bool {
        matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if self.list.iter().any(|c| c.owner == adt.did()))
    }

    /// Whether a value of `ty` reaches a cell struct `S`: `S`, `&S`,
    /// `Option<&S>`, a closure capturing one, a local struct holding one.
    pub(super) fn reaches(&self, tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
        if let Some(known) = self.reach.borrow().get(&ty) {
            return *known;
        }
        // A type too large to walk is assumed to reach a cell struct.
        let reaches = find_in_ty(tcx, self, false, ty, |ty| self.is_struct(ty).then_some(()))
            .map_or(true, |found| found.is_some());
        self.reach.borrow_mut().insert(ty, reaches);
        reaches
    }
}

/// Collects the cells. Reports a `pub` cell field, and a listed struct that
/// does not exist or has no `UnsafeCell` field.
pub(super) fn find_cells<'tcx>(cx: &LateContext<'tcx>, config: &Config) -> Cells<'tcx> {
    let tcx = cx.tcx;
    let mut list = Vec::new();
    let mut found = Vec::new();
    for id in tcx.hir_crate_items(()).free_items() {
        let item = tcx.hir_item(id);
        let hir::ItemKind::Struct(_, _, data) = item.kind else {
            continue;
        };
        let owner = item.owner_id.to_def_id();
        let path = full_path(cx, owner);
        if !config.structs.contains(&path) {
            continue;
        }
        let before = list.len();
        for field in data.fields() {
            let field_ty = type_of(tcx, field.def_id.to_def_id());
            let is_unsafe_cell = |ty: Ty<'tcx>| matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::UnsafeCell));
            if !is_unsafe_cell(field_ty) {
                // The lint only understands a cell that is a field of type
                // `UnsafeCell<T>`. It cannot check any other form, such as an
                // array or an `Option` of cells, so it rejects them.
                if field_ty
                    .walk()
                    .any(|arg| arg.as_type().is_some_and(is_unsafe_cell))
                {
                    report(
                        cx,
                        field.hir_id,
                        field.span,
                        format!(
                            "field `{}.{}` has an `UnsafeCell` inside another type; this lint only \
                             understands a field of type `UnsafeCell<T>` and cannot check any \
                             other form",
                            tcx.item_name(owner),
                            field.ident
                        ),
                        &[],
                    );
                }
                continue;
            }
            let ty::Adt(_, args) = field_ty.kind() else {
                continue;
            };
            let content = args.type_at(0);
            let derefs = if content.is_ref() { 2 } else { 1 };
            if tcx.visibility(field.def_id).is_public() {
                report(
                    cx,
                    field.hir_id,
                    field.span,
                    format!(
                        "cell field `{}.{}` is `pub`; cell fields must stay private to the crate",
                        tcx.item_name(owner),
                        field.ident
                    ),
                    &[],
                );
            }
            list.push(Cell {
                owner,
                field: field.ident.name,
                derefs,
            });
        }
        if list.len() == before {
            report_config(
                cx,
                item.span,
                format!(
                    "`{path}` is listed for `unsafe_cell_reborrow` but has no `UnsafeCell` field"
                ),
            );
        }
        found.push(path);
    }
    let krate = tcx.crate_name(LOCAL_CRATE);
    for listed in &config.structs {
        if listed.split("::").next() == Some(krate.as_str()) && !found.contains(listed) {
            report_config(
                cx,
                tcx.def_span(CRATE_DEF_ID).shrink_to_lo(),
                format!("`{listed}` is listed for `unsafe_cell_reborrow` but is not a struct of this crate"),
            );
        }
    }
    Cells {
        list,
        reach: RefCell::default(),
        too_large: RefCell::default(),
    }
}
