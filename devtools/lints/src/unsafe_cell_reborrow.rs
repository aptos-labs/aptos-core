// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `unsafe_cell_reborrow`: a deliberately conservative check of how `&self`
//! methods reborrow through `UnsafeCell` fields ("cells").
//!
//! A struct whose methods take `&self` but need `&mut` access to some state
//! keeps it in an `UnsafeCell` field, `UnsafeCell<&mut T>` for a borrowed
//! component or `UnsafeCell<T>` for an owned one, and reborrows it inside each
//! method: `&mut **self.cell.get()` or `&mut *self.cell.get()`. That is sound
//! only if each cell has at most one live reborrow at any instant and no
//! reborrow outlives the activation that made it. The borrow checker cannot
//! see this, so this lint enforces a coarse discipline on every function in a
//! crate that defines such a struct. It is over-conservative by design: a
//! reborrow counts as held for the whole function, and anything the lint
//! cannot see is assumed to reborrow. The accepted shape is to reborrow once
//! in the entry point and pass the reference down to callees; a site that
//! needs something else opts out with `allow(unsafe_cell_reborrow)` and a
//! comment, and then owns the soundness argument.
//!
//! The rules:
//!
//! 1. Cell fields are private and appear only at a *site*, written
//!    `&mut **self.cell.get()` / `&mut *self.cell.get()` (or `&` for a shared
//!    reborrow).
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
//!
//! Layout of this file: the cells and the shared helpers; the scan of one
//! body (rules 1 and 2); the crate-wide analysis over the call graph (rules 3
//! to 5); the inert impls (rule 6).

use crate::scope::Scope;
use clippy_utils::{
    diagnostics::span_lint_hir_and_then,
    get_parent_expr,
    visitors::{for_each_expr, for_each_expr_without_closures},
};
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::{
    self as hir,
    attrs::lang_items::LangItem,
    def::{DefKind, Res},
    def_id::{DefId, LocalDefId},
    BorrowKind, Expr, ExprKind, HirId, UnOp,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Instance, InstanceKind, Ty, TyCtxt, TypeckResults, TypingEnv};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::{Span, Symbol};
use serde::Deserialize;
use std::{collections::BTreeSet, ops::ControlFlow};

declare_lint! {
    /// ### What it does
    ///
    /// Checks the discipline for structs that mutate through `UnsafeCell`
    /// fields from `&self` methods: one reborrow per cell per function, no
    /// call that may reborrow a held cell, no escape of a reborrow, and no
    /// call into unseen code while a cell is held.
    ///
    /// ### Why is this bad?
    ///
    /// Two live reborrows of the same cell, or a reborrow that outlives its
    /// function, are undefined behaviour that compiles and runs. The borrow
    /// checker cannot see through the cell, so the rules must be checked
    /// here. The check is deliberately conservative; a site that needs more
    /// opts out and owns the argument.
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
    /// Use instead: reborrow once in the entry point and pass the reference
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
    "conservative check of reborrows through `UnsafeCell` fields"
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
            Analysis {
                cx,
                config: &self.config,
                cells,
                bodies: FxHashMap::default(),
                transitive: FxHashMap::default(),
            }
            .run();
        }
        check_inert_impls(cx, &self.config);
    }
}

/// What every finding says about the check and the two ways out.
const HELP: &str =
    "this is a deliberately conservative check of `UnsafeCell` reborrows (the rules \
    are in devtools/lints/README.md); either reborrow once at the entry point and pass the \
    reference down to callees, or opt out with `allow(unsafe_cell_reborrow)` plus a comment and \
    own the soundness argument yourself";

fn report(cx: &LateContext<'_>, hir_id: HirId, span: Span, msg: String, notes: &[(Span, String)]) {
    span_lint_hir_and_then(cx, UNSAFE_CELL_REBORROW, hir_id, span, msg, |diag| {
        for (span, note) in notes {
            diag.span_note(*span, note.clone());
        }
        diag.help(HELP);
    });
}

fn type_of<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId) -> Ty<'tcx> {
    tcx.type_of(def_id)
        .instantiate_identity()
        .skip_normalization()
}

