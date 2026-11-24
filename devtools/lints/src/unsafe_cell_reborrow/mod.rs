// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `unsafe_cell_reborrow`: an opt-in, deliberately conservative check of
//! structs whose `&self` methods get `&mut` access through `UnsafeCell` fields.
//!
//! It checks that borrows of such a field take turns: while a function has
//! a reference borrowed from the field, nothing it calls, directly or through
//! code the lint cannot see, can borrow the same field again.
//!
//! The check, as currently implemented, is not a proof: it has known gaps.
//!
//! The background, the terms, the specific rules and the limitations are in
//! `devtools/lints/docs/unsafe_cell_reborrow.md`.
//!
//! The lint runs in passes over one crate, driven by `check_crate` below:
//!
//! 1. `cells`: find the cells.
//! 2. `body`: scan every function and closure body for its borrows and
//!    calls, and check each borrow (`borrow_site`) and what the body drops
//!    (`drops`).
//! 3. `may_borrow`: compute the cells a call to each function may borrow.
//! 4. `calls` and `signature`: check every call and every signature against
//!    that.
//!
//! `types` and `util` hold the shared helpers.

// TODO(correctness): the lint has known gaps, where it accepts code that
// could borrow a cell twice. They are listed under "Limitations" in the doc.
//
// TODO(cleanup): a function that holds a cell and takes `&mut T`, where `T`
// is what a cell holds, is rejected if `T` contains a trait object or a
// generic. Decide whether that is wanted; no test covers it.

mod body;
mod borrow_site;
mod calls;
mod cells;
mod drops;
mod may_borrow;
mod signature;
mod types;
mod util;

use crate::scope::Scope;
use body::BodyInfo;
use cells::{CellIdx, Cells};
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::{
    def_id::{LocalDefId, CRATE_DEF_ID},
    CRATE_HIR_ID,
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint, impl_lint_pass};
use serde::Deserialize;
use std::collections::BTreeSet;

declare_lint! {
    /// ### What it does
    ///
    /// For the structs listed in the configuration, checks the discipline
    /// for mutating through `UnsafeCell` fields from `&self` methods: one
    /// reborrow per cell per function, no call that may reborrow a held
    /// cell, no escape of a reborrow, and no call into unseen code that
    /// could reach the struct while a cell is held.
    ///
    /// ### Why is this bad?
    ///
    /// Two live reborrows of the same cell, or a reborrow that outlives its
    /// function, are undefined behaviour that compiles and runs. The borrow
    /// checker cannot see through the cell, so the rules must be checked
    /// here. The check is opt-in per struct and deliberately conservative; a
    /// site that needs more opts out and owns the argument.
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
    "conservative check of reborrows through `UnsafeCell` fields",
    // A call hidden in another crate's macro (`format!`, `?`) is still a call.
    report_in_external_macro
}

/// Configuration, from `[aptos_lints.unsafe_cell_reborrow]` in `dylint.toml`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    #[serde(flatten)]
    pub scope: Scope,
    /// The structs (full paths) the check applies to. Every `UnsafeCell`
    /// field of a listed struct is a cell; no other struct is looked at.
    pub structs: Vec<String>,
    /// Functions (full paths) that tie a borrow to a local anchor. Every
    /// borrow of a cell must be the second argument of a call to one.
    pub anchor_fns: Vec<String>,
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
        let config = &self.config;
        if !config.scope.applies(cx) {
            return;
        }

        // Pass 1: find the cells.
        let cells = cells::find_cells(cx, config);
        if !cells.list.is_empty() {
            // Pass 2: scan every body.
            let owners: Vec<LocalDefId> = cx.tcx.hir_body_owners().collect();
            let bodies: FxHashMap<LocalDefId, BodyInfo<'tcx>> = owners
                .iter()
                .map(|&owner| (owner, body::scan_body(cx, config, &cells, owner)))
                .collect();

            // Pass 3: what a call to each body may borrow.
            let may_borrow = may_borrow::compute(&cells, &bodies);

            // Pass 4: check every call and signature.
            let analysis = Analysis {
                cx,
                cells: &cells,
                bodies: &bodies,
                may_borrow: &may_borrow,
            };
            for &owner in &owners {
                analysis.check_calls(&bodies[&owner]);
                analysis.check_signature(owner);
            }

            // A type the lint gave up on was treated as the worst case; say so.
            for ty in cells.too_large.borrow().iter() {
                util::report(
                    cx,
                    CRATE_HIR_ID,
                    cx.tcx.def_span(CRATE_DEF_ID).shrink_to_lo(),
                    format!(
                        "the type `{ty}` is too large for this lint to analyse; it is treated as \
                         if it could reach a cell struct and contain a trait object"
                    ),
                    &[],
                );
            }
        }
    }
}

/// What the checks of pass 4 read: the cells, the scan of every body, and
/// the cells a call to each body may borrow.
struct Analysis<'a, 'tcx> {
    cx: &'a LateContext<'tcx>,
    cells: &'a Cells<'tcx>,
    bodies: &'a FxHashMap<LocalDefId, BodyInfo<'tcx>>,
    may_borrow: &'a FxHashMap<LocalDefId, BTreeSet<CellIdx>>,
}
