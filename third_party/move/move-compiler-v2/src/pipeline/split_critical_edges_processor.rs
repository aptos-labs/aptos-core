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
        data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if fun_env.is_native() {
            return data;
        }
        let mut transformer = SplitCriticalEdgesTransformation::new(data);
        transformer.transform();
        transformer.data.annotations.clear();
        transformer.data
    }

    fn name(&self) -> String {
        "SplitCriticalEdgesProcessor".to_owned()
    }
}

struct SplitCriticalEdgesTransformation {
    data: FunctionData,
    // labels used in the original code and in the generated code
    labels: BTreeSet<Label>,
    srcs_count: BTreeMap<Label, usize>,
}

impl SplitCriticalEdgesTransformation {
    pub fn new(data: FunctionData) -> Self {
        let labels = Bytecode::labels(&data.code);
        let srcs_count = count_srcs(&data.code);
        Self {
            data,
            labels,
            srcs_count,
        }
    }

    /// Runs the transformation, which breaks critical edges with empty blocks.
    fn transform(&mut self) {
        let bytecodes = std::mem::take(&mut self.data.code);
        for bytecode in bytecodes {
            self.transform_bytecode(bytecode)
        }
    }

    /// Transforms a bytecode
    fn transform_bytecode(&mut self, bytecode: Bytecode) {
        match bytecode {
            Bytecode::Branch(attr_id, l0, l1, t) => self.transform_branch(attr_id, l0, l1, t),
            // Edge of a `Jump` is never critical because the source node only has one out edge.
            _ => self.emit(bytecode),
        }
    }

    /// Transforms a branch instruction by splitting the critical out edges
    pub fn transform_branch(&mut self, attr_id: AttrId, l0: Label, l1: Label, t: TempIndex) {
        match (
            self.split_critical_edge(attr_id, l0),
            self.split_critical_edge(attr_id, l1),
        ) {
            (None, None) => self.emit(Bytecode::Branch(attr_id, l0, l1, t)),
            (None, Some((l1_new, code))) => {
                self.emit(Bytecode::Branch(attr_id, l0, l1_new, t));
                self.emit_codes(code)
            },
            (Some((l0_new, code)), None) => {
                self.emit(Bytecode::Branch(attr_id, l0_new, l1, t));
                self.emit_codes(code)
            },
            (Some((l0_new, code0)), Some((l1_new, code1))) => {
                self.emit(Bytecode::Branch(attr_id, l0_new, l1_new, t));
                self.emit_codes(code0);
                self.emit_codes(code1);
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
        if *self.srcs_count.get(&label).expect("srcs count") > 1 {
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
                self.labels.iter().next_back().expect("label").as_usize() + 1
            },
        );
        self.labels.insert(new_label);
        new_label
    }

    fn emit(&mut self, bytecode: Bytecode) {
        self.data.code.push(bytecode)
    }

    fn emit_codes(&mut self, code: Vec<Bytecode>) {
        for instr in code {
            self.emit(instr)
        }
    }
}

/// If key present in `map`, add 1 to its value; else insert 1
fn map_inc<Key: Ord>(map: &mut BTreeMap<Key, usize>, key: Key) {
    map.entry(key)
        .and_modify(|n: &mut usize| {
            let (n_suc, overflows) = n.overflowing_add(1);
            if overflows {
                panic!("`count_srcs` overflows")
            } else {
                *n = n_suc;
            }
        })
        .or_insert(1usize);
}

/// Count the number of sources of labels
/// labels with no sources are not included
fn count_srcs(code: &[Bytecode]) -> BTreeMap<Label, usize> {
    let mut srcs_count = BTreeMap::new();
    for (code_offset, instr) in code.iter().enumerate() {
        match instr {
            Bytecode::Jump(_, label) => map_inc(&mut srcs_count, *label),
            Bytecode::Branch(_, l0, l1, _) => {
                map_inc(&mut srcs_count, *l0);
                map_inc(&mut srcs_count, *l1);
            },
            Bytecode::Label(_, label) => {
                if code_offset != 0 {
                    let prev_instr = code.get(code_offset - 1).expect("instruction");
                    // treat fall-through's to the label
                    if !prev_instr.is_branch() {
                        map_inc(&mut srcs_count, *label)
                    }
                }
            },
            _ => {},
        }
    }
    srcs_count
}
