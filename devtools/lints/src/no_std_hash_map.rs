// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `no_std_hash_map`: use `shared_dsa::{UnorderedMap, UnorderedSet}` instead
//! of the standard (or hashbrown) hash map and set.
//!
//! `std::collections::HashMap` iterates in an order that depends on the
//! hasher's random seed, so any code that walks one is a determinism bug
//! waiting to happen in a VM whose output must agree across validators; it is
//! also SipHash, which is slower than needed for keys that are not
//! attacker-controlled. `shared_dsa::UnorderedMap` is a hash map (Fx by
//! default) with no ordered iteration exposed: the only way to walk it is the
//! explicitly named `into_iter_unordered`. Code that genuinely needs to
//! iterate in order uses `BTreeMap` / `BTreeSet`.
//!
//! This only removes accidental dependence on hash order. It does not make a
//! site correct: a `BTreeMap` is deterministic only if its key's `Ord` is the
//! same on every build, and the default Fx hasher is not collision resistant,
//! so attacker-chosen keys need a `BTreeMap` or a randomly keyed hasher. See
//! "Choosing a map" in `third_party/move/mono-move/AGENTS.md`.
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

    fn report(&self, cx: &LateContext<'_>, span: Span, what: Banned) {
        if self.reported_locals.iter().any(|l| l.contains(span)) {
            return;
        }
        span_lint_and_help(
            cx,
            NO_STD_HASH_MAP,
            span,
            format!("`{}` has nondeterministic iteration order", what.name()),
            None,
            "use `shared_dsa::UnorderedMap` / `UnorderedSet` (no iteration), or \
             `BTreeMap` / `BTreeSet` when ordered iteration is needed",
        );
    }
}

/// A collection the lint rejects.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Banned {
    StdHashMap,
    StdHashSet,
    HashbrownHashMap,
    HashbrownHashSet,
}

impl Banned {
    /// How the diagnostic names it, whatever the site wrote (an alias such as
    /// `FxHashMap` is reported as the std type it expands to).
    fn name(self) -> &'static str {
        match self {
            Banned::StdHashMap => "std::collections::HashMap",
            Banned::StdHashSet => "std::collections::HashSet",
            Banned::HashbrownHashMap => "hashbrown::HashMap",
            Banned::HashbrownHashSet => "hashbrown::HashSet",
        }
    }
}

/// Whether a definition is a banned collection. std's are recognised by
/// their diagnostic items, independent of how they were imported; hashbrown
/// has none, so it is matched by definition path.
fn find_banned_def(tcx: TyCtxt<'_>, def_id: DefId) -> Option<Banned> {
    if tcx.is_diagnostic_item(sym::HashMap, def_id) {
        return Some(Banned::StdHashMap);
    }
    if tcx.is_diagnostic_item(sym::HashSet, def_id) {
        return Some(Banned::StdHashSet);
    }
    match tcx.def_path_str(def_id).as_str() {
        "hashbrown::map::HashMap" | "hashbrown::HashMap" => Some(Banned::HashbrownHashMap),
        "hashbrown::set::HashSet" | "hashbrown::HashSet" => Some(Banned::HashbrownHashSet),
        _ => None,
    }
}

/// The banned collection mentioned anywhere inside a type, if any.
fn find_banned_in_ty<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Banned> {
    ty.walk().find_map(|arg| {
        let ty = arg.as_type()?;
        match ty.kind() {
            ty::Adt(adt, _) => find_banned_def(tcx, adt.did()),
            _ => None,
        }
    })
}

/// The banned collection a written path names, if any, looking through
/// type aliases (`FxHashMap<K, V>` is `HashMap<K, V, FxBuildHasher>`).
fn find_banned_in_path<'tcx>(
    cx: &LateContext<'tcx>,
    qpath: &QPath<'tcx>,
    hir_id: hir::HirId,
) -> Option<Banned> {
    match cx.qpath_res(qpath, hir_id) {
        Res::Def(hir::def::DefKind::TyAlias, def_id) => find_banned_in_ty(
            cx.tcx,
            cx.tcx
                .type_of(def_id)
                .instantiate_identity()
                .skip_normalization(),
        ),
        Res::Def(_, def_id) => find_banned_def(cx.tcx, def_id),
        _ => None,
    }
}

impl<'tcx> LateLintPass<'tcx> for NoStdHashMap {
    /// Types as written: signatures, fields, annotations, turbofish, aliases.
    fn check_ty(&mut self, cx: &LateContext<'tcx>, ty: &'tcx hir::Ty<'tcx, AmbigArg>) {
        if !self.applies(cx) {
            return;
        }
        if let hir::TyKind::Path(qpath) = &ty.kind
            && let Some(what) = find_banned_in_path(cx, qpath, ty.hir_id)
        {
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
                    find_banned_in_path(cx, qpath, ty.hir_id).map(|what| (ty.span, what))
                },
                _ => None,
            },
            None => find_banned_in_ty(cx.tcx, cx.typeck_results().pat_ty(local.pat))
                .map(|what| (local.span, what)),
        };
        if let Some((span, what)) = found {
            self.report(cx, span, what);
            self.reported_locals.push(local.span);
        }
    }
}
