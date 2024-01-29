// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Simplifies the control flow graph by
//! (Implemented by `ControlFlowGraphSimplifier`)
//! - eliminating branch/jump to jump
//!     L1: goto L2 (L1 != L2)
//!     =>
//!     (removed)
//!
//!     goto L1
//!     =>
//!     goto L2
//!
//!     if ... goto L1 else ... / if ... goto ... else L1
//!     =>
//!     if ... goto L2 else ... / if ... goto ... else L2
//! - removes edges where the source has only one successor and the target has only one predecessor
//!     BB1: xx
//!          goto L2
//!     // BB1 BB2 don't have to be consecutive
//!     BB2: L2 (no other goto L2)
//!          yy
//!     =>
//!     BB1: xx
//!          yy
//! - removes all jumps to the next instruction:
//!     goto L;
//!     L;
//!     =>
//!     L;
//! - replaces branch to same target by jump
//!     if ... goto L else goto L
//!     =>
//!     goto L
//! (Implemented by `UnreachableCodeElimination`)
//! - removes unreachable codes
//! (TODO: eliminate jump to branch)

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
        let cfg_code_generator = ControlFlowGraphCodeGenerator::new(&data.code);
        Self {
            data,
            cfg_code_generator,
        }
    }

    /// Does the first four transformations described in the module doc
    fn transform(&mut self) {
        let mut elim_empty_blocks_transformer =
            RemoveEmptyBlock::new(std::mem::take(&mut self.cfg_code_generator));
        elim_empty_blocks_transformer.transform();
        let mut remove_redundant_jump_transformer =
            RemoveRedundantJump::new(elim_empty_blocks_transformer.0);
        remove_redundant_jump_transformer.transform();
        self.data.code = remove_redundant_jump_transformer.0.gen_code(true);
        self.eliminate_branch_to_same_target();
        // may introduce new empty blocks
        let mut elim_empty_blocks_transformer =
            RemoveEmptyBlock::new(ControlFlowGraphCodeGenerator::new(&self.data.code));
        elim_empty_blocks_transformer.transform();
        self.data.code = elim_empty_blocks_transformer.0.gen_code(true);
    }

    /// Transforms `if _ goto L else goto L` to `goto L`
    fn eliminate_branch_to_same_target(&mut self) {
        let codes = std::mem::take(&mut self.data.code);
        self.data.code = codes
            .into_iter()
            .map(|bytecode| match bytecode {
                Bytecode::Branch(attr_id, l0, l1, _) if l0 == l1 => Bytecode::Jump(attr_id, l0),
                _ => bytecode,
            })
            .collect();
    }
}

#[derive(Default)]
struct ControlFlowGraphCodeGenerator {
    // `BlockContent` is invalidated during transformations
    cfg: StacklessControlFlowGraph,
    code_blocks: BTreeMap<BlockId, Vec<Bytecode>>,
    // if `block_id` not in `pred_map`, the block either doesn't exist or doesn't have predecessors
    predecessors: BTreeMap<BlockId, Vec<BlockId>>,
}

impl ControlFlowGraphCodeGenerator {
    // TODO: take `Vec<Bytecode>` instead to avoid copying
    pub fn new(codes: &[Bytecode]) -> Self {
        let cfg = StacklessControlFlowGraph::new_forward(codes);
        let code_blocks = cfg
            .blocks()
            .into_iter()
            .map(|block| (block, cfg.content(block).to_bytecodes(codes).to_vec()))
            .collect();
        let pred_map = pred_map(&cfg);
        Self {
            cfg,
            code_blocks,
            predecessors: pred_map,
        }
    }

    /// Generates code from the control flow graph
    /// `visit_all`: determines whether to generate code for unreachable blocks
    pub fn gen_code(self, visit_all: bool) -> Vec<Bytecode> {
        let mut generated = Vec::new();
        let mut iter_dfs_left = self.cfg.iter_dfs_left(visit_all).peekable();
        while let Some(block) = iter_dfs_left.next() {
            if self.is_trivial_block(block) {
                continue;
            }
            // TODO:
            // can't use `.remove` instead to avoid copying,
            // because the following may look at visited block
            let mut code_block = self.code_blocks.get(&block).expect("code block").clone();
            code_block = self.gen_code_for_block(code_block, block, iter_dfs_left.peek());
            generated.append(&mut code_block);
        }
        generated
    }

