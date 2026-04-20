// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lint that warns on user-written code that is definitely unreachable.
//!
//! Prerequisite: `ReachableStateAnnotation`, produced by `UnreachableCodeProcessor`
//! (registered in the lint pipeline as a prereq, alongside live variable analysis).

use move_binary_format::file_format::CodeOffset;
use move_compiler_v2::{
    external_checks::StacklessBytecodeChecker,
    pipeline::unreachable_code_analysis::ReachableStateAnnotation,
};
use move_model::model::Loc;
use move_stackless_bytecode::function_target::FunctionTarget;
use std::collections::BTreeSet;

pub struct UnreachableCode {}

impl StacklessBytecodeChecker for UnreachableCode {
    fn get_name(&self) -> String {
        "unreachable_code".to_string()
    }

    fn check(&self, target: &FunctionTarget) {
        let annotation = target
            .get_annotations()
            .get::<ReachableStateAnnotation>()
            .expect(
                "ReachableStateAnnotation missing: \
                 UnreachableCodeProcessor must run before the lint pipeline",
            );
        let code = target.get_bytecode();

        // Two passes: a dead instruction can carry a Loc that encloses a
        // *later* reachable Loc (e.g. a synthesized jump using the surrounding
        // loop's Loc, emitted before the loop's reachable back-edge target),
        // so we need every reachable Loc in hand before applying the filter.
        // For inlined instructions, also record their call-site Loc. A
        // function whose body is entirely inlined ends with a synthesized
        // `Ret` whose Loc is the caller's body; without the call site in
        // the reachable set, that Loc encloses nothing reachable (the
        // inlinee Locs sit elsewhere) and we'd false-positive.
        let mut reachable_locs: BTreeSet<Loc> = BTreeSet::new();
        for (offset, instr) in code.iter().enumerate() {
            if !annotation.is_definitely_not_reachable(offset as CodeOffset) {
                let loc = target.get_bytecode_loc(instr.get_attr_id());
                reachable_locs.insert(call_site_loc(&loc));
                reachable_locs.insert(loc);
            }
        }

        let mut current_run: Vec<Loc> = Vec::new();
        for (offset, instr) in code.iter().enumerate() {
            if annotation.is_definitely_not_reachable(offset as CodeOffset) {
                let loc = target.get_bytecode_loc(instr.get_attr_id());
                // The two skip paths below use `continue` rather than flushing the
                // current run on purpose: when two dead instructions are split only
                // by skipped ones, we still want them reported as a single warning
                // instead of two adjacent ones.
                // Skip code from inlining — not actionable at this site.
                if loc.is_inlined() {
                    continue;
                }
                // Skip Locs of compiler-synthesized scaffolding (merge labels,
                // back-jumps, trailing `Ret`): they reuse a parent AST node's
                // Loc that physically wraps a reachable sibling instruction.
                if reachable_locs.iter().any(|r| encloses_by_span(&loc, r)) {
                    continue;
                }
                current_run.push(loc);
            } else {
                self.flush(target, std::mem::take(&mut current_run));
            }
        }
        self.flush(target, current_run);
    }
}

impl UnreachableCode {
    fn flush(&self, target: &FunctionTarget, run: Vec<Loc>) {
        if run.is_empty() {
            return;
        }
        // `Loc::enclosing` takes min-start / max-end, so duplicates and
        // unsorted input are fine — many bytecode instructions share one
        // statement Loc.
        let loc = Loc::enclosing(&run);
        self.report(target.global_env(), &loc, "unreachable code");
    }
}

/// Outermost Loc in the inlined-from chain — the user-visible call site.
fn call_site_loc(loc: &Loc) -> Loc {
    let mut cur = loc;
    while let Some(next) = cur.inlined_from_loc() {
        cur = next;
    }
    cur.clone()
}

/// Span-only enclosure (ignores `inlined_from_loc`, unlike `Loc::is_enclosing`).
///
/// The scaffolding-skip heuristic in `check` relies on a compiler invariant:
/// synthesized bytecode instructions (merge labels, back-jumps, trailing `Ret`)
/// inherit the `Loc` of their enclosing AST node rather than getting a distinct
/// `Loc`. The filter detects such instructions by checking whether a dead
/// instruction's `Loc` physically wraps a reachable instruction's `Loc`.
/// If that compiler invariant ever changes, this heuristic will need updating.
fn encloses_by_span(outer: &Loc, inner: &Loc) -> bool {
    outer.file_id() == inner.file_id()
        && inner.span().start() >= outer.span().start()
        && inner.span().end() <= outer.span().end()
}
