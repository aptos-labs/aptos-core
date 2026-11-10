// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `unsafe_cell_reborrow`: the discipline for `&self` methods that mutate
//! through `UnsafeCell<&mut T>` fields ("cells").
//!
//! A struct whose methods take `&self` but need `&mut` access to some
//! components keeps them in `UnsafeCell<&mut T>` fields and reborrows
//! `&mut **self.cell.get()` inside each method. That is sound only if each
//! cell has at most one live reborrow at any instant and no reborrow outlives
//! the activation that made it. The borrow checker cannot see this, so this
//! lint enforces a coarse, over-conservative discipline on every function in
//! a crate that defines such a struct:
//!
//! 1. Cell fields are private and appear only at a *site*, written
//!    `&mut **self.cell.get()` (or `&**` for a shared reborrow).
//! 2. A function has at most one site per cell, and the reborrow counts as
//!    held for the whole function body.
//! 3. A function that holds a cell calls nothing that may reborrow the same
//!    cell, directly or transitively. The call graph is followed through the
//!    functions of this crate; a closure is part of the function that defines
//!    it, and a closure that can reach the struct or a cell is never passed
//!    as an argument.
//! 4. While a cell is held, nothing that can reach the struct (a reference to
//!    it, a closure capturing it) is passed to code the lint cannot see
//!    (another crate), and no method is called on a type parameter, a trait
//!    object or a function value, other than through a field of the struct.
//!    Methods of the traits listed as `inert_traits` in the config are exempt
//!    (see 6).
//! 5. No method of the struct, and no function with a site, has a signature
//!    that can hold a reference to a cell's target type: a reborrow cannot be
//!    handed out or stored through an out-parameter.
//! 6. An impl of an inert trait cannot reach a `context_traits` trait: its
//!    self type mentions no type parameter bounded by one and no trait object
//!    of one, and its methods call none of its methods. This is what makes a
//!    call on an inert trait method safe under 4.
//!
//! Rules 1 to 5 run in any crate that defines a cell struct; rule 6 runs in
//! every crate in scope. Which crates, and which traits are inert and which
//! are contexts, is configured in `dylint.toml`.

use crate::scope::Scope;
use clippy_utils::{diagnostics::span_lint_hir_and_then, get_parent_expr};
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::{
    self as hir,
    attrs::lang_items::LangItem,
    def::{DefKind, Res},
    def_id::{DefId, LocalDefId},
    intravisit::{self, Visitor},
    BorrowKind, Expr, ExprKind, HirId, Mutability, UnOp,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Instance, InstanceKind, Ty, TyCtxt, TypeckResults, TypingEnv};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::{Span, Symbol};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

declare_lint! {
    /// ### What it does
    ///
    /// Checks the discipline for structs that mutate through
    /// `UnsafeCell<&mut T>` fields from `&self` methods: one reborrow per
    /// cell per function, no call that may reborrow a held cell, no escape
    /// of a reborrow, and no call into unseen code while a cell is held.
    ///
    /// ### Why is this bad?
    ///
    /// Two live reborrows of the same cell, or a reborrow that outlives its
    /// function, are undefined behaviour that compiles and runs. The borrow
    /// checker cannot see through the cell, so the rules must be checked
    /// here.
    ///
    /// ### Example
    ///
    /// ```rust,ignore
    /// fn outer(&self) {
    ///     let heap = unsafe { &mut **self.heap.get() };
    ///     self.inner(); // also reborrows `heap`: two live `&mut Heap`
    /// }
    /// ```
    ///
    /// Use instead: reborrow once in the entry point and pass the component
    /// down, so the borrow checker sees every use.
    ///
    /// ```rust,ignore
    /// fn outer(&self) {
    ///     let heap = unsafe { &mut **self.heap.get() };
    ///     inner(heap);
    /// }
    /// ```
    pub UNSAFE_CELL_REBORROW,
    Deny,
    "overlapping or escaping reborrow of an `UnsafeCell<&mut T>` field"
}

/// Configuration, from `[aptos_lints.unsafe_cell_reborrow]` in `dylint.toml`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    #[serde(flatten)]
    pub scope: Scope,
    /// Traits (full paths) whose methods may be called on a type parameter
    /// while a cell is held. Their impls are checked by rule 6.
    pub inert_traits: Vec<String>,
    /// Traits (full paths) an inert impl must not be able to reach: the
    /// public API of the cell struct.
    pub context_traits: Vec<String>,
}

pub struct UnsafeCellReborrow {
    config: Config,
}

impl_lint_pass!(UnsafeCellReborrow => [UNSAFE_CELL_REBORROW]);

