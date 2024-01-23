// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Implements a compiler pass that
//! - eliminates all empty blocks
//! - removes edges where the source has only one successors and the target has only one predecessors
//! - removes all jumps to the next instruction
//! - removes unreachable codes

use move_model::model::FunctionEnv;
use move_stackless_bytecode::{
    function_target::FunctionData,
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{Bytecode, Label},
    stackless_control_flow_graph::{BlockId, StacklessControlFlowGraph},
};
use std::collections::BTreeMap;

pub struct ControlFlowGraphSimplifier {}

impl FunctionTargetProcessor for ControlFlowGraphSimplifier {
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
        let mut transformer = ControlFlowGraphSimplifierTransformation::new(data);
        transformer.transform();
        transformer.data
    }

    fn name(&self) -> String {
        "EliminateEmptyBlocksProcessor".to_owned()
    }
}

struct ControlFlowGraphSimplifierTransformation {
    data: FunctionData,
    cfg_code_generator: ControlFlowGraphCodeGenerator,
}

impl ControlFlowGraphSimplifierTransformation {
    pub fn new(data: FunctionData) -> Self {
        let cfg = StacklessControlFlowGraph::new_forward(&data.code);
        let cfg_code_generator = ControlFlowGraphCodeGenerator::new(cfg, &data.code);
        Self {
            data,
            cfg_code_generator,
        }
    }

    fn transform(&mut self) {
        let mut elim_empty_blocks_transformer =
            RemoveEmptyBlock::new(std::mem::take(&mut self.cfg_code_generator));
        elim_empty_blocks_transformer.transform();
        let mut remove_redundant_jump_transformer =
            RemoveRedundantJump::new(elim_empty_blocks_transformer.0);
        remove_redundant_jump_transformer.transform();
        self.data.code = remove_redundant_jump_transformer.0.gen_codes();
    }
}

#[derive(Default)]
struct ControlFlowGraphCodeGenerator {
    cfg: StacklessControlFlowGraph,
    code_blocks: BTreeMap<BlockId, Vec<Bytecode>>,
    // if `block_id` not in `pred_map`, the block either doesn't exist or doesn't have predecessors
    pred_map: BTreeMap<BlockId, Vec<BlockId>>,
}

impl ControlFlowGraphCodeGenerator {
    // TODO: take `Vec<Bytecode>` instead to avoid copying
    pub fn new(cfg: StacklessControlFlowGraph, codes: &[Bytecode]) -> Self {
        let code_blocks = cfg
            .blocks()
            .into_iter()
            .map(|block| (block, cfg.content(block).to_bytecodes(codes).to_vec()))
            .collect();
        let pred_map = pred_map(&cfg);
        Self {
            cfg,
            code_blocks,
            pred_map,
        }
    }

    /// Generates code from the control flow graph
    /// Unreachable codes are also discarded
    pub fn gen_codes(self) -> Vec<Bytecode> {
        let mut generated = Vec::new();
        let mut iter_dfs_left = self.cfg.iter_dfs_left().peekable();
        while let Some(block) = iter_dfs_left.next() {
            if self.is_trivial_block(block) {
                continue;
            }
            // TODO:
            // can't use `.remove` instead to avoid copying,
            // because the following may look at visited block
            let mut code_block = self.code_blocks.get(&block).expect("code block").clone();
            let maybe_next_to_visit = iter_dfs_left.peek();
            if Self::falls_to_next_block(&code_block) {
                // if we have block 0 followed by block 1 without jump/branch
                // and we don't visit block 1 after block 0, then we have to add an explicit jump
                let suc_block = self.get_the_non_trivial_successor(block);
                if maybe_next_to_visit.is_none() || *maybe_next_to_visit.unwrap() != suc_block {
                    self.add_explicit_jump(&mut code_block, suc_block);
                }
            } else if matches!(
                code_block.last().expect("last instruction"),
                Bytecode::Jump(..)
            ) {
                // no need to jump to the next instruction
                let suc_block = self.get_the_non_trivial_successor(block);
                if let Some(next_to_vist) = maybe_next_to_visit {
                    if *next_to_vist == suc_block {
                        debug_assert!(code_block.pop().is_some());
                    }
                }
            }
            generated.append(&mut code_block);
        }
        generated
    }

    /// Checks whether a block falls to the next block without jump, branch, abort, or return;
    /// i.e., the block is followed by the next in the original code
    fn falls_to_next_block(codes: &[Bytecode]) -> bool {
        let last_instr = codes.last().expect("last instr");
        !matches!(
            last_instr,
            Bytecode::Jump(..) | Bytecode::Branch(..) | Bytecode::Ret(..) | Bytecode::Abort(..)
        )
    }

