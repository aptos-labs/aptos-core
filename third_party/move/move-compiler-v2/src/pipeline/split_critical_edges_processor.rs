// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This pass splits critical edges with empty blocks.
//! A critical edge is an edge where the source node has multiple successors,
//! and the target node has multiple predecessors.
//!
//! Side effects: clear existing annotations.
//!
//! Prerequisites: no call instructions have abort actions.
//!
//! Postconditions: no critical edges in the control flow graph.

use log::{log_enabled, Level};
use move_model::{ast::TempIndex, model::FunctionEnv};
use move_stackless_bytecode::{
    function_target::FunctionData,
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{AttrId, Bytecode, Label},
    stackless_control_flow_graph::{BlockId, StacklessControlFlowGraph},
};
use std::collections::{BTreeMap, BTreeSet};

pub struct SplitCriticalEdgesProcessor {}

impl FunctionTargetProcessor for SplitCriticalEdgesProcessor {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        fun_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if cfg!(debug_assertions) || log_enabled!(Level::Debug) {
            Self::check_precondition(&data);
        }
        if fun_env.is_native() {
            return data;
        }
        let mut transformer = SplitCriticalEdgesTransformation::new(std::mem::take(&mut data.code));
        transformer.transform();
        data.code = transformer.code;
        data.annotations.clear();
        if cfg!(debug_assertions) || log_enabled!(Level::Debug) {
            Self::check_postcondition(&data.code);
        }
        data
    }

    fn name(&self) -> String {
        "SplitCriticalEdgesProcessor".to_owned()
    }
}

impl SplitCriticalEdgesProcessor {
    /// Checks the precondition of the transformaiton; cf. module documentation.
    fn check_precondition(data: &FunctionData) {
        for instr in &data.code {
            if matches!(instr, Bytecode::Call(_, _, _, _, Some(_))) {
                panic!("precondition violated: found call instruction with abort action")
            }
        }
    }

    /// Checks the postcondition of the transformation; cf. module documentation.
    fn check_postcondition(code: &[Bytecode]) {
        let cfg = StacklessControlFlowGraph::new_forward(code);
        let blocks = cfg.blocks();
        let mut pred_count: BTreeMap<BlockId, usize> =
            blocks.iter().map(|block_id| (*block_id, 0)).collect();
        for block in &blocks {
            // don't count the edge from the dummy start to a block as an incoming edge
            if *block == cfg.entry_block() {
                continue;
            }
            for suc_block in cfg.successors(*block) {
                *pred_count
                    .get_mut(suc_block)
                    .unwrap_or_else(|| panic!("block {}", suc_block)) += 1;
            }
        }
        for block in blocks {
            let successors = cfg.successors(block);
            if successors.len() > 1 {
                for suc_block in successors {
                    assert!(
                        *pred_count.get(suc_block).expect("pred count") <= 1,
                        "{} has > 1 predecessors",
                        suc_block
                    )
                }
            }
        }
    }
}

struct SplitCriticalEdgesTransformation {
    /// Function data of the function being transformed
    code: Vec<Bytecode>,
    /// Labels used in the original code and in the generated code
    labels: BTreeSet<Label>,
    /// Maps label to its number of incoming edges, including explicit goto/branch or fall throughs.
    /// If a label is not in the map, it doesn't have any incoming edges.
    /// The count for lables generated in the transformation is not tracked,
    /// but this map will not be invalidated during the transformation.
    incoming_edge_count: BTreeMap<Label, usize>,
}

impl SplitCriticalEdgesTransformation {
    pub fn new(code: Vec<Bytecode>) -> Self {
        let labels = Bytecode::labels(&code);
        let incoming_edges_count = count_incoming_edges(&code);
        Self {
            code,
            labels,
            incoming_edge_count: incoming_edges_count,
        }
    }

    /// Runs the transformation, which breaks critical edges with empty blocks.
    pub fn transform(&mut self) {
        let code = std::mem::take(&mut self.code);
        self.code = self.transform_bytecode_vec(code)
    }

