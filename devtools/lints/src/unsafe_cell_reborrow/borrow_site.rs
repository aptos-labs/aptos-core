// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Check: a cell field is used only in an anchored borrow, and is not bound
//! by a pattern.

use super::{
    body::BodyScan,
    util::{is_listed, report},
};
use clippy_utils::get_parent_expr;
use rustc_hir::{
    self as hir,
    def::{DefKind, Res},
    intravisit::{self, Visitor},
    pat_util::EnumerateAndAdjustIterator,
    BorrowKind, Expr, ExprKind, Mutability, Node, PatKind, UnOp,
};
use rustc_lint::LateContext;
use rustc_middle::ty;
use rustc_span::{Span, Symbol};

/// How a cell field is used.
pub(super) enum Site {
    /// Not in a borrow, for example `&self.cell` or `self.cell.get_mut()`.
    Misuse,
    /// In a borrow that is not passed to an anchor function.
    Bare(Span),
    /// In a borrow passed to an anchor function, whose anchor is or is not
    /// a local of the same body.
    Anchored { span: Span, local_anchor: bool },
}

/// The borrow `&mut <*…*><field>.get()` or `&<*…*><field>.get()`, with
/// `derefs` stars, that the cell field `field_expr` is used in, if any.
pub(super) fn reborrow_of<'tcx>(
    cx: &LateContext<'tcx>,
    field_expr: &Expr<'tcx>,
    derefs: usize,
) -> Option<&'tcx Expr<'tcx>> {
    let get = get_parent_expr(cx, field_expr)?;
    let ExprKind::MethodCall(segment, receiver, [], _) = get.kind else {
        return None;
    };
    if segment.ident.as_str() != "get" || receiver.hir_id != field_expr.hir_id {
        return None;
    }
    let mut place = get;
    for _ in 0..derefs {
        let deref = get_parent_expr(cx, place)?;
        let ExprKind::Unary(UnOp::Deref, inner) = deref.kind else {
            return None;
        };
        if inner.hir_id != place.hir_id {
            return None;
        }
        place = deref;
    }
    let borrow = get_parent_expr(cx, place)?;
    let ExprKind::AddrOf(BorrowKind::Ref, _, inner) = borrow.kind else {
        return None;
    };
    (inner.hir_id == place.hir_id).then_some(borrow)
}

impl<'tcx> BodyScan<'_, 'tcx> {
    /// Classifies a use of a cell field. The accepted form is
    /// `anchor_fn(&mut local, unsafe { <borrow> })`.
    pub(super) fn site(&self, field_expr: &Expr<'tcx>, derefs: usize) -> Site {
        let tcx = self.cx.tcx;
        let Some(reborrow) = reborrow_of(self.cx, field_expr, derefs) else {
            return Site::Misuse;
        };
        let span = reborrow.span;
        let mut outer = reborrow;
        loop {
            match tcx.parent_hir_node(outer.hir_id) {
                Node::Block(block) if block.expr.is_some_and(|e| e.hir_id == outer.hir_id) => {
                    let Node::Expr(block_expr) = tcx.parent_hir_node(block.hir_id) else {
                        return Site::Bare(span);
                    };
                    outer = block_expr;
                },
                Node::Expr(call) => {
                    let ExprKind::Call(callee, [anchor, borrow]) = call.kind else {
                        return Site::Bare(span);
                    };
                    let is_anchor_fn = matches!(callee.kind, ExprKind::Path(ref qpath)
                        if matches!(self.cx.qpath_res(qpath, callee.hir_id), Res::Def(DefKind::Fn, def_id)
                            if is_listed(self.cx, &self.config.anchor_fns, def_id)));
                    if !is_anchor_fn || borrow.hir_id != outer.hir_id {
                        return Site::Bare(span);
                    }
                    return Site::Anchored {
                        span,
                        local_anchor: self.is_local_anchor(anchor),
                    };
                },
                _ => return Site::Bare(span),
            }
        }
    }

    /// Whether `anchor` is `&mut x`, with `x` declared by a `let` of this
    /// body.
    pub(super) fn is_local_anchor(&self, anchor: &Expr<'tcx>) -> bool {
        let tcx = self.cx.tcx;
        let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Mut, place) = anchor.kind else {
            return false;
        };
        let ExprKind::Path(hir::QPath::Resolved(None, path)) = place.kind else {
            return false;
        };
        let Res::Local(local) = path.res else {
            return false;
        };
        matches!(tcx.parent_hir_node(local), Node::LetStmt(_))
            && tcx.hir_enclosing_body_owner(local) == self.owner
    }
}

/// Check: a cell field is not bound by a pattern, such as
/// `let Self { heap, .. } = self`.
pub(super) struct CellPatterns<'a, 'b, 'tcx> {
    pub(super) scan: &'a BodyScan<'b, 'tcx>,
}

impl<'tcx> Visitor<'tcx> for CellPatterns<'_, '_, 'tcx> {
    fn visit_pat(&mut self, pat: &'tcx hir::Pat<'tcx>) {
        let scan = self.scan;
        if let ty::Adt(adt, _) = scan.typeck.pat_ty(pat).peel_refs().kind() {
            // The fields the pattern names, with the pattern each is matched to.
            let fields: Vec<(Symbol, &hir::Pat<'tcx>)> = match pat.kind {
                PatKind::Struct(_, fields, _) => {
                    fields.iter().map(|f| (f.ident.name, f.pat)).collect()
                },
                PatKind::TupleStruct(_, pats, dotdot) if adt.is_struct() => pats
                    .iter()
                    .enumerate_and_adjust(adt.non_enum_variant().fields.len(), dotdot)
                    .map(|(index, pat)| (Symbol::intern(&index.to_string()), pat))
                    .collect(),
                _ => Vec::new(),
            };
            for (name, field_pat) in fields {
                if let Some(cell) = scan.cells.index_of(adt.did(), name)
                    && !matches!(field_pat.kind, PatKind::Wild)
                {
                    report(
                        scan.cx,
                        field_pat.hir_id,
                        field_pat.span,
                        format!(
                            "{} bound by a pattern; a cell field may only be named at a reborrow \
                             site",
                            scan.cells.name(scan.cx.tcx, cell)
                        ),
                        &[],
                    );
                }
            }
        }
        intravisit::walk_pat(self, pat);
    }
}