/// Whether `list` names `def_id` by its full path (crate name included).
fn is_listed(cx: &LateContext<'_>, list: &[String], def_id: DefId) -> bool {
    let path = cx
        .get_def_path(def_id)
        .iter()
        .map(Symbol::as_str)
        .collect::<Vec<_>>()
        .join("::");
    list.iter().any(|l| *l == path)
}

/// Whether `trait_def_id` is, or has as a supertrait, a trait in `list`.
fn reaches_listed_trait(cx: &LateContext<'_>, list: &[String], trait_def_id: DefId) -> bool {
    ty::elaborate::supertrait_def_ids(cx.tcx, trait_def_id).any(|t| is_listed(cx, list, t))
}

// ---------------------------------------------------------------------------
// Cells
// ---------------------------------------------------------------------------

/// One `UnsafeCell` field.
struct Cell<'tcx> {
    /// The struct that owns the field.
    owner: DefId,
    field: Symbol,
    /// What a reborrow yields: the `T` of `UnsafeCell<&mut T>` or of
    /// `UnsafeCell<T>`.
    target: Ty<'tcx>,
    /// Derefs between `.get()` and the reborrow: two through a stored
    /// reference (`&mut **`), one for an owned value (`&mut *`).
    derefs: usize,
}

type CellIdx = usize;

fn cell_name(tcx: TyCtxt<'_>, cell: &Cell<'_>) -> String {
    format!("`{}.{}`", tcx.item_name(cell.owner), cell.field)
}

/// Every `UnsafeCell` field of every struct in the crate (rule 1's privacy
/// check happens here too).
fn find_cells<'tcx>(cx: &LateContext<'tcx>) -> Vec<Cell<'tcx>> {
    let tcx = cx.tcx;
    let mut cells = Vec::new();
    for id in tcx.hir_crate_items(()).free_items() {
        let item = tcx.hir_item(id);
        let hir::ItemKind::Struct(_, _, data) = item.kind else {
            continue;
        };
        for field in data.fields() {
            let ty::Adt(adt, args) = type_of(tcx, field.def_id.to_def_id()).kind() else {
                continue;
            };
            if !tcx.is_lang_item(adt.did(), LangItem::UnsafeCell) {
                continue;
            }
            let content = args.type_at(0);
            let (target, derefs) = match content.kind() {
                ty::Ref(_, target, _) => (*target, 2),
                _ => (content, 1),
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
                target,
                derefs,
            });
        }
    }
    cells
}

fn is_cell_struct(cells: &[Cell<'_>], ty: Ty<'_>) -> bool {
    matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if cells.iter().any(|c| c.owner == adt.did()))
}

/// A type that mentions a cell struct anywhere: `&S`, `Option<&S>`, a closure
/// capturing one, ...
fn mentions_cell_struct<'tcx>(cells: &[Cell<'tcx>], ty: Ty<'tcx>) -> bool {
    ty.walk()
        .any(|arg| arg.as_type().is_some_and(|ty| is_cell_struct(cells, ty)))
}

