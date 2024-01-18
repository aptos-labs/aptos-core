// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements the "definitely available copies" analysis, also called "available copies" analysis (in short).
//! This analysis is a prerequisite for the copy propagation transformation.
//!
//! A copy is of the form `a := b` (i.e., `a` is assigned `b`), where `a` and `b` are locals/temporaries.
//!
//! A definitely available copy at a given program point `P` is a copy `a := b` that has reached `P`
//! along all possible program paths such that neither `a` nor `b` is overwritten along any of these paths.
//! That is, `a` and `b` are always available unmodified at `P` after the copy `a := b`,
//! making it definitely available.
//! In the current implementation, variables that are borrowed are excluded from being a part of an
//! available copy. We can make this analysis more precise by having more refined rules when it comes
//! to borrowed variables.
//!
//! This is a forward "must" analysis.
//! In a forward analysis, we reason about facts at a program point `P` using facts at its predecessors.
//! The "must" qualifier means that the analysis facts must be true at `P`,
//! irrespective of what path lead to `P`.

use itertools::Itertools;
use move_binary_format::file_format::CodeOffset;
use move_model::{ast::TempIndex, model::FunctionEnv};
use move_stackless_bytecode::{
    dataflow_analysis::{DataflowAnalysis, TransferFunctions},
    dataflow_domains::{AbstractDomain, JoinResult},
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{Bytecode, Operation},
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use std::collections::{BTreeMap, BTreeSet};

/// Collection of definitely available copies.
/// For a copy `a := b`, we store the key-value pair `(a, b)` in the internal map.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AvailCopies(BTreeMap</*dst*/ TempIndex, /*src*/ TempIndex>);

impl AvailCopies {
    /// Create a new (empty) collection of definitely available copies.
    fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Make a copy `dst := src` available.
    /// Neither `dst` nor `src` should be borrowed locals.
    /// To call this method, `dst := x` should not already be available for any `x`.
    fn make_copy_available(&mut self, dst: TempIndex, src: TempIndex) {
        if src == dst {
            // No need to make a copy available for self-assignments.
            return;
        }
        let old_src = self.0.insert(dst, src);
        if let Some(old_src) = old_src {
            panic!(
                "copy `$t{} = $t{}` already available, \
                    cannot have `$t{} = $t{}` available as well",
                dst, old_src, dst, src
            );
        }
    }

    /// Kill all available copies of the form `x := y` where `x` or `y` is `tmp`.
    /// Note that `tmp` should not be a borrowed local.
    fn kill_copies_with(&mut self, tmp: TempIndex) {
        // TODO: consider optimizing the following operation by keeping a two-way map between
        // `dst -> src` and `src -> set(dst)`. Another optimization to consider is to use im::OrdMap.
        self.0.retain(|dst, src| *dst != tmp && *src != tmp);
    }

    /// Given a set of available copies: `tmp_1 := tmp_0, tmp_2 := tmp_1,..., tmp_n := tmp_n-1`, forming
    /// the copy chain: `tmp_0 --copied-to--> tmp_1 --copied-to--> tmp_2 -> ... -> tmp_n-1 -> tmp_n`,
    /// return the head of the copy chain `tmp_0` for any input `tmp_x` (x in 0..=n) in the chain.
    ///
    /// Note that it is a required invariant that the copy chain is acyclic, else we panic.
    /// The natural way of constructing the copy chain for move bytecode (like in this file) ensures this.
    pub fn get_head_of_copy_chain(&self, mut tmp: TempIndex) -> TempIndex {
        let mut visit_counter = 0;
        let limit_visits = self.0.len();
        while let Some(src) = self.0.get(&tmp) {
            visit_counter += 1;
            if visit_counter > limit_visits {
                // The copy chain is cyclic, which is an invariant violation.
                panic!("copy chain is cyclic");
            }
            tmp = *src;
        }
        tmp
    }
}

impl Default for AvailCopies {
    /// Create a default (empty) collection of definitely available copies.
    fn default() -> Self {
        Self::new()
    }
}

impl AbstractDomain for AvailCopies {
    /// Keep only those copies in `self` that are available in both `self` and `other`.
    /// Report if `self` has changed.
    fn join(&mut self, other: &Self) -> JoinResult {
        let mut result = JoinResult::Unchanged;
        let prev_copies = std::mem::take(&mut self.0);
        for (other_dst, other_src) in &other.0 {
            if let Some(src) = prev_copies.get(other_dst) {
                if src != other_src {
                    // We are removing the available copy (dst, src) from self.
                    result = JoinResult::Changed;
                } else {
                    // Both have (other_dst, other_src), so keep it in self.
                    self.0.insert(*other_dst, *other_src);
                }
            }
            // else: a copy (other_dst, other_src) was previously not available in self, so no change to self.
        }
        if prev_copies.len() != self.0.len() {
            // We have removed some copies from self.
            result = JoinResult::Changed;
        }
        result
    }
}

/// Definitely available copies before and after a stackless bytecode instruction.
#[derive(Clone)]
struct AvailCopiesState {
    before: AvailCopies,
    after: AvailCopies,
}

/// Mapping from code offsets to definitely available copies before and after the instruction at the code offset.
#[derive(Clone)]
pub struct AvailCopiesAnnotation(BTreeMap<CodeOffset, AvailCopiesState>);

impl AvailCopiesAnnotation {
    /// Get the definitely available copies before the instruction at the given `code_offset`.
    pub fn before(&self, code_offset: &CodeOffset) -> Option<&AvailCopies> {
        if let Some(state) = self.0.get(code_offset) {
            Some(&state.before)
        } else {
            None
        }
    }
}

/// The definitely available copies analysis for a function.
pub struct AvailCopiesAnalysis {
    borrowed_locals: BTreeSet<TempIndex>, // Locals borrowed in the function being analyzed.
}

impl AvailCopiesAnalysis {
    /// Create a new instance of definitely available copies analysis.
    /// `code` is the bytecode of the function being analyzed.
    pub fn new(code: &[Bytecode]) -> Self {
        Self {
            borrowed_locals: Self::get_borrowed_locals(code),
        }
    }

    /// Analyze the given function and return the definitely available copies annotation.
    fn analyze(&self, func_target: &FunctionTarget) -> AvailCopiesAnnotation {
        let code = func_target.get_bytecode();
        let cfg = StacklessControlFlowGraph::new_forward(code);
        let block_state_map = self.analyze_function(AvailCopies::new(), code, &cfg);
        let per_bytecode_state =
            self.state_per_instruction(block_state_map, code, &cfg, |before, after| {
                AvailCopiesState {
                    before: before.clone(),
                    after: after.clone(),
                }
            });
        AvailCopiesAnnotation(per_bytecode_state)
    }

    /// Get the set of locals that have been borrowed in the function being analyzed.
    fn get_borrowed_locals(code: &[Bytecode]) -> BTreeSet<TempIndex> {
        code.iter()
            .filter_map(|bc| {
                if let Bytecode::Call(_, _, Operation::BorrowLoc, srcs, _) = bc {
                    // BorrowLoc should have only one source.
                    srcs.first().cloned()
                } else {
                    None
                }
            })
            .collect()
    }
}

impl TransferFunctions for AvailCopiesAnalysis {
    type State = AvailCopies;

    // This is a forward analysis.
    const BACKWARD: bool = false;

    fn execute(&self, state: &mut Self::State, instr: &Bytecode, _offset: CodeOffset) {
        use Bytecode::*;
        instr.dests().iter().for_each(|dst| {
            if !self.borrowed_locals.contains(dst) {
                // We don't track copies of borrowed locals, so no need to kill them.
                state.kill_copies_with(*dst);
            }
        });
        if let Assign(_, dst, src, _) = instr {
            if !self.borrowed_locals.contains(dst) && !self.borrowed_locals.contains(src) {
                // Note that we are conservative here for the sake of simplicity, and disallow
                // tracking copies when either `dst` or `src` is borrowed.
                // We could track more copies as available by using the reference analysis.
                state.make_copy_available(*dst, *src);
            }
        }
    }
}

impl DataflowAnalysis for AvailCopiesAnalysis {}

/// Processor for the definitely available copies analysis.
pub struct AvailCopiesAnalysisProcessor();

impl FunctionTargetProcessor for AvailCopiesAnalysisProcessor {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if func_env.is_native() {
            return data;
        }
        let target = FunctionTarget::new(func_env, &data);
        let analysis = AvailCopiesAnalysis::new(target.get_bytecode());
        let annotation = analysis.analyze(&target);
        data.annotations.set(annotation, true);
        data
    }

    fn name(&self) -> String {
        "AvailableCopiesAnalysisProcessor".to_string()
    }
}