impl UnsafeCellReborrow {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl<'tcx> LateLintPass<'tcx> for UnsafeCellReborrow {
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        if !self.config.scope.applies(cx) {
            return;
        }
        let cells = find_cells(cx);
        if !cells.is_empty() {
            Analysis::new(cx, &self.config, cells).run();
        }
        check_inert_impls(cx, &self.config);
    }
}

fn report(cx: &LateContext<'_>, hir_id: HirId, span: Span, msg: String, notes: &[(Span, String)]) {
    span_lint_hir_and_then(cx, UNSAFE_CELL_REBORROW, hir_id, span, msg, |diag| {
        for (span, note) in notes {
            diag.span_note(*span, note.clone());
        }
        diag.help("see `unsafe_cell_reborrow` in devtools/lints/README.md for the rules");
    });
}

/// `def_path_str` without the generic arguments it prints for impl self
/// types (`Ctx::<'a>::leaf` becomes `Ctx::leaf`).
fn callee_name(tcx: TyCtxt<'_>, def_id: DefId) -> String {
    let path = tcx.def_path_str(def_id);
    let mut out = String::with_capacity(path.len());
    let mut depth = 0usize;
    let mut chars = path.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ':' if depth == 0 && chars.peek() == Some(&':') => {
                chars.next();
                // Skip a `::<...>` segment entirely.
                if chars.peek() == Some(&'<') {
                    chars.next();
                    depth = 1;
                } else {
                    out.push_str("::");
                }
            },
            '<' if depth > 0 => depth += 1,
            '>' if depth > 0 => depth -= 1,
            _ if depth > 0 => {},
            _ => out.push(c),
        }
    }
    out
}

/// The full path of a definition, crate name included (unlike
/// `def_path_str`, which omits the local crate's name).
fn full_path(cx: &LateContext<'_>, def_id: DefId) -> String {
    cx.get_def_path(def_id)
        .iter()
        .map(Symbol::as_str)
        .collect::<Vec<_>>()
        .join("::")
}

fn trait_listed(cx: &LateContext<'_>, list: &[String], trait_def_id: DefId) -> bool {
    let path = full_path(cx, trait_def_id);
    list.iter().any(|t| *t == path)
}

/// Whether `trait_def_id` is, or has as a supertrait, a trait in `list`.
fn reaches_listed_trait(cx: &LateContext<'_>, list: &[String], trait_def_id: DefId) -> bool {
    ty::elaborate::supertrait_def_ids(cx.tcx, trait_def_id).any(|t| trait_listed(cx, list, t))
}

// ---------------------------------------------------------------------------
// Cells
// ---------------------------------------------------------------------------

/// One `UnsafeCell<&mut T>` field.
struct Cell<'tcx> {
    /// The struct that owns the field.
    owner: DefId,
    field: Symbol,
    /// The `T`.
    target: Ty<'tcx>,
}

type CellIdx = usize;

/// Every `UnsafeCell<&mut T>` field of every struct in the crate (rule 1's
/// privacy check happens here too).
fn find_cells<'tcx>(cx: &LateContext<'tcx>) -> Vec<Cell<'tcx>> {
    let tcx = cx.tcx;
    let mut cells = Vec::new();
    for id in tcx.hir_crate_items(()).free_items() {
        let item = tcx.hir_item(id);
        let hir::ItemKind::Struct(_, _, data) = item.kind else {
            continue;
        };
        for field in data.fields() {
            let ty = tcx
                .type_of(field.def_id)
                .instantiate_identity()
                .skip_normalization();
            let ty::Adt(adt, args) = ty.kind() else {
                continue;
            };
            if !tcx.is_lang_item(adt.did(), LangItem::UnsafeCell) {
                continue;
            }
            let ty::Ref(_, target, Mutability::Mut) = args.type_at(0).kind() else {
                continue;
            };
            let owner = item.owner_id.to_def_id();
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
            cells.push(Cell {
                owner,
                field: field.ident.name,
                target: *target,
            });
        }
    }
    cells
}

// ---------------------------------------------------------------------------
// Per-body scan
// ---------------------------------------------------------------------------

/// What a call may be, as far as the lint can tell.
enum Callee {
    /// A function or closure of this crate.
    Local(LocalDefId),
    /// A function of another crate (including a trait default method).
    External,
    /// A trait method the receiver type does not pin down: a type parameter
    /// or a trait object.
    Dynamic { trait_def_id: Option<DefId> },
    /// A call through a function value: pointer, `dyn Fn`, type parameter.
    Value,
}

