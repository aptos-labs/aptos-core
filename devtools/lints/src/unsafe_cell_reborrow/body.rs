// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The scan of a single function or closure body, which is the second pass.
//! It records the cells the body borrows and the calls it makes, for the
//! crate-wide checks, and runs the checks that need only the body itself
//! (in `borrow_site.rs` and `drops.rs`).

use super::{
    borrow_site::{CellPatterns, Site},
    cells::{CellIdx, Cells},
    util::report,
    Config,
};
use clippy_utils::visitors::for_each_expr_without_closures;
use rustc_hir::{
    def::{DefKind, Res},
    def_id::{DefId, LocalDefId},
    intravisit::Visitor,
    Expr, ExprKind, HirId, UnOp,
};
use rustc_lint::LateContext;
use rustc_middle::ty::{
    self,
    adjustment::{Adjust, DerefAdjustKind, PointerCoercion},
    Instance, InstanceKind, Ty, TyCtxt, TypeckResults, TypingEnv,
};
use rustc_span::{Span, Symbol};
use std::ops::ControlFlow;

/// What a call resolves to.
pub(super) enum Callee {
    /// A function or closure of this crate, with a body to follow.
    Local(LocalDefId),
    /// A function of another crate, including a trait default method.
    External(DefId),
    /// A trait method the receiver type does not pin down: a type parameter
    /// or a trait object.
    Dynamic(DefId),
    /// A call through a value: a function pointer, `dyn Fn`, a type parameter.
    Value,
}

impl Callee {
    pub(super) fn describe(&self, tcx: TyCtxt<'_>) -> String {
        match self {
            Callee::Local(def_id) => match tcx.opt_item_name(def_id.to_def_id()) {
                Some(name) => format!("`{name}`"),
                None => "a closure".to_string(),
            },
            Callee::External(def_id) | Callee::Dynamic(def_id) => {
                format!("`{}`", tcx.def_path_str(*def_id))
            },
            Callee::Value => "a function value".to_string(),
        }
    }
}

/// `Local` if `def_id` has a body in this crate. A local declaration with
/// no body (an `extern` item, a required trait method) runs code that is
/// somewhere else.
pub(super) fn local_or_external(tcx: TyCtxt<'_>, def_id: DefId) -> Callee {
    match def_id.as_local() {
        Some(local) if tcx.hir_maybe_body_owned_by(local).is_some() => Callee::Local(local),
        Some(_) | None => Callee::External(def_id),
    }
}

/// A call, explicit or implied by an operator, an index or a dereference
/// that resolves to a trait impl.
pub(super) struct Call<'tcx> {
    pub(super) hir_id: HirId,
    pub(super) span: Span,
    pub(super) callee: Callee,
    /// The operands (receiver first) whose value can reach a cell struct.
    pub(super) reaching: Vec<Reaching<'tcx>>,
    /// The receiver is a field of a cell struct (`self.loader.f()`).
    pub(super) through_field: bool,
    /// Closures passed as arguments.
    pub(super) closure_args: Vec<LocalDefId>,
}

/// An operand of a call that can reach a cell struct.
pub(super) struct Reaching<'tcx> {
    /// Its position in the callee's signature.
    pub(super) position: usize,
    /// Its type as passed, after any coercion.
    pub(super) passed_as: Ty<'tcx>,
}

impl Call<'_> {
    /// Whether the call hands a value that reaches a cell struct to code
    /// with no body in this crate.
    pub(super) fn leaks(&self) -> bool {
        !matches!(self.callee, Callee::Local(_)) && !self.reaching.is_empty()
    }
}

#[derive(Default)]
pub(super) struct BodyInfo<'tcx> {
    /// The cells this body borrows, and where. The body *holds* them.
    pub(super) sites: Vec<(CellIdx, Span)>,
    pub(super) calls: Vec<Call<'tcx>>,
    /// Closures defined in this body; each is a body of its own.
    pub(super) closures: Vec<LocalDefId>,
    /// The body owns a value whose destructor reaches a cell struct.
    pub(super) drops_struct: bool,
}