    /// Gets the only successor of `block`; panics if this is not the case.
    /// (May not panic in the release version due to the use of debug_assert)
    fn get_the_successor(&self, block: BlockId) -> BlockId {
        let successors = self.cfg.successors(block);
        debug_assert!(successors.len() == 1);
        *successors.first().expect("successor block")
    }

    /// Gets the only successor of `block` which is not entry/exit block; panics if this is not the case.
    /// (May not panic in the release version due to the use of debug_assert)
    fn get_the_non_trivial_successor(&self, block: BlockId) -> BlockId {
        let the_suc = self.get_the_successor(block);
        debug_assert!(!self.is_trivial_block(the_suc));
        the_suc
    }

    /// Adds an explicit jump to `to_block` to the end of `codes`
    fn add_explicit_jump(&self, codes: &mut Vec<Bytecode>, to_block: BlockId) {
        debug_assert!(!self.is_trivial_block(to_block));
        let to_label = self.get_block_label(to_block).expect("label");
        let attr_id = self
            .code_blocks
            .get(&to_block)
            .expect("code block")
            .first()
            .expect("first instruction")
            .get_attr_id();
        codes.push(Bytecode::Jump(attr_id, to_label));
    }

    /// Returns the instructions of the block
    fn block_instrs(&self, block_id: BlockId) -> &[Bytecode] {
        self.code_blocks.get(&block_id).expect("block instructions")
    }

    /// Returns the label of the block or `None` if it doesn't start with a label
    fn get_block_label(&self, block_id: BlockId) -> Option<Label> {
        if let Bytecode::Label(_, label) = self
            .block_instrs(block_id)
            .first()
            .expect("first instruction")
        {
            Some(*label)
        } else {
            None
        }
    }

    /// Checks if the block is entry or exit block
    fn is_trivial_block(&self, block: BlockId) -> bool {
        block == self.cfg.entry_block() || block == self.cfg.exit_block()
    }
}

struct RemoveEmptyBlock(ControlFlowGraphCodeGenerator);

