// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements a data-flow analysis to determine whether an instruction is reachable or not.
//! This analysis does not have any prerequisites.
//! This analysis sets an annotation of type `UnreachableCodeAnnotation` on each function target.
//! This annotation is a prerequisite for the unreachable code checker and unreachable code remover.
//!
//! This analysis a forward "may" analysis, it tracks whether an instruction is:
//! - maybe reachable (there may be an execution path from the function entry to the instruction)
//! - definitely not reachable (there is no execution path from the function entry to the instruction)

use move_binary_format::file_format::CodeOffset;
use move_model::model::FunctionEnv;
use move_stackless_bytecode::{
    dataflow_analysis::{DataflowAnalysis, TransferFunctions},
    dataflow_domains::{AbstractDomain, JoinResult},
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::Bytecode,
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use std::collections::BTreeMap;

/// Reachability state of an instruction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReachableState {
    Maybe, // Maybe reachable from function entry
    No,    // Definitely not reachable from function entry
}

impl ReachableState {
    /// Mark this state as maybe reachable from the function entry.
    fn mark_as_maybe_reachable(&mut self) {
        *self = ReachableState::Maybe;
    }

    /// Mark this state as definitely not reachable from the function entry.
    fn mark_as_not_reachable(&mut self) {
        *self = ReachableState::No;
    }
}

impl AbstractDomain for ReachableState {
    fn join(&mut self, other: &Self) -> JoinResult {
        use ReachableState::*;
        match (self.clone(), other) {
            (No, Maybe) => {
                self.mark_as_maybe_reachable();
                JoinResult::Changed
            },
            (Maybe, _) | (No, No) => JoinResult::Unchanged,
        }
    }
}

/// Mapping from code offsets to their reachability state, before executing the
/// instruction at the code offset.
#[derive(Clone, Debug)]
pub struct ReachableStateAnnotation(BTreeMap<CodeOffset, ReachableState>);

impl ReachableStateAnnotation {
    /// Is the instruction at the given `offset` definitely not reachable?
    pub fn is_not_reachable(&self, offset: CodeOffset) -> bool {
        self.0
            .get(&offset)
            .map_or(true, |state| matches!(state, ReachableState::No))
    }
}

/// Forward intra-procedural dataflow analysis.
/// Determines whether an instruction is reachable or not.
pub struct UnreachableCodeAnalysis {}

impl UnreachableCodeAnalysis {
    /// Analyze the given function and return a mapping from code offsets to their reachability state.
    fn analyze(&self, func_target: &FunctionTarget) -> ReachableStateAnnotation {
        let code = func_target.get_bytecode();
        let cfg = StacklessControlFlowGraph::new_forward(code);
        // We assume the entry of a function is reachable, as we have implemented this analysis
        // as an intra-procedural analysis.
        let block_state_map = self.analyze_function(ReachableState::Maybe, code, &cfg);
        let per_bytecode_state =
            self.state_per_instruction(block_state_map, code, &cfg, |before, _| before.clone());
        ReachableStateAnnotation(per_bytecode_state)
    }
}

impl TransferFunctions for UnreachableCodeAnalysis {
    type State = ReachableState;

    // This is forward analysis.
    const BACKWARD: bool = false;

    fn execute(&self, state: &mut Self::State, instr: &Bytecode, _offset: CodeOffset) {
        use Bytecode::*;
        // TODO: the precision of this analysis can be improved when constant propagation
        // information is available.
        // For example:
        // - if a branch condition is a constant false, then the branch target is definitely not reachable.
        // - if addition of two constants overflows, then code after is definitely not reachable.
        if matches!(instr, Ret(..) | Abort(..)) {
            state.mark_as_not_reachable();
        }
    }
}

impl DataflowAnalysis for UnreachableCodeAnalysis {}

/// A processor which performs the unreachable code analysis.
pub struct UnreachableCodeProcessor {}

impl FunctionTargetProcessor for UnreachableCodeProcessor {
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
        let analysis = UnreachableCodeAnalysis {};
        let annotation = analysis.analyze(&target);
        data.annotations.set(annotation, true);
        data
    }

    fn name(&self) -> String {
        "UnreachableCodeProcessor".to_string()
    }
}

impl UnreachableCodeProcessor {
    /// Registers annotation formatter at the given function target.
    /// Helps with testing and debugging.
    pub fn register_formatters(target: &FunctionTarget) {
        target.register_annotation_formatter(Box::new(format_reachable_state_annotation));
    }
}

// ====================================================================
// Formatting functionality for reachability state annotation.

pub fn format_reachable_state_annotation(
    target: &FunctionTarget,
    code_offset: CodeOffset,
) -> Option<String> {
    let annotation = target.get_annotations().get::<ReachableStateAnnotation>()?;
    if annotation.is_not_reachable(code_offset) {
        Some("no".to_string())
    } else {
        Some("maybe".to_string())
    }
}