pub(super) struct BodyScan<'a, 'tcx> {
    pub(super) cx: &'a LateContext<'tcx>,
    pub(super) config: &'a Config,
    pub(super) owner: LocalDefId,
    pub(super) cells: &'a Cells<'tcx>,
    pub(super) typeck: &'tcx TypeckResults<'tcx>,
    pub(super) typing_env: TypingEnv<'tcx>,
    pub(super) info: BodyInfo<'tcx>,
}

/// Scans one body: records its borrows and calls, and reports the checks
/// that need only the body itself.
pub(super) fn scan_body<'tcx>(
    cx: &LateContext<'tcx>,
    config: &Config,
    cells: &Cells<'tcx>,
    owner: LocalDefId,
) -> BodyInfo<'tcx> {
    let tcx = cx.tcx;
    let body = tcx.hir_body_owned_by(owner);
    let mut scan = BodyScan {
        cx,
        config,
        owner,
        cells,
        typeck: tcx.typeck_body(body.id()),
        typing_env: TypingEnv::post_analysis(tcx, owner),
        info: BodyInfo::default(),
    };
    for_each_expr_without_closures(body, |expr| {
        scan.visit(expr);
        ControlFlow::<()>::Continue(())
    });
    CellPatterns { scan: &scan }.visit_body(body);
    scan.info.drops_struct = scan.check_drops(body);
    // Check: no cell is borrowed in a coroutine (an `async` body). It can
    // return to its caller at an `.await` with the borrow still live.
    if tcx.is_coroutine(owner.to_def_id())
        && let Some((_, span)) = scan.info.sites.first()
    {
        report(
            cx,
            tcx.local_def_id_to_hir_id(owner),
            *span,
            format!(
                "{} is reborrowed in an `async` body or other coroutine, which can return to \
                 its caller with the reborrow still live",
                cells.names(tcx, &scan.info.sites)
            ),
            &[],
        );
    }
    scan.info
}

