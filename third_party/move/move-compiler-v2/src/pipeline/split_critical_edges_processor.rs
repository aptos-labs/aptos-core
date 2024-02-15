// Copyright © Aptos Foundation
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

use move_model::{ast::TempIndex, model::FunctionEnv};
use move_stackless_bytecode::{
    function_target::FunctionData,
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{AttrId, Bytecode, Label},
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
        if fun_env.is_native() {
            return data;
        }
        let mut transformer = SplitCriticalEdgesTransformation::new(std::mem::take(&mut data.code));
        transformer.transform();
        data.code = transformer.code;
        data.annotations.clear();
        data
    }

    fn name(&self) -> String {
        "SplitCriticalEdgesProcessor".to_owned()
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
    fn transform(&mut self) {
        self.code = self.transform_bytecode_vec(self.code)
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
            Bytecode::Branch(attr_id, l0, l1, t) => self.transform_branch(transformed, attr_id, l0, l1, t),
            // Edge of a `Jump` is never critical because the source node only has one out edge.
            _ => transformed.push(bytecode)
        }
    }

    /// Transforms a branch instruction by splitting the critical out edges, and append to `transformed`.
    /// Note that this will not invalidate `incoming_edge_count`, since the incoming edge of a branch
    /// is replaced by the goto in the generated empty block
    pub fn transform_branch(&mut self, transformed: &mut Vec<Bytecode>, attr_id: AttrId, l0: Label, l1: Label, t: TempIndex) {
        match (
            self.split_critical_edge(attr_id, l0),
            self.split_critical_edge(attr_id, l1),
        ) {
            (None, None) => {
                transformed.push(Bytecode::Branch(attr_id, l0, l1, t))
            },
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
                    if !prev_instr.is_branch() {
                        increment_key_count(&mut srcs_count, *label)
                    }
                }
            },
            _ => {},
        }
    }
    srcs_count
}
