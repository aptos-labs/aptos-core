// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Computes at a given program point, how the function may exit later.
//! Does the function return, abort, or doesn't terminate?
//! Check documentation of `ExitState` for more on the abstract domain used in the analysis.
//! The analysis is intraprocedural, and considers any user function may abort.
//!
//! Requires: The program cannot silently exits, i.e., neither returns nor aborts, but runs out of instructions.

use abstract_domain_derive::AbstractDomain;
use itertools::Itertools;
use move_binary_format::file_format::CodeOffset;
use move_model::model::FunctionEnv;
use move_stackless_bytecode::{
    dataflow_analysis::{DataflowAnalysis, TransferFunctions},
    dataflow_domains::{AbstractDomain, JoinResult, SetDomain},
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::Bytecode,
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use std::{collections::BTreeMap, fmt::Display};

/// The power set lattice of `ExitStatus`
///
/// - the join operation is set union
/// - the top element is { Return, Abort }: may return, abort, or not terminate
/// - { Return }: may return or not terminate, but definitely does not abort
/// - { Abort }: may abort or not terminate, but definitely does not return
/// - the bottom element is {}: don't terminate
///
/// That is, if at a program point the abstract state is `s`, then for all paths from that point,
/// the program can only exit in the exit states contained in `s`, if the program does terminate.
#[derive(AbstractDomain, Clone)]
pub struct ExitState(SetDomain<ExitStatus>);

impl ExitState {
    /// Returns a empty set, which is the bottom element
    pub fn bot() -> Self {
        Self(SetDomain::default())
    }

    /// Returns a singleton
    pub fn singleton(e: ExitStatus) -> Self {
        Self(SetDomain::singleton(e))
    }

    /// Checks whether the state may return
    pub fn may_return(&self) -> bool {
        self.0.iter().contains(&ExitStatus::Return)
    }
}

/// The exit state of a function
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum ExitStatus {
    /// The program returns
    Return,
    /// The program aborts
    Abort,
}

impl Display for ExitStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ExitStatus::Return => "returns",
            ExitStatus::Abort => "aborts",
        })
    }
}

/// Before and after abort state at a program point
#[derive(Clone)]
pub struct ExitStateAtCodeOffset {
    pub before: ExitState,
    pub after: ExitState,
}

impl ExitStateAtCodeOffset {
    pub fn new(before: ExitState, after: ExitState) -> Self {
        Self { before, after }
    }
}

#[derive(Clone)]
pub struct ExitStateAnnotation(BTreeMap<CodeOffset, ExitStateAtCodeOffset>);

impl ExitStateAnnotation {
    /// Get the abort state at the given code offset
    pub fn get_annotation_at(&self, code_offset: CodeOffset) -> Option<&ExitStateAtCodeOffset> {
        self.0.get(&code_offset)
    }
}

pub struct ExitStateAnalysis {}

impl ExitStateAnalysis {
    /// Returns the state per instruction of the given function
    fn analyze(&self, target: &FunctionTarget) -> BTreeMap<CodeOffset, ExitStateAtCodeOffset> {
        let code = target.get_bytecode();
        let cfg = StacklessControlFlowGraph::new_backward(code, true);
        let state_map = self.analyze_function(ExitState::bot(), code, &cfg);
        self.state_per_instruction(state_map, code, &cfg, |before, after| {
            ExitStateAtCodeOffset::new(before.clone(), after.clone())
        })
    }
}

impl TransferFunctions for ExitStateAnalysis {
    type State = ExitState;

    const BACKWARD: bool = true;

    fn execute(&self, state: &mut Self::State, instr: &Bytecode, _offset: CodeOffset) {
        match instr {
            Bytecode::Abort(..) => {
                *state = ExitState::singleton(ExitStatus::Abort);
            },
            Bytecode::Ret(..) => {
                *state = ExitState::singleton(ExitStatus::Return);
            },
            Bytecode::Call(_, _, op, _, _) => {
                if op.can_abort() {
                    state.join(&ExitState::singleton(ExitStatus::Abort));
                }
            },
            _ => {},
        }
    }
}

impl DataflowAnalysis for ExitStateAnalysis {}

pub struct ExitStateAnalysisProcessor {}

impl FunctionTargetProcessor for ExitStateAnalysisProcessor {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        fun_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if fun_env.is_native() {
            return data;
        }
        let target = FunctionTarget::new(fun_env, &data);
        let analysis = ExitStateAnalysis {};
        let annotations = ExitStateAnnotation(analysis.analyze(&target));
        data.annotations.set(annotations, true);
        data
    }

    fn name(&self) -> String {
        "AbortAnalysisProcessor".to_owned()
    }
}

impl ExitStateAnalysisProcessor {
    pub fn register_formatters(target: &FunctionTarget) {
        target.register_annotation_formatter(Box::new(format_abort_state_annotation))
    }
}

/// Formats abort state annotations
pub fn format_abort_state_annotation(
    target: &FunctionTarget,
    code_offset: CodeOffset,
) -> Option<String> {
    let ExitStateAnnotation(state_per_instr) =
        target.get_annotations().get::<ExitStateAnnotation>()?;
    let ExitStateAtCodeOffset { before, after } = state_per_instr.get(&code_offset)?;
    Some(format!(
        "abort state before: {}\nabort state after : {}",
        before.0.to_string(ExitStatus::to_string),
        after.0.to_string(ExitStatus::to_string)
    ))
}