/// Visits `ty` and everything a value of it can contain (through references,
/// generic arguments, the fields of this crate's own ADTs, closure captures,
/// tuples and arrays), stopping at the first `Some` from `f`. A cell struct is
/// not entered: its fields are only reachable at a site.
fn find_in_ty<'tcx, R>(
    tcx: TyCtxt<'tcx>,
    cells: &[Cell<'tcx>],
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
            ty::Ref(_, inner, _)
            | ty::RawPtr(inner, _)
            | ty::Array(inner, _)
            | ty::Slice(inner) => {
                stack.push(*inner);
            },
            ty::Adt(adt, args) => {
                stack.extend(args.types());
                if adt.did().is_local() && !is_cell_struct(cells, ty) {
                    stack.extend(adt.all_fields().map(|field| {
                        tcx.type_of(field.did)
                            .instantiate(tcx, args)
                            .skip_normalization()
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

/// A site is exactly `&mut <*…*><field>.get()` or `&<*…*><field>.get()`
/// with `derefs` stars: the span of the whole reborrow if `field_expr` is
/// used that way.
fn site_span<'tcx>(cx: &LateContext<'tcx>, field_expr: &Expr<'tcx>, derefs: usize) -> Option<Span> {
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
    (inner.hir_id == place.hir_id).then_some(borrow.span)
}

// ---------------------------------------------------------------------------
// One body (rules 1 and 2)
// ---------------------------------------------------------------------------

/// What a call may be, as far as the lint can tell.
enum Callee {
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
    fn describe(&self, tcx: TyCtxt<'_>) -> String {
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

fn local_or_external(def_id: DefId) -> Callee {
    match def_id.as_local() {
        Some(local) => Callee::Local(local),
        None => Callee::External(def_id),
    }
}

struct Call {
    hir_id: HirId,
    span: Span,
    callee: Callee,
    /// The receiver or an argument can reach a cell struct.
    tainted: bool,
    /// The receiver is a field of a cell struct (`self.loader.f()`).
    through_field: bool,
    /// Closures passed as arguments.
    closure_args: Vec<LocalDefId>,
}

#[derive(Default)]
struct BodyInfo {
    /// The one reborrow site per cell (rule 2), with its span.
    sites: Vec<(CellIdx, Span)>,
    calls: Vec<Call>,
    /// Closures defined in this body; each is a body of its own.
    closures: Vec<LocalDefId>,
}

struct BodyScan<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    cells: &'a [Cell<'tcx>],
    typeck: &'tcx TypeckResults<'tcx>,
    typing_env: TypingEnv<'tcx>,
    info: BodyInfo,
}

/// Scans one body, reporting rules 1 and 2 on the way. Closures inside it are
/// scanned as bodies of their own and linked through `BodyInfo::closures`.
fn scan_body<'tcx>(cx: &LateContext<'tcx>, cells: &[Cell<'tcx>], owner: LocalDefId) -> BodyInfo {
    let tcx = cx.tcx;
    let body = tcx.hir_body_owned_by(owner);
    let mut scan = BodyScan {
        cx,
        cells,
        typeck: tcx.typeck_body(body.id()),
        typing_env: TypingEnv::post_analysis(tcx, owner),
        info: BodyInfo::default(),
    };
    for_each_expr_without_closures(body, |expr| {
        scan.visit(expr);
        ControlFlow::<()>::Continue(())
    });
    scan.info
}

impl<'tcx> BodyScan<'_, 'tcx> {
    fn visit(&mut self, expr: &'tcx Expr<'tcx>) {
        match expr.kind {
            ExprKind::Field(base, ident) => {
                let Some(cell) = self.cell_field(base, ident.name) else {
                    return;
                };
                let name = cell_name(self.cx.tcx, &self.cells[cell]);
                match site_span(self.cx, expr, self.cells[cell].derefs) {
                    None => report(
                        self.cx,
                        expr.hir_id,
                        expr.span,
                        format!(
                            "{name} used outside a reborrow site; the only permitted form is \
                             `&mut {stars}self.{}.get()` (or `&{stars}`)",
                            self.cells[cell].field,
                            stars = "*".repeat(self.cells[cell].derefs),
                        ),
                        &[],
                    ),
                    Some(span) => match self.info.sites.iter().find(|(c, _)| *c == cell) {
                        Some((_, first)) => report(
                            self.cx,
                            expr.hir_id,
                            span,
                            format!("{name} is reborrowed more than once in this function"),
                            &[(*first, "first reborrowed here".to_string())],
                        ),
                        None => self.info.sites.push((cell, span)),
                    },
                }
            },
            ExprKind::MethodCall(segment, receiver, args, _) => {
                // The `.get()` of a site is not a call of interest.
                let on_cell = matches!(receiver.kind, ExprKind::Field(base, ident)
                    if self.cell_field(base, ident.name).is_some());
                if segment.ident.as_str() == "get" && on_cell {
                    return;
                }
                let callee = match self.typeck.type_dependent_def_id(expr.hir_id) {
                    Some(def_id) => self.resolve(def_id, self.typeck.node_args_opt(expr.hir_id)),
                    None => Callee::Value,
                };
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
            ExprKind::Closure(closure) => self.info.closures.push(closure.def_id),
            _ => {},
        }
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

    /// A call through a value: `f(x)` with `f` a closure, a function item, or
    /// something opaque.
    fn value_callee(&self, callee: &Expr<'tcx>) -> Callee {
        match self.typeck.expr_ty_adjusted(callee).peel_refs().kind() {
            ty::Closure(def_id, _) => local_or_external(*def_id),
            ty::FnDef(def_id, args) => self.resolve(*def_id, Some(args.skip_binder())),
            _ => Callee::Value,
        }
    }

    /// Resolves a callee to the body that will run, as far as the receiver
    /// type pins it down. `args` is `None` when typeck recorded none, which
    /// it skips for a callee without generics; a trait item needs at least
    /// its `Self` argument to resolve.
    fn resolve(&self, def_id: DefId, args: Option<ty::GenericArgsRef<'tcx>>) -> Callee {
        let tcx = self.cx.tcx;
        let is_trait_item = tcx.trait_of_assoc(def_id).is_some();
        let Some(args) = args.filter(|args| !(is_trait_item && args.is_empty())) else {
            return local_or_external(def_id);
        };
        match Instance::try_resolve(tcx, self.typing_env, def_id, args) {
            Ok(Some(instance)) if !matches!(instance.def, InstanceKind::Virtual(..)) => {
                local_or_external(instance.def_id())
            },
            // A trait object, or a type parameter the body does not pin down.
            _ if is_trait_item => Callee::Dynamic(def_id),
            _ => local_or_external(def_id),
        }
    }

    fn push_call(
        &mut self,
        expr: &Expr<'tcx>,
        callee: Callee,
        receiver: Option<&Expr<'tcx>>,
        args: &[Expr<'tcx>],
    ) {
        let tainted = receiver
            .into_iter()
            .chain(args)
            .any(|e| mentions_cell_struct(self.cells, self.typeck.expr_ty_adjusted(e)));
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
            tainted,
            through_field,
            closure_args,
        });
    }

    /// `self.field` (through any `&` / `*`), with `self` a cell struct.
    fn is_field_of_cell_struct(&self, mut expr: &Expr<'tcx>) -> bool {
        loop {
            match expr.kind {
                ExprKind::AddrOf(_, _, inner) | ExprKind::Unary(UnOp::Deref, inner) => expr = inner,
                ExprKind::Field(base, _) => {
                    return is_cell_struct(self.cells, self.typeck.expr_ty_adjusted(base));
                },
                _ => return false,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The crate (rules 3 to 5)
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

impl<'tcx> Analysis<'_, 'tcx> {
    fn run(mut self) {
        let owners: Vec<LocalDefId> = self.cx.tcx.hir_body_owners().collect();
        for &owner in &owners {
            let info = scan_body(self.cx, &self.cells, owner);
            self.bodies.insert(owner, info);
        }
        self.compute_transitive();
        for &owner in &owners {
            self.check_calls(&self.bodies[&owner]);
            self.check_signature(owner);
        }
    }

    /// Fixpoint over the call graph restricted to this crate.
    fn compute_transitive(&mut self) {
        for (owner, info) in &self.bodies {
            let own = info.sites.iter().map(|(cell, _)| *cell).collect();
            self.transitive.insert(*owner, own);
        }
        loop {
            let mut changed = false;
            for (owner, info) in &self.bodies {
                let mut set = self.transitive[owner].clone();
                let callees = info.calls.iter().filter_map(|c| match c.callee {
                    Callee::Local(def_id) => Some(def_id),
                    _ => None,
                });
                for callee in info.closures.iter().copied().chain(callees) {
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

    fn reaches_cells(&self, body: LocalDefId) -> bool {
        self.transitive
            .get(&body)
            .is_some_and(|set| !set.is_empty())
    }

    fn captures_cell_struct(&self, closure: LocalDefId) -> bool {
        let ty::Closure(_, args) = type_of(self.cx.tcx, closure.to_def_id()).kind() else {
            return false;
        };
        args.as_closure()
            .upvar_tys()
            .iter()
            .any(|ty| mentions_cell_struct(&self.cells, ty))
    }

    /// Rules 3 and 4 for every call of one body.
    fn check_calls(&self, info: &BodyInfo) {
        let tcx = self.cx.tcx;
        let held = &info.sites;
        let held_names = || {
            held.iter()
                .map(|(cell, _)| cell_name(tcx, &self.cells[*cell]))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let held_notes = || {
            held.iter()
                .map(|(cell, span)| {
                    (
                        *span,
                        format!("{} is reborrowed here", cell_name(tcx, &self.cells[*cell])),
                    )
                })
                .collect::<Vec<_>>()
        };
        for call in &info.calls {
            // Rule 3: a closure that can reach the struct or a cell goes nowhere.
            let bad_closure = call
                .closure_args
                .iter()
                .any(|c| self.reaches_cells(*c) || self.captures_cell_struct(*c));
            if bad_closure {
                report(
                    self.cx,
                    call.hir_id,
                    call.span,
                    "a closure that can reach a cell struct is passed as an argument; call it \
                     directly or pass the components it needs instead"
                        .to_string(),
                    &[],
                );
                continue;
            }
            if held.is_empty() {
                continue;
            }
            let problem = match &call.callee {
                Callee::Local(callee) => {
                    let overlap = held
                        .iter()
                        .filter(|(cell, _)| {
                            self.transitive
                                .get(callee)
                                .is_some_and(|s| s.contains(cell))
                        })
                        .map(|(cell, _)| cell_name(tcx, &self.cells[*cell]))
                        .collect::<Vec<_>>();
                    (!overlap.is_empty()).then(|| {
                        format!(
                            "{} may reborrow {}, which this function already holds",
                            call.callee.describe(tcx),
                            overlap.join(", ")
                        )
                    })
                },
                Callee::External(_) if call.tainted => Some(format!(
                    "a cell struct reaches {}, outside this crate, while {} is held",
                    call.callee.describe(tcx),
                    held_names()
                )),
                Callee::Dynamic(method) if !call.through_field => {
                    let inert = tcx
                        .trait_of_assoc(*method)
                        .is_some_and(|t| is_listed(self.cx, &self.config.inert_traits, t));
                    (!inert).then(|| {
                        format!(
                            "{} is called on a type parameter or trait object while {} is held; \
                             the callee may reach the struct through its caller",
                            call.callee.describe(tcx),
                            held_names()
                        )
                    })
                },
                Callee::Value if !call.through_field => Some(format!(
                    "a call through a function value while {} is held; the callee may reach the \
                     struct through its caller",
                    held_names()
                )),
                _ => None,
            };
            if let Some(problem) = problem {
                report(self.cx, call.hir_id, call.span, problem, &held_notes());
            }
        }
    }

    /// Rule 5: a reborrow must not be able to leave through a signature.
    /// Applies to every method of a cell struct (its public API) and to any
    /// other function or closure with a site. A plain `&mut T` parameter is
    /// fine, it cannot store a reference; a constructor is exempt, it takes
    /// the components in.
    fn check_signature(&self, owner: LocalDefId) {
        let tcx = self.cx.tcx;
        let def_id = owner.to_def_id();
        let is_method =
            tcx.def_kind(def_id) == DefKind::AssocFn && tcx.associated_item(def_id).is_method();
        let of_cell_struct = tcx
            .impl_of_assoc(def_id)
            .is_some_and(|impl_id| mentions_cell_struct(&self.cells, type_of(tcx, impl_id)));
        if !(is_method && of_cell_struct) && self.bodies[&owner].sites.is_empty() {
            return;
        }
        let (inputs, output) = match tcx.def_kind(def_id) {
            DefKind::Fn | DefKind::AssocFn => {
                let sig = tcx.fn_sig(def_id).skip_binder().skip_binder();
                (sig.inputs().to_vec(), sig.output())
            },
            DefKind::Closure => {
                let ty::Closure(_, args) = type_of(tcx, def_id).kind() else {
                    return;
                };
                let sig = args.as_closure().sig().skip_binder();
                (sig.inputs().to_vec(), sig.output())
            },
            _ => return,
        };
        let mut problems = Vec::new();
        for input in inputs.into_iter().skip(usize::from(is_method)) {
            if matches!(input.kind(), ty::Ref(_, inner, _) if self.is_target(*inner)) {
                continue;
            }
            if let Some(what) = self.holds_target_ref(input) {
                problems.push(format!("a parameter can hold {what}"));
            }
        }
        if let Some(what) = self.holds_target_ref(output) {
            problems.push(format!("the return type can hold {what}"));
        }
        for problem in problems {
            report(
                self.cx,
                tcx.local_def_id_to_hir_id(owner),
                tcx.def_span(owner),
                format!(
                    "in {}, {problem}, so a reborrow can escape",
                    Callee::Local(owner).describe(tcx)
                ),
                &[],
            );
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

    /// If a value of `ty` can hold a reference to a cell target, what it is.
    /// A trait object or an opaque type is assumed able to.
    fn holds_target_ref(&self, ty: Ty<'tcx>) -> Option<String> {
        let tcx = self.cx.tcx;
        find_in_ty(tcx, &self.cells, ty, |ty| match ty.kind() {
            ty::Ref(_, inner, _) | ty::RawPtr(inner, _) if self.is_target(inner.peel_refs()) => {
                Some(format!("a reference to `{}`", inner.peel_refs()))
            },
            ty::Dynamic(..) => Some("a trait object".to_string()),
            ty::Alias(_, alias) if matches!(alias.kind(tcx), ty::AliasTyKind::Opaque { .. }) => {
                Some("an opaque type".to_string())
            },
            _ => None,
        })
    }
}

// ---------------------------------------------------------------------------
// Inert impls (rule 6)
// ---------------------------------------------------------------------------

/// Every impl of an inert trait in the crate: its self type must not be able
/// to reach a context trait, through a bounded type parameter or a trait
/// object, and its methods must not call one.
fn check_inert_impls<'tcx>(cx: &LateContext<'tcx>, config: &Config) {
    if config.inert_traits.is_empty() || config.context_traits.is_empty() {
        return;
    }
    let tcx = cx.tcx;
    let reaches_context =
        |trait_def_id| reaches_listed_trait(cx, &config.context_traits, trait_def_id);
    for id in tcx.hir_crate_items(()).free_items() {
        let item = tcx.hir_item(id);
        let hir::ItemKind::Impl(impl_) = item.kind else {
            continue;
        };
        let Some(trait_def_id) = impl_.of_trait.and_then(|h| h.trait_ref.trait_def_id()) else {
            continue;
        };
        if !is_listed(cx, &config.inert_traits, trait_def_id) {
            continue;
        }
        let impl_def_id = item.owner_id.to_def_id();
        let inert = tcx.item_name(trait_def_id);

        for (clause, span) in tcx.explicit_clauses_of(impl_def_id).clauses {
            if let Some(pred) = clause.as_trait_clause()
                && reaches_context(pred.def_id())
            {
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
        let object = find_in_ty(tcx, &[], type_of(tcx, impl_def_id), |ty| match ty.kind() {
            ty::Dynamic(preds, ..) => preds.principal_def_id().filter(|p| reaches_context(*p)),
            _ => None,
        });
        if let Some(found) = object {
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
        for item_id in impl_.items {
            let hir::ImplItemKind::Fn(_, body_id) = tcx.hir_impl_item(*item_id).kind else {
                continue;
            };
            let typeck = tcx.typeck_body(body_id);
            for_each_expr(tcx, tcx.hir_body(body_id), |expr| {
                let callee = match expr.kind {
                    ExprKind::MethodCall(..) => typeck.type_dependent_def_id(expr.hir_id),
                    ExprKind::Call(callee, _) => match callee.kind {
                        ExprKind::Path(ref qpath) => match cx.qpath_res(qpath, callee.hir_id) {
                            Res::Def(DefKind::AssocFn, def_id) => Some(def_id),
                            _ => None,
                        },
                        _ => None,
                    },
                    _ => None,
                };
                if let Some(def_id) = callee
                    && let Some(trait_def_id) = tcx.trait_of_assoc(def_id)
                    && reaches_context(trait_def_id)
                {
                    report(
                        cx,
                        expr.hir_id,
                        expr.span,
                        format!(
                            "impl of inert trait `{inert}` calls `{}`, a context trait method",
                            tcx.def_path_str(def_id)
                        ),
                        &[],
                    );
                }
                ControlFlow::<()>::Continue(())
            });
        }
    }
}