    /// Transforms the given code and returns the transformed code
    fn transform_bytecode_vec(&mut self, code: Vec<Bytecode>) -> Vec<Bytecode> {
        let mut transformed = Vec::new();
        for instr in code {
            self.transform_bytecode(&mut transformed, instr)
        }
        transformed
    }

    /// Transforms a bytecode, and append it to `transformed`
    fn transform_bytecode(&mut self, transformed: &mut Vec<Bytecode>, bytecode: Bytecode) {
        match bytecode {
            Bytecode::Branch(attr_id, l0, l1, t) => {
                self.transform_branch(transformed, attr_id, l0, l1, t)
            },
            // Edge of a `Jump` is never critical because the source node only has one out edge.
            _ => transformed.push(bytecode),
        }
    }

    /// Transforms a branch instruction by splitting the critical out edges, and append to `transformed`.
    /// Note that this will not invalidate `incoming_edge_count`, since the incoming edge of a branch
    /// is replaced by the goto in the generated empty block
    pub fn transform_branch(
        &mut self,
        transformed: &mut Vec<Bytecode>,
        attr_id: AttrId,
        l0: Label,
        l1: Label,
        t: TempIndex,
    ) {
        match (
            self.split_critical_edge(attr_id, l0),
            self.split_critical_edge(attr_id, l1),
        ) {
            (None, None) => transformed.push(Bytecode::Branch(attr_id, l0, l1, t)),
            (None, Some((l1_new, mut code))) => {
                transformed.push(Bytecode::Branch(attr_id, l0, l1_new, t));
                transformed.append(&mut code);
            },
            (Some((l0_new, mut code)), None) => {
                transformed.push(Bytecode::Branch(attr_id, l0_new, l1, t));
                transformed.append(&mut code)
            },
            (Some((l0_new, mut code0)), Some((l1_new, mut code1))) => {
                transformed.push(Bytecode::Branch(attr_id, l0_new, l1_new, t));
                transformed.append(&mut code0);
                transformed.append(&mut code1);
            },
        }
    }

    /// `label` is the target of some branch instruction
    /// If label has multiple sources, returns
    /// - a fresh label
    /// - a new empty block that
    ///     - starts with the fresh label
    ///     - jumps to `label`
    /// otherwise returns `None`
    fn split_critical_edge(
        &mut self,
        attr_id: AttrId,
        label: Label,
    ) -> Option<(Label, Vec<Bytecode>)> {
        // label is in `srcs_count` by construction
        if *self.incoming_edge_count.get(&label).expect("srcs count") > 1 {
            Some(self.split_edge(attr_id, label))
        } else {
            None
        }
    }

    /// Returns
    /// - a fresh label
    /// - a new empty block that
    ///     - starts with the fresh label
    ///     - jumps to `label`
    fn split_edge(&mut self, attr_id: AttrId, label: Label) -> (Label, Vec<Bytecode>) {
        let new_label = self.gen_fresh_label();
        let code = vec![
            Bytecode::Label(attr_id, new_label),
            Bytecode::Jump(attr_id, label),
        ];
        (new_label, code)
    }

    /// Generates a fresh label
    fn gen_fresh_label(&mut self) -> Label {
        let new_label = Label::new(
            if self.labels.is_empty() {
                0
            } else {
                let max_label = self.labels.iter().next_back().expect("label");
                max_label.as_usize() + 1
            },
        );
        self.labels.insert(new_label);
        new_label
    }
}

/// If key present in `map`, add 1 to its value; else insert 1
fn increment_key_count<Key: Ord>(map: &mut BTreeMap<Key, usize>, key: Key) {
    map.entry(key)
        .and_modify(|n: &mut usize| {
            *n += 1;
        })
        .or_insert(1usize);
}