    /// Generates code for block.
    fn gen_code_for_block(
        &self,
        mut code_block: Vec<Bytecode>,
        block: BlockId,
        next_block_to_visit: Option<&BlockId>,
    ) -> Vec<Bytecode> {
        // since we may generate the blocks in a different order, we may need to add explicit jump or eliminate unnecessary jump.
        if Self::falls_to_next_block(&code_block) {
            // if we have block 0 followed by block 1 without jump/branch
            // and we don't visit block 1 after block 0, then we have to add an explicit jump
            let suc_block = self.get_the_non_trivial_successor(block);
            if next_block_to_visit.is_none() || *next_block_to_visit.unwrap() != suc_block {
                self.add_explicit_jump(&mut code_block, suc_block);
            }
        } else if matches!(
            code_block.last().expect("last instruction"),
            Bytecode::Jump(..)
        ) {
            // no need to jump to the next instruction
            let suc_block = self.get_the_non_trivial_successor(block);
            if let Some(next_to_vist) = next_block_to_visit {
                if *next_to_vist == suc_block {
                    debug_assert!(code_block.pop().is_some());
                }
            }
        }
        code_block
    }

    /// Checks whether a block falls to the next block without jump, branch, abort, or return;
    /// i.e., the block is followed by the next in the original code
    fn falls_to_next_block(codes: &[Bytecode]) -> bool {
        let last_instr = codes.last().expect("last instr");
        !last_instr.is_branch()
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

/// `impl_deref!(Wrapper, WrappedType);` implements `Deref` trait for a wrapper struct with one field
/// `struct Wrapper(WrappedType);` where the `deref` method simply returns a reference to the wrapped value.
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

/// `impl_deref!(Wrapper)` implements `DerefMut` trait for a wrapper struct with one field,
/// where the `deref_mut` method simply returns a mutable reference to the wrapped value.
/// Note that `Wrapper` should've implemented `Deref`, which is a supertrait of `DerefMut`.
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

    /// Eliminating branch/jump to jump
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

    /// Removes the block from the control flow graph, and let any block jumpping to it
    /// to jump to `redirect_to` directly
    /// Requires: `block_to_remove` doesn't have itself as a successor. We don't remove `loop {}`.
    fn remove_empty_block(&mut self, block_to_remove: BlockId, redirect_to: BlockId) {
        debug_assert!(block_to_remove != redirect_to);
        debug_assert!(!self
            .cfg
            .successors(block_to_remove)
            .contains(&block_to_remove));
        let maybe_preds = self.predecessors.remove(&block_to_remove);
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
                self.predecessors
                    .get_mut(&redirect_to)
                    .expect("predecessors")
                    .push(pred);
            }
        }
        // remove `block_to_remove`
        self.predecessors
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
            if self.is_trivial_block(block) || !self.predecessors.contains_key(&block) {
                continue;
            }
            self.transform_edges_from(block)
        }
    }

    /// Removes all redundant edges from `block`
    /// Requires: `block` still in the cfg, `block` is not entry/exit
    fn transform_edges_from(&mut self, block: BlockId) {
        for suc in self.cfg.successors(block).clone() {
            if !self.is_trivial_block(suc) && self.remove_jump_if_possible(block, suc) {
                // successors of `block` changes
                // consider L0: goto L1; L1: goto L2; L2: goto L3;
                // suc L0: L1
                // after merging block L1 into block L0
                // L0: goto L2; L2: goto L3;
                // suc L0: L2
                // we continue to merge block L3 into block L0
                self.transform_edges_from(block);
            }
        }
    }

    /// An edge can be removed if `from` has only one successor, and `to` has only one predecessor
    fn can_remove_edge(&self, from: BlockId, to: BlockId) -> bool {
        debug_assert!(!self.is_trivial_block(from));
        debug_assert!(!self.is_trivial_block(to));
        self.cfg.successors(from).len() == 1
            && self.predecessors.get(&to).map_or(0, |preds| preds.len()) == 1
    }

    /// If possible, append the code of block `to` to the back of block `from` and remove the `to` block
    /// Returns true if the edge is removed
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
            for pred in self.predecessors.get_mut(&to).expect("predecessors") {
                if *pred == to {
                    *pred = from;
                }
            }
        }
        self.cfg.remove_block(to);
        self.predecessors.remove(&to);
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

/// Removes unreachable codes by only generates code blocks reachable from the entry block
pub struct UnreachableCodeElimination {}

impl FunctionTargetProcessor for UnreachableCodeElimination {
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
        let generator = ControlFlowGraphCodeGenerator::new(&data.code);
        data.code = generator.gen_code(false);
        data
    }

    fn name(&self) -> String {
        "UnreachableCodeElimination".to_owned()
    }
}
