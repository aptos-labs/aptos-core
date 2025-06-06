// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

// Final phase of cleanup and optimization.

use crate::options::ProverOptions;
use move_binary_format::file_format::CodeOffset;
use move_model::{
    model::FunctionEnv,
    pragmas::INTRINSIC_FUN_MAP_BORROW_MUT,
    well_known::{EVENT_EMIT_EVENT, VECTOR_BORROW_MUT},
};
use move_stackless_bytecode::{
    dataflow_analysis::{DataflowAnalysis, TransferFunctions},
    dataflow_domains::{AbstractDomain, JoinResult},
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{BorrowNode, Bytecode, Operation},
    stackless_control_flow_graph::StacklessControlFlowGraph,
};
use std::collections::BTreeSet;

pub struct CleanAndOptimizeProcessor();

impl CleanAndOptimizeProcessor {
    pub fn new() -> Box<Self> {
        Box::new(Self())
    }
}

impl FunctionTargetProcessor for CleanAndOptimizeProcessor {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if func_env.is_native() {
            // Nothing to do
            return data;
        }

        // Run optimizer
        let options = ProverOptions::get(func_env.module_env.env);
        let instrs = std::mem::take(&mut data.code);
        let new_instrs = Optimizer {
            options: &options,
            target: &FunctionTarget::new(func_env, &data),
        }
        .run(instrs);
        data.code = new_instrs;
        data
    }

    fn name(&self) -> String {
        "clean_and_optimize".to_string()
    }
}

// Analysis
// ========

/// A data flow analysis state used for optimization analysis. Currently it tracks the nodes
/// which have been updated but not yet written back.
#[derive(Debug, Clone, Default, Eq, PartialEq, PartialOrd)]
struct AnalysisState {
    unwritten: BTreeSet<BorrowNode>,
}

impl AbstractDomain for AnalysisState {
    fn join(&mut self, other: &Self) -> JoinResult {
        let n = self.unwritten.len();
        self.unwritten.extend(other.unwritten.iter().cloned());
        if self.unwritten.len() == n {
            JoinResult::Unchanged
        } else {
            JoinResult::Changed
        }
    }
}

struct Optimizer<'a> {
    options: &'a ProverOptions,
    target: &'a FunctionTarget<'a>,
}

impl TransferFunctions for Optimizer<'_> {
    type State = AnalysisState;

    const BACKWARD: bool = false;

    fn execute(&self, state: &mut AnalysisState, instr: &Bytecode, _offset: CodeOffset) {
        use BorrowNode::*;
        use Bytecode::*;
        use Operation::*;
        if let Call(_, _, oper, srcs, _) = instr {
            match oper {
                WriteRef => {
                    state.unwritten.insert(Reference(srcs[0]));
                },
                WriteBack(Reference(dest), ..) => {
                    if state.unwritten.contains(&Reference(srcs[0])) {
                        state.unwritten.insert(Reference(*dest));
                    }
                },
                Function(mid, fid, _) => {
                    let callee_env = &self
                        .target
                        .global_env()
                        .get_function_qid(mid.qualified(*fid));
                    let has_effect = if !self.options.for_interpretation
                        && callee_env.is_native_or_intrinsic()
                    {
                        // Exploit knowledge about builtin functions
                        !(callee_env.is_well_known(VECTOR_BORROW_MUT)
                            || callee_env.is_well_known(EVENT_EMIT_EVENT)
                            || callee_env.is_intrinsic_of(INTRINSIC_FUN_MAP_BORROW_MUT)
                            || is_custom_borrow(callee_env, &self.options.borrow_natives))
                    } else {
                        true
                    };

                    // Mark &mut parameters to functions as unwritten.
                    if has_effect {
                        for src in srcs {
                            if self.target.get_local_type(*src).is_mutable_reference() {
                                state.unwritten.insert(Reference(*src));
                            }
                        }
                    }
                },
                _ => {},
            }
        }
    }
}

/// Check if fun_env matches one of the functions implementing custom mutable borrow semantics.
fn is_custom_borrow(fun_env: &FunctionEnv, borrow_natives: &Vec<String>) -> bool {
    for name in borrow_natives {
        if &fun_env.get_full_name_str() == name {
            return true;
        }
    }
    false
}

impl DataflowAnalysis for Optimizer<'_> {}

// Transformation
// ==============

impl Optimizer<'_> {
    fn run(&mut self, instrs: Vec<Bytecode>) -> Vec<Bytecode> {
        // Rum Analysis
        let cfg = StacklessControlFlowGraph::new_forward(&instrs);
        let state = self.analyze_function(AnalysisState::default(), &instrs, &cfg);
        let data = self.state_per_instruction(state, &instrs, &cfg, |before, _| before.clone());

        // Transform code.
        let mut new_instrs = vec![];
        let mut should_skip = BTreeSet::new();
        for (code_offset, instr) in instrs.iter().enumerate() {
            use BorrowNode::*;
            use Bytecode::*;
            use Operation::*;

            let is_unwritten = |code_offset: CodeOffset, node: &BorrowNode| {
                if let Some(unwritten) = data.get(&code_offset).map(|d| &d.unwritten) {
                    unwritten.contains(node)
                } else {
                    // No data for this node, so assume it is unwritten.
                    true
                }
            };

            // Perform peephole optimization
            match (new_instrs.last(), instr) {
                (None, _) => {},
                (Some(Call(_, _, UnpackRef, srcs1, _)), Call(_, _, PackRef, srcs2, _))
                    if srcs1[0] == srcs2[0] =>
                {
                    // skip this redundant unpack/pack pair.
                    new_instrs.pop();
                    continue;
                },
                (Some(Call(_, dests, IsParent(..), srcs, _)), Branch(_, _, _, tmp))
                    if dests[0] == *tmp
                        && !is_unwritten(code_offset as CodeOffset, &Reference(srcs[0])) =>
                {
                    assert!(matches!(instrs[code_offset + 1], Label(..)));
                    // skip this obsolete IsParent check when all WriteBacks in this block are redundant
                    let mut block_cursor = code_offset + 2;
                    let mut skip_branch = true;
                    loop {
                        match &instrs[block_cursor] {
                            Call(_, _, WriteBack(_, _), srcs, _) => {
                                if is_unwritten(block_cursor as CodeOffset, &Reference(srcs[0])) {
                                    skip_branch = false;
                                    break;
                                }
                                // skip redundant write-backs
                                should_skip.insert(block_cursor);
                            },
                            Call(_, _, TraceLocal(_), _, _) => {
                                // since the previous write-back is skipped, this trace local is redundant as well
                                should_skip.insert(block_cursor);
                            },
                            _ => {
                                break;
                            },
                        }
                        block_cursor += 1;
                    }
                    if skip_branch {
                        // get rid of the label as well
                        should_skip.insert(code_offset + 1);
                        new_instrs.pop();
                        continue;
                    }
                },
                (Some(_), _) => {},
            }

            // Do not include this instruction if it is marked as skipped
            if should_skip.contains(&code_offset) {
                continue;
            }

            // Other cases for skipping the instruction
            match instr {
                // Remove unnecessary WriteBack
                Call(_, _, WriteBack(..), srcs, _)
                    if !is_unwritten(code_offset as CodeOffset, &Reference(srcs[0])) =>
                {
                    continue;
                },
                _ => {},
            }

            // This instruction should be included
            new_instrs.push(instr.clone());
        }
        new_instrs
    }
}