struct Call {
    hir_id: HirId,
    span: Span,
    callee: Callee,
    /// What the callee is called, for diagnostics.
    name: String,
    /// The receiver or an argument can reach a cell struct.
    tainted: bool,
    /// The receiver is a field projection of a cell struct (`self.loader.f()`).
    through_field: bool,
    /// Closures passed as arguments.
    closure_args: Vec<LocalDefId>,
}

#[derive(Default)]
struct BodyInfo {
    /// Reborrow sites: cell, the expression, its span.
    sites: Vec<(CellIdx, HirId, Span)>,
    calls: Vec<Call>,
    /// Closures defined in this body.
    closures: Vec<LocalDefId>,
    /// Uses of a cell field other than at a site.
    misuses: Vec<(HirId, Span, CellIdx)>,
}

struct BodyScan<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    cells: &'a [Cell<'tcx>],
    typeck: &'tcx TypeckResults<'tcx>,
    typing_env: TypingEnv<'tcx>,
    info: BodyInfo,
}

impl<'a, 'tcx> BodyScan<'a, 'tcx> {
    fn tcx(&self) -> TyCtxt<'tcx> {
        self.cx.tcx
    }

    fn is_cell_struct(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if self.cells.iter().any(|c| c.owner == adt.did()))
    }

    /// The cell `base.ident` names, if any.
    fn cell_field(&self, base: &Expr<'tcx>, ident: Symbol) -> Option<CellIdx> {
        let ty::Adt(adt, _) = self.typeck.expr_ty_adjusted(base).peel_refs().kind() else {
            return None;
        };
        self.cells
            .iter()
            .position(|c| c.owner == adt.did() && c.field == ident)
    }

    fn is_cell_field_expr(&self, expr: &Expr<'tcx>) -> bool {
        matches!(expr.kind, ExprKind::Field(base, ident) if self.cell_field(base, ident.name).is_some())
    }

    /// A value whose type mentions a cell struct: `&S`, `Option<&S>`, a
    /// closure capturing one, ...
    fn is_tainted(&self, expr: &Expr<'tcx>) -> bool {
        ty_mentions_cell_struct(self.cells, self.typeck.expr_ty_adjusted(expr))
    }

    /// `self.field` (through any `&`/`*`), with `self` a cell struct.
    fn is_field_of_cell_struct(&self, mut expr: &Expr<'tcx>) -> bool {
        loop {
            match expr.kind {
                ExprKind::AddrOf(_, _, inner) | ExprKind::Unary(UnOp::Deref, inner) => expr = inner,
                ExprKind::Field(base, _) => {
                    return self.is_cell_struct(self.typeck.expr_ty_adjusted(base));
                },
                _ => return false,
            }
        }
    }

    /// `args` is `None` when typeck recorded none for the call, which it
    /// skips for a callee with no generics at all; such a callee is a plain
    /// function or inherent method, resolved by its definition alone.
    fn resolve(&self, def_id: DefId, args: Option<ty::GenericArgsRef<'tcx>>) -> Callee {
        let tcx = self.tcx();
        let Some(args) = args else {
            return local_or_external(def_id);
        };
        // A trait item needs at least its `Self` argument to resolve.
        if tcx.trait_of_assoc(def_id).is_some() && args.is_empty() {
            return local_or_external(def_id);
        }
        match Instance::try_resolve(tcx, self.typing_env, def_id, args) {
            Ok(Some(instance)) => match instance.def {
                InstanceKind::Virtual(..) => Callee::Dynamic {
                    trait_def_id: self.tcx().trait_of_assoc(def_id),
                },
                _ => local_or_external(instance.def_id()),
            },
            // Depends on a type parameter the body does not pin down.
            Ok(None) | Err(_) => match self.tcx().trait_of_assoc(def_id) {
                Some(trait_def_id) => Callee::Dynamic {
                    trait_def_id: Some(trait_def_id),
                },
                None => local_or_external(def_id),
            },
        }
    }

    fn push_call(
        &mut self,
        expr: &Expr<'tcx>,
        callee: Callee,
        name: String,
        receiver: Option<&Expr<'tcx>>,
        args: &[Expr<'tcx>],
    ) {
        let tainted =
            receiver.is_some_and(|r| self.is_tainted(r)) || args.iter().any(|a| self.is_tainted(a));
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
            name,
            tainted,
            through_field,
            closure_args,
        });
    }

    fn method_call(&mut self, expr: &Expr<'tcx>, receiver: &Expr<'tcx>, args: &[Expr<'tcx>]) {
        let callee = match self.typeck.type_dependent_def_id(expr.hir_id) {
            Some(def_id) => self.resolve(def_id, self.typeck.node_args_opt(expr.hir_id)),
            None => Callee::Value,
        };
        let name = match self.typeck.type_dependent_def_id(expr.hir_id) {
            Some(def_id) => callee_name(self.tcx(), def_id),
            None => "<unknown>".to_string(),
        };
        self.push_call(expr, callee, name, Some(receiver), args);
    }

    fn call(&mut self, expr: &Expr<'tcx>, callee: &Expr<'tcx>, args: &[Expr<'tcx>]) {
        if let ExprKind::Path(qpath) = &callee.kind {
            match self.cx.qpath_res(qpath, callee.hir_id) {
                Res::Def(DefKind::Fn | DefKind::AssocFn, def_id) => {
                    let resolved = self.resolve(def_id, self.typeck.node_args_opt(callee.hir_id));
                    let name = callee_name(self.tcx(), def_id);
                    self.push_call(expr, resolved, name, None, args);
                    return;
                },
                // A constructor: not a call.
                Res::Def(DefKind::Ctor(..), _) | Res::SelfCtor(_) => return,
                _ => {},
            }
        }
        // A call through a value.
        let callee_ty = self.typeck.expr_ty_adjusted(callee).peel_refs();
        let (resolved, name) = match callee_ty.kind() {
            ty::Closure(def_id, _) => (local_or_external(*def_id), "closure".to_string()),
            ty::FnDef(def_id, args) => (
                self.resolve(*def_id, Some(args.skip_binder())),
                callee_name(self.tcx(), *def_id),
            ),
            _ => (Callee::Value, "function value".to_string()),
        };
        self.push_call(expr, resolved, name, None, args);
    }
}

fn local_or_external(def_id: DefId) -> Callee {
    match def_id.as_local() {
        Some(local) => Callee::Local(local),
        None => Callee::External,
    }
}

fn ty_mentions_cell_struct<'tcx>(cells: &[Cell<'tcx>], ty: Ty<'tcx>) -> bool {
    ty.walk().any(|arg| match arg.as_type().map(|t| *t.kind()) {
        Some(ty::Adt(adt, _)) => cells.iter().any(|c| c.owner == adt.did()),
        _ => false,
    })
}