macro_rules! impl_deref {
    ($struct_name:ident, $target_type:ty) => {
        impl std::ops::Deref for $struct_name {
            type Target = $target_type;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

macro_rules! impl_deref_mut {
    ($struct_name:ident) => {
        impl std::ops::DerefMut for $struct_name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };
}

impl_deref!(RemoveEmptyBlock, ControlFlowGraphCodeGenerator);

impl_deref_mut!(RemoveEmptyBlock);

impl RemoveEmptyBlock {
    /// Wrapper
    pub fn new(generator: ControlFlowGraphCodeGenerator) -> Self {
        Self(generator)
    }

    /// Removes all empty blocks from the CFG except for empty loops (Label; jump Label)
    pub fn transform(&mut self) {
        for block in self.cfg.blocks() {
            if self.is_empty_block(block) {
                let suc_block = self.get_the_non_trivial_successor(block);
                if suc_block != block {
                    self.remove_empty_block(block, suc_block);
                }
            }
        }
    }

    /// Checks if the given block is empty; i.e., only a label and a jump
    fn is_empty_block(&self, block_id: BlockId) -> bool {
        let block_instrs = self.block_instrs(block_id);
        block_instrs.len() == 2
            && matches!(block_instrs[0], Bytecode::Label(..))
            && matches!(
                block_instrs.last().expect("instruction"),
                Bytecode::Jump(..)
            )
    }

    /// Removes block from the control flow graph, and redirects any block jumpping to it
    /// to `redirect_to` instead
    /// Requires: `block_to_remove` doesn't have itself as a successor;
    fn remove_empty_block(&mut self, block_to_remove: BlockId, redirect_to: BlockId) {
        debug_assert!(block_to_remove != redirect_to);
        debug_assert!(!self
            .cfg
            .successors(block_to_remove)
            .contains(&block_to_remove));
        let maybe_preds = self.pred_map.remove(&block_to_remove);
        if let Some(preds) = maybe_preds {
            for pred in preds {
                if pred != self.cfg.entry_block() {
                    let from = self.get_block_label(block_to_remove).expect("label");
                    let to = self.get_block_label(redirect_to).expect("label");
                    let pred_codes = self.code_blocks.get_mut(&pred).expect("code block");
                    Self::redirects_block(pred_codes, from, to);
                }
                // update successors of predecessors of `block_to_remove`
                for suc_of_pred in self.cfg.successors_mut(pred) {
                    if *suc_of_pred == block_to_remove {
                        *suc_of_pred = redirect_to;
                    }
                }
                // update predecessors of `redirect_to`
                // add preds of `remove_block` to `redirect_to`
                self.pred_map
                    .get_mut(&redirect_to)
                    .expect("predecessors")
                    .push(pred);
            }
        }
        // remove `block_to_remove`
        self.pred_map
            .get_mut(&redirect_to)
            .expect("predecessors")
            .retain(|pred| *pred != block_to_remove);
        self.code_blocks.remove(&block_to_remove);
        self.cfg.remove_block(block_to_remove);
    }

    /// Redirects a sequence of codes so that it jumps/branches to `to`
    /// where it originally jumps/branches to `from`.
    /// Does nothing if `codes` doesn't end with a jump/branch
    /// Requries: `codes` not empty
    fn redirects_block(codes: &mut [Bytecode], from: Label, to: Label) {
        let last_instr = codes.last_mut().expect("last instruction");
        match last_instr {
            Bytecode::Branch(_, l0, l1, _) => {
                subst_label(l0, from, to);
                subst_label(l1, from, to);
            },
            Bytecode::Jump(_, label) => {
                subst_label(label, from, to);
            },
            _ => {},
        }
    }
}

struct RemoveRedundantJump(pub ControlFlowGraphCodeGenerator);

impl RemoveRedundantJump {
    /// Wrapper
    pub fn new(cfg_generator: ControlFlowGraphCodeGenerator) -> Self {
        Self(cfg_generator)
    }

    /// Removes edges where the source has only one successors and the target has only one predecessors
    pub fn transform(&mut self) {
        for block in self.cfg.blocks() {
            // the later condition says that `block` is unreachable or has been removed
            if self.is_trivial_block(block) || !self.pred_map.contains_key(&block) {
                continue;
            }
            self.transform_edges_from(block)
        }
    }

    /// Requires: `block` still in the cfg, `block` is not entry/exit
    fn transform_edges_from(&mut self, block: BlockId) {
        for suc in self.cfg.successors(block).clone() {
            if !self.is_trivial_block(suc) && self.remove_jump_if_possible(block, suc) {
                // successors of `block` changes
                self.transform_edges_from(block);
            }
        }
    }

    /// An edge can be removed if `from` has only one successor, and `to` has only one predecessor
    fn can_remove_edge(&self, from: BlockId, to: BlockId) -> bool {
        debug_assert!(!self.is_trivial_block(from));
        debug_assert!(!self.is_trivial_block(to));
        self.cfg.successors(from).len() == 1
            && self.pred_map.get(&to).map_or(0, |preds| preds.len()) == 1
    }

    /// If possible, append `to` to `from` and remove `to` block
    fn remove_jump_if_possible(&mut self, from: BlockId, to: BlockId) -> bool {
        debug_assert!(self.cfg.successors(from).contains(&to));
        debug_assert!(!self.is_trivial_block(from));
        debug_assert!(!self.is_trivial_block(to));
        if from == to || !self.can_remove_edge(from, to) {
            return false;
        }
        let mut to_codes = self.code_blocks.remove(&to).expect("codes");
        if matches!(
            to_codes.first().expect("first instruction"),
            Bytecode::Label(..)
        ) {
            to_codes.remove(0);
        }
        let from_codes = self.code_blocks.get_mut(&from).expect("codes");
        if matches!(
            from_codes.last().expect("last instruction"),
            Bytecode::Jump(..)
        ) {
            from_codes.pop();
        }
        from_codes.append(&mut to_codes);
        self.cfg.successors_mut(from).clear();
        // for all successors of `to`, update their preds by substituting `to` to `from`
        for suc_of_to in self.cfg.successors(to).clone() {
            // successors of `from` becomes successors of `to`
            self.cfg.successors_mut(from).push(suc_of_to);
            for pred in self.pred_map.get_mut(&to).expect("predecessors") {
                if *pred == to {
                    *pred = from;
                }
            }
        }
        self.cfg.remove_block(to);
        self.pred_map.remove(&to);
        true
    }
}

impl_deref!(RemoveRedundantJump, ControlFlowGraphCodeGenerator);

impl_deref_mut!(RemoveRedundantJump);

/// Computes the map from a blcok to its predecessors
fn pred_map(cfg: &StacklessControlFlowGraph) -> BTreeMap<BlockId, Vec<BlockId>> {
    let mut pred_map = BTreeMap::new();
    for block in cfg.blocks() {
        for suc_block in cfg.successors(block) {
            let preds: &mut Vec<BlockId> = pred_map.entry(*suc_block).or_default();
            preds.push(block);
        }
    }
    pred_map
}

/// Replaces `label` with `to` if `label` equals `from`
fn subst_label(label: &mut Label, from: Label, to: Label) {
    if *label == from {
        *label = to;
    }
}