impl<'tcx> BodyScan<'_, 'tcx> {
    pub(super) fn visit(&mut self, expr: &'tcx Expr<'tcx>) {
        self.overloaded_derefs(expr);
        self.check_not_hidden(expr);
        match expr.kind {
            ExprKind::Field(base, ident) => {
                let Some(cell) = self.cell_field(base, ident.name) else {
                    return;
                };
                let name = self.cells.name(self.cx.tcx, cell);
                let (span, problem) = match self.site(expr, self.cells.list[cell].derefs) {
                    Site::Misuse => {
                        report(
                            self.cx,
                            expr.hir_id,
                            expr.span,
                            format!(
                                "{name} used outside a reborrow site; the only permitted form is \
                                 `&mut {stars}self.{}.get()` (or `&{stars}`), passed to an anchor \
                                 function",
                                self.cells.list[cell].field,
                                stars = "*".repeat(self.cells.list[cell].derefs),
                            ),
                            &[],
                        );
                        return;
                    },
                    Site::Bare(span) => (
                        span,
                        Some(format!(
                            "{name} is reborrowed without an anchor, so nothing stops the \
                             reference from leaving this function"
                        )),
                    ),
                    Site::Anchored {
                        span,
                        local_anchor: false,
                    } => (
                        span,
                        Some(format!(
                            "the anchor of this reborrow of {name} is not a local variable of \
                             this function, so it does not confine the reference to it"
                        )),
                    ),
                    Site::Anchored { span, .. } => (span, None),
                };
                if let Some(problem) = problem {
                    report(self.cx, expr.hir_id, span, problem, &[]);
                }
                match self.info.sites.iter().find(|(c, _)| *c == cell) {
                    Some((_, first)) => report(
                        self.cx,
                        expr.hir_id,
                        span,
                        format!("{name} is reborrowed more than once in this function"),
                        &[(*first, "first reborrowed here".to_string())],
                    ),
                    None => self.info.sites.push((cell, span)),
                }
            },
            ExprKind::MethodCall(segment, receiver, args, _) => {
                // The `.get()` of a borrow is part of the borrow, not a call.
                if segment.ident.as_str() == "get"
                    && matches!(receiver.kind, ExprKind::Field(base, ident)
                        if self.cell_field(base, ident.name).is_some())
                {
                    return;
                }
                let callee = self.method_callee(expr);
                self.push_call(expr, callee, Some(receiver), args);
            },
            ExprKind::Call(callee, args) => {
                let resolved = match callee.kind {
                    ExprKind::Path(ref qpath) => match self.cx.qpath_res(qpath, callee.hir_id) {
                        Res::Def(DefKind::Fn | DefKind::AssocFn, def_id) => {
                            self.resolve(def_id, self.typeck.node_args_opt(callee.hir_id))
                        },
                        // A constructor is not a call.
                        Res::Def(DefKind::Ctor(..), _) | Res::SelfCtor(_) => return,
                        _ => self.value_callee(callee),
                    },
                    _ => self.value_callee(callee),
                };
                self.push_call(expr, resolved, None, args);
            },
            // An operator or an index that resolves to a trait impl is a call.
            ExprKind::Binary(_, lhs, rhs)
            | ExprKind::AssignOp(_, lhs, rhs)
            | ExprKind::Index(lhs, rhs, _)
                if self.typeck.is_method_call(expr) =>
            {
                let callee = self.method_callee(expr);
                self.push_call(expr, callee, Some(lhs), std::slice::from_ref(rhs));
            },
            ExprKind::Unary(_, operand) if self.typeck.is_method_call(expr) => {
                let callee = self.method_callee(expr);
                self.push_call(expr, callee, Some(operand), &[]);
            },
            ExprKind::Closure(closure) => self.info.closures.push(closure.def_id),
            _ => {},
        }
    }

    /// The cell `base.ident` names, if any.
    pub(super) fn cell_field(&self, base: &Expr<'tcx>, ident: Symbol) -> Option<CellIdx> {
        let ty::Adt(adt, _) = self.typeck.expr_ty_adjusted(base).peel_refs().kind() else {
            return None;
        };
        self.cells.index_of(adt.did(), ident)
    }

    /// The callee of a method call or an overloaded operator.
    pub(super) fn method_callee(&self, expr: &Expr<'tcx>) -> Callee {
        match self.typeck.type_dependent_def_id(expr.hir_id) {
            Some(def_id) => self.resolve(def_id, self.typeck.node_args_opt(expr.hir_id)),
            None => Callee::Value,
        }
    }

    /// A call through a value: `f(x)` with `f` a closure, a function item, or
    /// something opaque.
    pub(super) fn value_callee(&self, callee: &Expr<'tcx>) -> Callee {
        match self.typeck.expr_ty_adjusted(callee).peel_refs().kind() {
            ty::Closure(def_id, _) => local_or_external(self.cx.tcx, *def_id),
            ty::FnDef(def_id, args) => self.resolve(*def_id, Some(args.skip_binder())),
            _ => Callee::Value,
        }
    }

    /// Resolves a callee to the body that will run, if the types at the call
    /// pin it down. `args` is `None` when typeck recorded none.
    pub(super) fn resolve(&self, def_id: DefId, args: Option<ty::GenericArgsRef<'tcx>>) -> Callee {
        let tcx = self.cx.tcx;
        let is_trait_item = tcx.trait_of_assoc(def_id).is_some();
        let Some(args) = args.filter(|args| !(is_trait_item && args.is_empty())) else {
            return local_or_external(tcx, def_id);
        };
        match Instance::try_resolve(tcx, self.typing_env, def_id, args) {
            Ok(Some(instance)) if !matches!(instance.def, InstanceKind::Virtual(..)) => {
                local_or_external(tcx, instance.def_id())
            },
            // A trait object, or a type parameter the body does not pin down.
            _ if is_trait_item => Callee::Dynamic(def_id),
            _ => local_or_external(tcx, def_id),
        }
    }

    /// Whether the value of `expr` reaches a cell struct, before or after
    /// any coercion at this use.
    pub(super) fn reaches(&self, expr: &Expr<'tcx>) -> bool {
        let tcx = self.cx.tcx;
        self.cells.reaches(tcx, self.typeck.expr_ty(expr))
            || self.cells.reaches(tcx, self.typeck.expr_ty_adjusted(expr))
    }

    pub(super) fn push_call(
        &mut self,
        expr: &Expr<'tcx>,
        callee: Callee,
        receiver: Option<&Expr<'tcx>>,
        args: &[Expr<'tcx>],
    ) {
        let reaching = receiver
            .into_iter()
            .chain(args)
            .enumerate()
            .filter(|(_, e)| self.reaches(e))
            .map(|(position, e)| Reaching {
                position,
                passed_as: self.typeck.expr_ty_adjusted(e),
            })
            .collect();
        let through_field = receiver.is_some_and(|r| self.is_field_of_cell_struct(r));
        let closure_args = args
            .iter()
            .filter_map(
                |a| match self.typeck.expr_ty_adjusted(a).peel_refs().kind() {
                    ty::Closure(def_id, _) => def_id.as_local(),
                    _ => None,
                },
            )
            .collect();
        self.info.calls.push(Call {
            hir_id: expr.hir_id,
            span: expr.span,
            callee,
            reaching,
            through_field,
            closure_args,
        });
    }

    /// Records a call for each dereference the compiler inserted that
    /// resolves to a `Deref` impl (`handle.len()` with `Handle: Deref`).
    pub(super) fn overloaded_derefs(&mut self, expr: &'tcx Expr<'tcx>) {
        let tcx = self.cx.tcx;
        let mut source = self.typeck.expr_ty(expr);
        for adjustment in self.typeck.expr_adjustments(expr) {
            if let Adjust::Deref(DerefAdjustKind::Overloaded(deref)) = adjustment.kind {
                let callee = self.resolve(
                    deref.method_call(tcx),
                    Some(tcx.mk_args(&[source.peel_refs().into()])),
                );
                let reaching = if self.cells.reaches(tcx, source) {
                    vec![Reaching {
                        position: 0,
                        passed_as: source,
                    }]
                } else {
                    Vec::new()
                };
                self.info.calls.push(Call {
                    hir_id: expr.hir_id,
                    span: expr.span,
                    callee,
                    reaching,
                    through_field: self.is_field_of_cell_struct(expr),
                    closure_args: Vec::new(),
                });
            }
            source = adjustment.target;
        }
    }

    /// Check, in any body: a value that reaches a cell struct is not turned
    /// into a trait object (an `as` cast to one is a coercion too), and is
    /// not cast to a type that no longer shows the struct
    /// (`self as *const Self as *const ()`). The lint could not follow the
    /// value afterwards.
    fn check_not_hidden(&self, expr: &'tcx Expr<'tcx>) {
        let tcx = self.cx.tcx;
        if let ExprKind::Cast(operand, _) = expr.kind
            && self
                .cells
                .reaches(tcx, self.typeck.expr_ty_adjusted(operand))
            && !self.cells.reaches(tcx, self.typeck.expr_ty(expr))
        {
            report(
                self.cx,
                expr.hir_id,
                expr.span,
                "a value that can reach a cell struct is cast to a type that hides the struct; \
                 where it goes afterwards cannot be followed"
                    .to_string(),
                &[],
            );
        }
        let mut hidden = false;
        let mut source = self.typeck.expr_ty(expr);
        for adjustment in self.typeck.expr_adjustments(expr) {
            hidden |= matches!(adjustment.kind, Adjust::Pointer(PointerCoercion::Unsize))
                && self.cells.reaches(tcx, source)
                && !self.cells.reaches(tcx, adjustment.target);
            source = adjustment.target;
        }
        if hidden {
            report(
                self.cx,
                expr.hir_id,
                expr.span,
                "a value that can reach a cell struct is turned into a trait object; what is \
                 called on it cannot be followed"
                    .to_string(),
                &[],
            );
        }
    }

    /// Whether `expr` is `s.field` (through any `&` / `*`), with `s` a cell
    /// struct.
    pub(super) fn is_field_of_cell_struct(&self, mut expr: &Expr<'tcx>) -> bool {
        loop {
            match expr.kind {
                ExprKind::AddrOf(_, _, inner) | ExprKind::Unary(UnOp::Deref, inner) => expr = inner,
                ExprKind::Field(base, _) => {
                    return self.cells.is_struct(self.typeck.expr_ty_adjusted(base));
                },
                _ => return false,
            }
        }
    }
}