/// Count the number of explicit and implicit (fall throughs) uses of labels.
/// Labels with no predecessors are not included.
fn count_incoming_edges(code: &[Bytecode]) -> BTreeMap<Label, usize> {
    let mut srcs_count = BTreeMap::new();
    for (code_offset, instr) in code.iter().enumerate() {
        match instr {
            Bytecode::Jump(_, label) => increment_key_count(&mut srcs_count, *label),
            Bytecode::Branch(_, l0, l1, _) => {
                increment_key_count(&mut srcs_count, *l0);
                increment_key_count(&mut srcs_count, *l1);
            },
            Bytecode::Label(_, label) => {
                if code_offset != 0 {
                    let prev_instr = code.get(code_offset - 1).expect("instruction");
                    // treat fall-through's to the label
                    if !prev_instr.is_branching() {
                        increment_key_count(&mut srcs_count, *label)
                    }
                }
            },
            _ => {},
        }
    }
    srcs_count
}

#[cfg(test)]
mod tests {
    use super::{AttrId, Bytecode, SplitCriticalEdgesProcessor, SplitCriticalEdgesTransformation};
    use move_stackless_bytecode::stackless_bytecode::Label as L;
    use Bytecode::*;

    /// Splits critical edges
    fn transform(code: Vec<Bytecode>) -> Vec<Bytecode> {
        let mut transformer = SplitCriticalEdgesTransformation::new(code);
        transformer.transform();
        transformer.code
    }

    #[test]
    fn test_empty_branch() {
        let attr = AttrId::new(0);
        let l0 = L::new(0);
        let l1 = L::new(1);
        let t = 0;
        // if (t) { L0: nop } L1: return t;
        let code = vec![
            Branch(attr, l0, l1, t),
            Label(attr, l0),
            Nop(attr),
            Label(attr, l1),
            Ret(attr, vec![t]),
        ];
        let transformed = transform(code);
        let l2 = L::new(2);
        let expected = vec![
            Branch(attr, l0, l2, t),
            Label(attr, l2),
            Jump(attr, l1),
            Label(attr, l0),
            Nop(attr),
            Label(attr, l1),
            Ret(attr, vec![t]),
        ];
        SplitCriticalEdgesProcessor::check_postcondition(&transformed);
        assert_eq!(transformed, expected)
    }

    #[test]
    fn test_break_in_while() {
        let attr = AttrId::new(0);
        let l0 = L::new(0);
        let t = 0;
        // while (t) { break } L0: return t;
        let code = vec![Branch(attr, l0, l0, t), Label(attr, l0), Ret(attr, vec![t])];
        let transformed = transform(code);
        let l1 = L::new(1);
        let l2 = L::new(2);
        let expected = vec![
            Branch(attr, l1, l2, t),
            Label(attr, l1),
            Jump(attr, l0),
            Label(attr, l2),
            Jump(attr, l0),
            Label(attr, l0),
            Ret(attr, vec![t]),
        ];
        SplitCriticalEdgesProcessor::check_postcondition(&transformed);
        assert_eq!(transformed, expected)
    }

    /// Demonstrates what happens for branch with equal labels
    #[test]
    fn test_branch_eq_label() {
        let attr = AttrId::new(0);
        let l0 = L::new(0);
        let t0 = 0;
        let code = vec![Label(attr, l0), Branch(attr, l0, l0, t0)];
        let transformed = transform(code);
        let l1 = L::new(1);
        let l2 = L::new(2);
        let expected = vec![
            Label(attr, l0),
            Branch(attr, l1, l2, t0),
            Label(attr, l1),
            Jump(attr, l0),
            Label(attr, l2),
            Jump(attr, l0),
        ];
        SplitCriticalEdgesProcessor::check_postcondition(&transformed);
        assert_eq!(transformed, expected)
    }

    /// Branch to the block containing the branch
    #[test]
    fn test_branch_self() {
        let attr = AttrId::new(0);
        let l0 = L::new(0);
        let l1 = L::new(1);
        let t = 0;
        let code = vec![
            Label(attr, l0),
            Nop(attr),
            Label(attr, l1),
            Branch(attr, l0, l1, t),
        ];
        let transformed = transform(code);
        let l2 = L::new(2);
        let expected = vec![
            Label(attr, l0),
            Nop(attr),
            Label(attr, l1),
            Branch(attr, l0, l2, t),
            Label(attr, l2),
            Jump(attr, l1),
        ];
        SplitCriticalEdgesProcessor::check_postcondition(&transformed);
        assert_eq!(transformed, expected)
    }
}