/// A site is exactly `&mut **<field>.get()` or `&**<field>.get()`: the span
/// of the whole reborrow if `field_expr` is used that way.
fn site_span<'tcx>(cx: &LateContext<'tcx>, field_expr: &Expr<'tcx>) -> Option<Span> {
    let get = get_parent_expr(cx, field_expr)?;
    let ExprKind::MethodCall(segment, receiver, [], _) = get.kind else {
        return None;
    };
    if segment.ident.as_str() != "get" || receiver.hir_id != field_expr.hir_id {
        return None;
    }
    let deref1 = get_parent_expr(cx, get)?;
    let ExprKind::Unary(UnOp::Deref, inner) = deref1.kind else {
        return None;
    };
    if inner.hir_id != get.hir_id {
        return None;
    }
    let deref2 = get_parent_expr(cx, deref1)?;
    let ExprKind::Unary(UnOp::Deref, inner) = deref2.kind else {
        return None;
    };
    if inner.hir_id != deref1.hir_id {
        return None;
    }
    let borrow = get_parent_expr(cx, deref2)?;
    let ExprKind::AddrOf(BorrowKind::Ref, _, inner) = borrow.kind else {
        return None;
    };
    (inner.hir_id == deref2.hir_id).then_some(borrow.span)
}

impl<'tcx> Visitor<'tcx> for BodyScan<'_, 'tcx> {
    // The default `NestedFilter` skips nested bodies: each closure is scanned
    // as a body of its own and linked to its definer through `closures`.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        match expr.kind {
            ExprKind::Field(base, ident) => {
                if let Some(cell) = self.cell_field(base, ident.name) {
                    match site_span(self.cx, expr) {
                        Some(span) => self.info.sites.push((cell, expr.hir_id, span)),
                        None => self.info.misuses.push((expr.hir_id, expr.span, cell)),
                    }
                }
            },
            ExprKind::MethodCall(segment, receiver, args, _) => {
                // The `.get()` of a site is not a call of interest.
                if !(segment.ident.as_str() == "get" && self.is_cell_field_expr(receiver)) {
                    self.method_call(expr, receiver, args);
                }
            },
            ExprKind::Call(callee, args) => self.call(expr, callee, args),
            ExprKind::Closure(closure) => self.info.closures.push(closure.def_id),
            _ => {},
        }
        intravisit::walk_expr(self, expr);
    }
}