impl AvailCopiesAnalysisProcessor {
    /// Registers annotation formatter at the given function target.
    /// Helps with testing and debugging.
    pub fn register_formatters(target: &FunctionTarget) {
        target.register_annotation_formatter(Box::new(format_avail_copies_annotation));
    }
}

// ====================================================================
// Formatting functionality for available copies annotation.

pub fn format_avail_copies_annotation(
    target: &FunctionTarget<'_>,
    code_offset: CodeOffset,
) -> Option<String> {
    let AvailCopiesAnnotation(map) = target.get_annotations().get::<AvailCopiesAnnotation>()?;
    let AvailCopiesState { before, after } = map.get(&code_offset)?;
    let mut s = String::new();
    s.push_str("before: ");
    s.push_str(&format_avail_copies(before, target));
    s.push_str(", after: ");
    s.push_str(&format_avail_copies(after, target));
    Some(s)
}

fn format_avail_copies(state: &AvailCopies, target: &FunctionTarget<'_>) -> String {
    let mut s = String::new();
    s.push('{');
    let mut first = true;
    for (dst, src) in &state.0 {
        if first {
            first = false;
        } else {
            s.push_str(", ");
        }
        s.push_str(
            &vec![dst, src]
                .into_iter()
                .map(|tmp| {
                    let name = target.get_local_raw_name(*tmp);
                    name.display(target.symbol_pool()).to_string()
                })
                .join(" := "),
        );
    }
    s.push('}');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avail_copies_join() {
        let mut a = AvailCopies::new();
        let mut b = AvailCopies::new();
        a.make_copy_available(1, 2);
        b.make_copy_available(3, 4);
        // a = (1, 2), b = (3, 4)
        assert_eq!(a.join(&b), JoinResult::Changed);
        assert_eq!(a.0.len(), 0);
        a.make_copy_available(3, 4);
        // a = (3, 4), b = (3, 4)
        assert_eq!(a.join(&b), JoinResult::Unchanged);
        assert_eq!(a, b);
        a.make_copy_available(1, 2);
        // a = (1, 2), (3, 4), b = (3, 4)
        assert_eq!(a.join(&b), JoinResult::Changed);
        assert_eq!(a, b);
        b.make_copy_available(1, 2);
        // a = (3, 4), b = (1, 2), (3, 4)
        assert_eq!(a.join(&b), JoinResult::Unchanged);
        assert_eq!(a.0.len(), 1);
    }

    #[test]
    fn test_get_head_of_copy_chain() {
        let mut copies = AvailCopies::new();
        copies.make_copy_available(1, 0);
        copies.make_copy_available(2, 1);
        copies.make_copy_available(3, 2);
        copies.make_copy_available(4, 3);
        copies.make_copy_available(44, 14);
        // copies = (1, 0), (2, 1), (3, 2), (4, 3), (44, 14)
        for i in 0..=4 {
            assert_eq!(copies.get_head_of_copy_chain(i), 0);
        }
    }

    #[test]
    #[should_panic]
    fn test_cyclic_copy_chain() {
        let mut copies = AvailCopies::new();
        copies.make_copy_available(1, 0);
        copies.make_copy_available(2, 1);
        copies.make_copy_available(3, 2);
        copies.make_copy_available(4, 3);
        copies.make_copy_available(0, 4);
        // copies = (1, 0), (2, 1), (3, 2), (4, 3), (0, 4)
        copies.get_head_of_copy_chain(4);
    }
}
