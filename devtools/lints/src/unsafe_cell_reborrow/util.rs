// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Diagnostics, and matching items against the paths in the configuration.

use super::UNSAFE_CELL_REBORROW;
use clippy_utils::diagnostics::span_lint_hir_and_then;
use rustc_hir::{self as hir, def_id::DefId, HirId};
use rustc_lint::LateContext;
use rustc_span::{Span, Symbol};

/// What every finding says about the check and the two ways out.
pub(super) const HELP: &str =
    "this is a deliberately conservative check of `UnsafeCell` reborrows (the rules are in \
    devtools/lints/docs/unsafe_cell_reborrow.md); either reborrow once at the entry point and \
    pass the reference down to callees, or opt out with `allow(unsafe_cell_reborrow)` plus a \
    comment and own the soundness argument yourself";

pub(super) fn report(
    cx: &LateContext<'_>,
    hir_id: HirId,
    span: Span,
    msg: String,
    notes: &[(Span, String)],
) {
    span_lint_hir_and_then(cx, UNSAFE_CELL_REBORROW, hir_id, span, msg, |diag| {
        for (span, note) in notes {
            diag.span_note(*span, note.clone());
        }
        diag.help(HELP);
    });
}

/// A finding about the `structs` list itself.
pub(super) fn report_config(cx: &LateContext<'_>, span: Span, msg: String) {
    span_lint_hir_and_then(
        cx,
        UNSAFE_CELL_REBORROW,
        hir::CRATE_HIR_ID,
        span,
        msg,
        |diag| {
            diag.help(
                "fix the `structs` list of `[aptos_lints.unsafe_cell_reborrow]` in dylint.toml",
            );
        },
    );
}

/// The full path of `def_id`, crate name included.
pub(super) fn full_path(cx: &LateContext<'_>, def_id: DefId) -> String {
    cx.get_def_path(def_id)
        .iter()
        .map(Symbol::as_str)
        .collect::<Vec<_>>()
        .join("::")
}

/// Whether `list` names `def_id` by its full path.
pub(super) fn is_listed(cx: &LateContext<'_>, list: &[String], def_id: DefId) -> bool {
    list.contains(&full_path(cx, def_id))
}