// ---------------------------------------------------------------------------
// Crate-wide analysis
// ---------------------------------------------------------------------------

struct Analysis<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    config: &'a Config,
    cells: Vec<Cell<'tcx>>,
    bodies: FxHashMap<LocalDefId, BodyInfo>,
    /// Cells a body may reborrow when called: its own sites, its closures',
    /// and those of everything it calls in this crate.
    transitive: FxHashMap<LocalDefId, BTreeSet<CellIdx>>,
}

impl<'a, 'tcx> Analysis<'a, 'tcx> {
    fn new(cx: &'a LateContext<'tcx>, config: &'a Config, cells: Vec<Cell<'tcx>>) -> Self {
        Self {
            cx,
            config,
            cells,
            bodies: FxHashMap::default(),
            transitive: FxHashMap::default(),
        }
    }

    fn cell_name(&self, cell: CellIdx) -> String {
        let c = &self.cells[cell];
        format!("`{}.{}`", self.cx.tcx.item_name(c.owner), c.field)
    }

    fn run(mut self) {
        let tcx = self.cx.tcx;
        for owner in tcx.hir_body_owners() {
            let body = tcx.hir_body_owned_by(owner);
            let mut scan = BodyScan {
                cx: self.cx,
                cells: &self.cells,
                typeck: tcx.typeck_body(body.id()),
                typing_env: TypingEnv::post_analysis(tcx, owner),
                info: BodyInfo::default(),
            };
            scan.visit_body(body);
            self.bodies.insert(owner, scan.info);
        }
        self.compute_transitive();
        // In definition order, so diagnostics come out in a stable order.
        let mut owners: Vec<LocalDefId> = self.bodies.keys().copied().collect();
        owners.sort_by_key(|owner| owner.local_def_index.as_u32());
        for owner in owners {
            self.check_body(owner, &self.bodies[&owner]);
        }
        self.check_signatures();
    }

