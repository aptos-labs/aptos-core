// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `no_std_hash_map`: use `shared_dsa::{UnorderedMap, UnorderedSet}` instead
//! of the standard (or hashbrown) hash map and set.
//!
//! `std::collections::HashMap` iterates in an order that depends on the
//! hasher's random seed, so any code that walks one is a determinism bug
//! waiting to happen in a VM whose output must agree across validators; it is
//! also SipHash, which is slower than needed for keys that are not
//! attacker-controlled. `shared_dsa::UnorderedMap` is `FxHashMap` with no
//! iteration over keys or pairs exposed at all. Code that genuinely needs to
//! iterate uses `BTreeMap` / `BTreeSet`.
//!
//! Flags the type wherever it is written — signatures, fields, annotations,
//! turbofish, and the self type of a constructor call such as
//! `HashMap::new()` — and locals whose inferred type is a hash map. Type
//! aliases such as `rustc_hash::FxHashMap` are resolved, so they are flagged
//! too. One `let` yields one diagnostic.

use crate::scope::Scope;
use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{self as hir, def::Res, def_id::DefId, AmbigArg, QPath};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::{sym, Span};

declare_lint! {
    /// ### What it does
    ///
    /// Flags `std::collections::{HashMap, HashSet}` and `hashbrown`'s
    /// equivalents (including through aliases such as `FxHashMap`).
    ///
    /// ### Why is this bad?
    ///
    /// Their iteration order is nondeterministic, which is a consensus bug in
    /// VM code, and SipHash is slower than needed. `shared_dsa::UnorderedMap`
    /// / `UnorderedSet` expose no iteration; `BTreeMap` / `BTreeSet` iterate
    /// in a defined order.
    ///
    /// ### Example
    ///
    /// ```rust,ignore
    /// let cache: std::collections::HashMap<Key, Value> = HashMap::new();
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust,ignore
    /// let cache: shared_dsa::UnorderedMap<Key, Value> = UnorderedMap::new();
    /// ```
    pub NO_STD_HASH_MAP,
    Deny,
    "use of std/hashbrown HashMap or HashSet where shared_dsa::UnorderedMap / UnorderedSet is required"
}

pub struct NoStdHashMap {
    scope: Scope,
    /// Decided once per crate.
    applies: Option<bool>,
    /// Spans of `let` statements already reported, so the constructor call
    /// inside one is not reported a second time.
    reported_locals: Vec<Span>,
}

impl_lint_pass!(NoStdHashMap => [NO_STD_HASH_MAP]);

impl NoStdHashMap {
    pub fn new(scope: Scope) -> Self {
        Self {
            scope,
            applies: None,
            reported_locals: Vec::new(),
        }
    }

    fn applies(&mut self, cx: &LateContext<'_>) -> bool {
        *self.applies.get_or_insert_with(|| self.scope.applies(cx))
    }

    fn report(&self, cx: &LateContext<'_>, span: Span, what: &str) {
        if self.reported_locals.iter().any(|l| l.contains(span)) {
            return;
        }
        span_lint_and_help(
            cx,
            NO_STD_HASH_MAP,
            span,
            format!("`{what}` has nondeterministic iteration order"),
            None,
            "use `shared_dsa::UnorderedMap` / `UnorderedSet` (no iteration), or \
             `BTreeMap` / `BTreeSet` when ordered iteration is needed",
        );
    }
}

/// The banned collection a definition is, if any.
fn banned_def(tcx: TyCtxt<'_>, def_id: DefId) -> Option<&'static str> {
    if tcx.is_diagnostic_item(sym::HashMap, def_id) {
        return Some("std::collections::HashMap");
    }
    if tcx.is_diagnostic_item(sym::HashSet, def_id) {
        return Some("std::collections::HashSet");
    }
    let path = tcx.def_path_str(def_id);
    match path.as_str() {
        "hashbrown::map::HashMap" | "hashbrown::HashMap" => Some("hashbrown::HashMap"),
        "hashbrown::set::HashSet" | "hashbrown::HashSet" => Some("hashbrown::HashSet"),
        _ => None,
    }
}

/// The banned collection mentioned anywhere inside a type, if any.
fn banned_in_ty<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<&'static str> {
    ty.walk().find_map(|arg| {
        let ty = arg.as_type()?;
        match ty.kind() {
            ty::Adt(adt, _) => banned_def(tcx, adt.did()),
            _ => None,
        }
    })
}

/// The banned collection a written type path names, if any. Resolves type
/// aliases (`FxHashMap<K, V>` is `HashMap<K, V, FxBuildHasher>`).
fn banned_in_hir_ty<'tcx>(cx: &LateContext<'tcx>, ty: &hir::Ty<'tcx, AmbigArg>) -> Option<&'static str> {
    let hir::TyKind::Path(qpath) = &ty.kind else {
        return None;
    };
    banned_in_qpath(cx, qpath, ty.hir_id)
}

fn banned_in_qpath<'tcx>(
    cx: &LateContext<'tcx>,
    qpath: &QPath<'tcx>,
    hir_id: hir::HirId,
) -> Option<&'static str> {
    match cx.qpath_res(qpath, hir_id) {
        Res::Def(hir::def::DefKind::TyAlias, def_id) => {
            banned_in_ty(cx.tcx, cx.tcx.type_of(def_id).instantiate_identity().skip_normalization())
        },
        Res::Def(_, def_id) => banned_def(cx.tcx, def_id),
        _ => None,
    }
}

impl<'tcx> LateLintPass<'tcx> for NoStdHashMap {
    /// Types as written: signatures, fields, annotations, turbofish, aliases.
    fn check_ty(&mut self, cx: &LateContext<'tcx>, ty: &'tcx hir::Ty<'tcx, AmbigArg>) {
        if !self.applies(cx) {
            return;
        }
        if let Some(what) = banned_in_hir_ty(cx, ty) {
            self.report(cx, ty.span, what);
        }
    }

    /// A `let`: report its annotation if written, else its inferred type, and
    /// claim the whole statement so the initializer is not reported again.
    fn check_local(&mut self, cx: &LateContext<'tcx>, local: &'tcx hir::LetStmt<'tcx>) {
        if !self.applies(cx) {
            return;
        }
        let found = match local.ty {
            Some(ty) => match &ty.kind {
                hir::TyKind::Path(qpath) => {
                    banned_in_qpath(cx, qpath, ty.hir_id).map(|what| (ty.span, what))
                },
                _ => None,
            },
            None => banned_in_ty(cx.tcx, cx.typeck_results().pat_ty(local.pat))
                .map(|what| (local.span, what)),
        };
        if let Some((span, what)) = found {
            self.report(cx, span, what);
            self.reported_locals.push(local.span);
        }
    }
}