    /// Fixpoint over the call graph restricted to this crate.
    fn compute_transitive(&mut self) {
        for (owner, info) in &self.bodies {
            let own: BTreeSet<_> = info.sites.iter().map(|(cell, ..)| *cell).collect();
            self.transitive.insert(*owner, own);
        }
        loop {
            let mut changed = false;
            for (owner, info) in &self.bodies {
                let mut set = self.transitive[owner].clone();
                let callees = info
                    .closures
                    .iter()
                    .copied()
                    .chain(info.calls.iter().filter_map(|c| match c.callee {
                        Callee::Local(def_id) => Some(def_id),
                        _ => None,
                    }));
                for callee in callees {
                    if let Some(theirs) = self.transitive.get(&callee) {
                        set.extend(theirs.iter().copied());
                    }
                }
                if set.len() != self.transitive[owner].len() {
                    self.transitive.insert(*owner, set);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    }

    fn captures_cell_struct(&self, closure: LocalDefId) -> bool {
        let ty = self
            .cx
            .tcx
            .type_of(closure)
            .instantiate_identity()
            .skip_normalization();
        let ty::Closure(_, args) = ty.kind() else {
            return false;
        };
        args.as_closure()
            .upvar_tys()
            .iter()
            .any(|ty| ty_mentions_cell_struct(&self.cells, ty))
    }

    fn check_body(&self, owner: LocalDefId, info: &BodyInfo) {
        let cx = self.cx;
        for (hir_id, span, cell) in &info.misuses {
            report(
                cx,
                *hir_id,
                *span,
                format!(
                    "{} used outside a reborrow site; the only permitted form is \
                     `&mut **self.{}.get()` (or `&**`)",
                    self.cell_name(*cell),
                    self.cells[*cell].field
                ),
                &[],
            );
        }

        // Rule 2: one site per cell.
        let mut first: BTreeMap<CellIdx, Span> = BTreeMap::new();
        for (cell, hir_id, span) in &info.sites {
            match first.get(cell) {
                None => {
                    first.insert(*cell, *span);
                },
                Some(earlier) => report(
                    cx,
                    *hir_id,
                    *span,
                    format!(
                        "{} is reborrowed more than once in this function",
                        self.cell_name(*cell)
                    ),
                    &[(*earlier, "first reborrowed here".to_string())],
                ),
            }
        }
        let held: Vec<(CellIdx, Span)> = first.into_iter().collect();

        for call in &info.calls {
            // Rule 3: a closure that can reach the struct or a cell goes nowhere.
            let bad_closure = call.closure_args.iter().any(|closure| {
                let reaches_cells = self
                    .transitive
                    .get(closure)
                    .is_some_and(|set| !set.is_empty());
                reaches_cells || self.captures_cell_struct(*closure)
            });
            if bad_closure {
                report(
                    cx,
                    call.hir_id,
                    call.span,
                    "a closure that can reach a cell struct is passed as an argument; call it \
                     directly or pass the components it needs instead"
                        .to_string(),
                    &[],
                );
                // One finding per call: the closure is the problem, whatever
                // the callee is.
                continue;
            }
            if held.is_empty() {
                continue;
            }
            let held_names = || {
                held.iter()
                    .map(|(cell, _)| self.cell_name(*cell))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let held_notes = || {
                held.iter()
                    .map(|(cell, span)| {
                        (
                            *span,
                            format!("{} is reborrowed here", self.cell_name(*cell)),
                        )
                    })
                    .collect::<Vec<_>>()
            };
            match &call.callee {
                Callee::Local(callee) => {
                    let Some(theirs) = self.transitive.get(callee) else {
                        continue;
                    };
                    let overlap: Vec<_> = held
                        .iter()
                        .filter(|(cell, _)| theirs.contains(cell))
                        .collect();
                    if overlap.is_empty() {
                        continue;
                    }
                    let names = overlap
                        .iter()
                        .map(|(cell, _)| self.cell_name(*cell))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let notes: Vec<_> = overlap
                        .iter()
                        .map(|(cell, span)| {
                            (
                                *span,
                                format!("{} is reborrowed here", self.cell_name(*cell)),
                            )
                        })
                        .collect();
                    report(
                        cx,
                        call.hir_id,
                        call.span,
                        format!(
                            "`{}` may reborrow {names}, which this function already holds",
                            call.name
                        ),
                        &notes,
                    );
                },
                Callee::External => {
                    if call.tainted {
                        report(
                            cx,
                            call.hir_id,
                            call.span,
                            format!(
                                "a cell struct reaches `{}`, whose body is outside this crate, \
                                 while {} is held",
                                call.name,
                                held_names()
                            ),
                            &held_notes(),
                        );
                    }
                },
                Callee::Dynamic { trait_def_id } => {
                    let inert = trait_def_id
                        .is_some_and(|t| trait_listed(cx, &self.config.inert_traits, t));
                    if !call.through_field && !inert {
                        report(
                            cx,
                            call.hir_id,
                            call.span,
                            format!(
                                "`{}` is called on a type parameter or trait object while {} is \
                                 held; the callee may reach the struct through its caller",
                                call.name,
                                held_names()
                            ),
                            &held_notes(),
                        );
                    }
                },
                Callee::Value => {
                    if !call.through_field {
                        report(
                            cx,
                            call.hir_id,
                            call.span,
                            format!(
                                "call through a {} while {} is held; the callee may reach the \
                                 struct through its caller",
                                call.name,
                                held_names()
                            ),
                            &held_notes(),
                        );
                    }
                },
            }
        }

        // Rule 5 for a function with a site: its return type. Methods of a
        // cell struct are covered by `check_signatures` instead.
        let tcx = cx.tcx;
        let is_cell_struct_method =
            tcx.impl_of_assoc(owner.to_def_id())
                .is_some_and(|impl_def_id| {
                    let self_ty = tcx
                        .type_of(impl_def_id)
                        .instantiate_identity()
                        .skip_normalization();
                    ty_mentions_cell_struct(&self.cells, self_ty)
                });
        if let Some((_, site)) = held.first()
            && !is_cell_struct_method
        {
            let output = match tcx.def_kind(owner) {
                DefKind::Fn | DefKind::AssocFn => {
                    Some(tcx.fn_sig(owner).skip_binder().skip_binder().output())
                },
                DefKind::Closure => {
                    let ty = tcx
                        .type_of(owner)
                        .instantiate_identity()
                        .skip_normalization();
                    match ty.kind() {
                        ty::Closure(_, args) => {
                            Some(args.as_closure().sig().skip_binder().output())
                        },
                        _ => None,
                    }
                },
                _ => None,
            };
            if let Some(output) = output
                && let Some(what) = self.holds_target_ref(output)
            {
                report(
                    cx,
                    tcx.local_def_id_to_hir_id(owner),
                    tcx.def_span(owner),
                    format!("the return type of this function can hold {what}, so a reborrow can escape"),
                    &[(*site, "reborrowed here".to_string())],
                );
            }
        }
    }

    /// Rule 5 for every method of a cell struct: no parameter can hold a
    /// reference to a cell target (a plain `&mut T` parameter is fine: it
    /// cannot store one), and neither can the return type.
    fn check_signatures(&self) {
        let tcx = self.cx.tcx;
        for id in tcx.hir_crate_items(()).impl_items() {
            let item = tcx.hir_impl_item(id);
            let hir::ImplItemKind::Fn(..) = item.kind else {
                continue;
            };
            let def_id = item.owner_id.to_def_id();
            let Some(impl_def_id) = tcx.impl_of_assoc(def_id) else {
                continue;
            };
            let self_ty = tcx
                .type_of(impl_def_id)
                .instantiate_identity()
                .skip_normalization();
            if !ty_mentions_cell_struct(&self.cells, self_ty) {
                continue;
            }
            if !tcx.associated_item(def_id).is_method() {
                // A constructor takes the components in.
                continue;
            }
            let sig = tcx.fn_sig(def_id).skip_binder().skip_binder();
            let mut problems = Vec::new();
            for input in sig.inputs().iter().skip(1) {
                let pass_through =
                    matches!(input.kind(), ty::Ref(_, inner, _) if self.is_target(*inner));
                if pass_through {
                    continue;
                }
                if let Some(what) = self.holds_target_ref(*input) {
                    problems.push(format!("a parameter can hold {what}"));
                }
            }
            if let Some(what) = self.holds_target_ref(sig.output()) {
                problems.push(format!("the return type can hold {what}"));
            }
            for problem in problems {
                report(
                    self.cx,
                    item.hir_id(),
                    item.span,
                    format!(
                        "in `{}`, {problem}, so a reborrow can escape",
                        callee_name(tcx, def_id)
                    ),
                    &[],
                );
            }
        }
    }

    fn is_target(&self, ty: Ty<'tcx>) -> bool {
        let ty::Adt(adt, _) = ty.kind() else {
            return false;
        };
        self.cells
            .iter()
            .any(|c| matches!(c.target.kind(), ty::Adt(t, _) if t.did() == adt.did()))
    }

    /// If `ty` can contain a reference to a cell target, a description of
    /// how. Walks generic arguments everywhere and fields of this crate's
    /// ADTs; a trait object or opaque type is treated as able to hold one.
    fn holds_target_ref(&self, ty: Ty<'tcx>) -> Option<String> {
        let tcx = self.cx.tcx;
        let mut seen: Vec<Ty<'tcx>> = Vec::new();
        let mut stack = vec![ty];
        while let Some(ty) = stack.pop() {
            if seen.contains(&ty) {
                continue;
            }
            seen.push(ty);
            match ty.kind() {
                ty::Ref(_, inner, _) | ty::RawPtr(inner, _) => {
                    if self.is_target(inner.peel_refs()) {
                        return Some(format!("a reference to `{}`", inner.peel_refs()));
                    }
                    stack.push(*inner);
                },
                ty::Dynamic(..) => return Some("a trait object".to_string()),
                ty::Alias(_, alias)
                    if matches!(alias.kind(tcx), ty::AliasTyKind::Opaque { .. }) =>
                {
                    return Some("an opaque type".to_string());
                },
                ty::Adt(adt, args) => {
                    stack.extend(args.types());
                    if adt.did().is_local() {
                        for field in adt.all_fields() {
                            stack.push(
                                tcx.type_of(field.did)
                                    .instantiate(tcx, args)
                                    .skip_normalization(),
                            );
                        }
                    }
                },
                ty::Closure(_, args) => stack.extend(args.as_closure().upvar_tys().iter()),
                ty::Tuple(tys) => stack.extend(tys.iter()),
                ty::Array(elem, _) | ty::Slice(elem) => stack.push(*elem),
                _ => {},
            }
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Rule 6: impls of inert traits
// ---------------------------------------------------------------------------

/// Every impl of an inert trait in the crate: its self type must not reach a
/// context trait, and its methods must not call one.
fn check_inert_impls<'tcx>(cx: &LateContext<'tcx>, config: &Config) {
    if config.inert_traits.is_empty() || config.context_traits.is_empty() {
        return;
    }
    let tcx = cx.tcx;
    for id in tcx.hir_crate_items(()).free_items() {
        let item = tcx.hir_item(id);
        let hir::ItemKind::Impl(impl_) = item.kind else {
            continue;
        };
        let impl_def_id = item.owner_id.to_def_id();
        let Some(trait_def_id) = impl_
            .of_trait
            .and_then(|header| header.trait_ref.trait_def_id())
        else {
            continue;
        };
        if !trait_listed(cx, &config.inert_traits, trait_def_id) {
            continue;
        }
        let inert = tcx.item_name(trait_def_id);

        // A type parameter bounded by a context trait.
        for (clause, span) in tcx.explicit_clauses_of(impl_def_id).clauses {
            let Some(pred) = clause.as_trait_clause() else {
                continue;
            };
            if reaches_listed_trait(cx, &config.context_traits, pred.def_id()) {
                report(
                    cx,
                    item.hir_id(),
                    *span,
                    format!(
                        "impl of inert trait `{inert}` for a type bounded by `{}`: the value could \
                         reach a cell struct",
                        tcx.def_path_str(pred.def_id())
                    ),
                    &[],
                );
            }
        }
        // A trait object of a context trait anywhere in the self type.
        let self_ty = tcx
            .type_of(impl_def_id)
            .instantiate_identity()
            .skip_normalization();
        if let Some(found) = find_context_object(cx, config, self_ty) {
            report(
                cx,
                item.hir_id(),
                item.span,
                format!(
                    "impl of inert trait `{inert}` for a type holding `dyn {}`: the value could \
                     reach a cell struct",
                    tcx.def_path_str(found)
                ),
                &[],
            );
        }
        // A method of a context trait called from the impl.
        for item_id in impl_.items {
            let impl_item = tcx.hir_impl_item(*item_id);
            let hir::ImplItemKind::Fn(_, body_id) = impl_item.kind else {
                continue;
            };
            let mut scan = ContextCallScan {
                cx,
                config,
                typeck: tcx.typeck_body(body_id),
                inert,
            };
            scan.visit_body(tcx.hir_body(body_id));
        }
    }
}

/// A `dyn` of a context trait inside `ty` (through generic arguments and the
/// fields of this crate's ADTs).
fn find_context_object<'tcx>(
    cx: &LateContext<'tcx>,
    config: &Config,
    ty: Ty<'tcx>,
) -> Option<DefId> {
    let tcx = cx.tcx;
    let mut seen: Vec<Ty<'tcx>> = Vec::new();
    let mut stack = vec![ty];
    while let Some(ty) = stack.pop() {
        if seen.contains(&ty) {
            continue;
        }
        seen.push(ty);
        match ty.kind() {
            ty::Dynamic(preds, ..) => {
                if let Some(principal) = preds.principal_def_id()
                    && reaches_listed_trait(cx, &config.context_traits, principal)
                {
                    return Some(principal);
                }
            },
            ty::Adt(adt, args) => {
                stack.extend(args.types());
                if adt.did().is_local() {
                    for field in adt.all_fields() {
                        stack.push(
                            tcx.type_of(field.did)
                                .instantiate(tcx, args)
                                .skip_normalization(),
                        );
                    }
                }
            },
            ty::Ref(_, inner, _)
            | ty::RawPtr(inner, _)
            | ty::Array(inner, _)
            | ty::Slice(inner) => {
                stack.push(*inner);
            },
            ty::Tuple(tys) => stack.extend(tys.iter()),
            _ => {},
        }
    }
    None
}

struct ContextCallScan<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    config: &'a Config,
    typeck: &'tcx TypeckResults<'tcx>,
    inert: Symbol,
}

impl<'tcx> ContextCallScan<'_, 'tcx> {
    fn check_callee(&self, expr: &Expr<'tcx>, def_id: DefId) {
        let tcx = self.cx.tcx;
        if let Some(trait_def_id) = tcx.trait_of_assoc(def_id)
            && reaches_listed_trait(self.cx, &self.config.context_traits, trait_def_id)
        {
            report(
                self.cx,
                expr.hir_id,
                expr.span,
                format!(
                    "impl of inert trait `{}` calls `{}`, a context trait method",
                    self.inert,
                    tcx.def_path_str(def_id)
                ),
                &[],
            );
        }
    }
}

impl<'tcx> Visitor<'tcx> for ContextCallScan<'_, 'tcx> {
    type NestedFilter = rustc_middle::hir::nested_filter::OnlyBodies;

    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.cx.tcx
    }

    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        match expr.kind {
            ExprKind::MethodCall(..) => {
                if let Some(def_id) = self.typeck.type_dependent_def_id(expr.hir_id) {
                    self.check_callee(expr, def_id);
                }
            },
            ExprKind::Call(callee, _) => {
                if let ExprKind::Path(qpath) = &callee.kind
                    && let Res::Def(DefKind::AssocFn, def_id) =
                        self.cx.qpath_res(qpath, callee.hir_id)
                {
                    self.check_callee(expr, def_id);
                }
            },
            _ => {},
        }
        intravisit::walk_expr(self, expr);
    }
}
