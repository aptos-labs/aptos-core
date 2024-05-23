// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Prerequisites: none.
//!
//! Simplifies the control flow graph in the following ways:
//! (Implemented by `ControlFlowGraphSimplifier`)
//! 1. Eliminates branch or jump to jump:
//!     Any empty block
//!
//!     L1: goto L2 where L1 != L2
//!
//!     is removed, and any block jumping to L1 will jump to L2 directly
//!
//!     goto L1
//!     => (the => here means the instruction above will be replaced by the instruction below)
//!     goto L2
//!
//!     if ... goto L1 else ...
//!     =>
//!     if ... goto L2 else ...
//!
//!     if ... goto ... else L1
//!     =>
//!     if ... goto ... else L2
//!
//! 2. Removes edges where the source has only one successor and the target has only one predecessor:
//!     If there is block
//!     BB1: xx
//!          goto L2
//!     and block
//!
//!     BB2: L2
//!          yy
//!
//!     which has only one predecessor BB1.
//!     Then we move BB2 to the end of BB1 so that BB1 becomes
//!
//!     BB1: xx
//!          yy
//!
//!     and BB2 is removed.
//!
//! 3. Removes all jumps to the next instruction:
//!     Whenever we have
//!
//!     goto L;
//!     L;
//!
//!     replace it by simply
//!
//!     L;
//!
//! 4. Replaces branch to same target by jump:
//!     Whenever we have
//!
//!     if ... goto L else goto L
//!
//!     replace it by simply
//!
//!     goto L
//!
//! Side effects: remove all annotations.

use itertools::Itertools;
use move_model::model::FunctionEnv;
use move_stackless_bytecode::{
    function_target::FunctionData,
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{Bytecode, Label},
    stackless_control_flow_graph::{BlockContent, BlockId, DFSLeft, StacklessControlFlowGraph},
};
use std::collections::BTreeMap;

/// Simplifies the control flow graph as described in the module doc
pub struct ControlFlowGraphSimplifier {}

impl FunctionTargetProcessor for ControlFlowGraphSimplifier {
    /// Does the first four transformations described in the module doc
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
        "ControlFlowGraphSimplifier".to_owned()
    }
}

/// State of the transformation
struct ControlFlowGraphSimplifierTransformation {
    data: FunctionData,
}

impl ControlFlowGraphSimplifierTransformation {
    pub fn new(data: FunctionData) -> Self {
        Self { data }
    }

    /// Does the first four transformations described in the module doc
    fn transform(&mut self) {
        // eliminates the trick case where a block may contain repeated successor/predecessor blocks
        self.eliminate_branch_to_same_target();
        let cfg1 = ControlFlowGraphCodeGenerator::new(std::mem::take(&mut self.data.code));
        // may introduce new empty blocks, so perform before we remove empty blocks
        let cfg2 = RedundantJumpRemover::transform(cfg1);
        let cfg3 = EmptyBlockRemover::transform(cfg2);
        self.data.code = cfg4.gen_code(true);
        // the above transformation may introduce branches to same target
        // for instance, if the original CFG is BB1 -> BB2, BB1 -> BB3 -> BB2, where BB3 is an empty block
        self.eliminate_branch_to_same_target();
        self.data.annotations.clear()
    }

    /// Transforms `if _ goto L else goto L` to `goto L`, which is the transformation 4 in the module doc
    fn eliminate_branch_to_same_target(&mut self) {
        let code = std::mem::take(&mut self.data.code);
        self.data.code = code
            .into_iter()
            .map(|bytecode| match bytecode {
                Bytecode::Branch(attr_id, l0, l1, _) if l0 == l1 => Bytecode::Jump(attr_id, l0),
                _ => bytecode,
            })
            .collect();
    }
}

/// Control flow graph that owns the code for transformation
///
/// To do a control flow graph based transformation:
/// 1. construct a `ControlFlowGraphCodeGenerator` from the code,
/// 2. perform the transformation on the `ControlFlowGraphCodeGenerator`
/// 3. generate the code back from the `ControlFlowGraphCodeGenerator`
#[derive(Debug)]
struct ControlFlowGraphCodeGenerator {
    /// The control flow graph.
    /// `BlockContent` is invalidated during transformations
    entry_block: BlockId,
    exit_block: BlockId,
    /// The code of each basic block under transformation.
    /// Entry/exit block is mapped to an empty vector.
    code_blocks: BTreeMap<BlockId, Vec<Bytecode>>,
    /// Maps a basic block to its successors.
    successors: BTreeMap<BlockId, Vec<BlockId>>,
    /// Maps a basic block to its predecessors.
    predecessors: BTreeMap<BlockId, Vec<BlockId>>,
}

impl ControlFlowGraphCodeGenerator {
    pub fn new(mut code: Vec<Bytecode>) -> Self {
        let cfg = StacklessControlFlowGraph::new_forward(&code);
        let mut code_blocks = BTreeMap::new();
        // traverse the blocks in decreasing order of its start code index
        for (block_id, lower, upper) in cfg
            .blocks()
            .into_iter()
            .filter_map(|block_id| match cfg.content(block_id) {
                BlockContent::Basic { lower, upper } => Some((block_id, *lower, *upper)),
                BlockContent::Dummy => None,
            })
            .sorted_by_key(|x| x.2)
            .rev()
        {
            let block_code = code.drain(lower as usize..=upper as usize).collect_vec();
            code_blocks.insert(block_id, block_code);
        }
        code_blocks.insert(cfg.entry_block(), Vec::new());
        code_blocks.insert(cfg.exit_block(), Vec::new());
        let pred_map = pred_map(&cfg);
        Self {
            entry_block: cfg.entry_block(),
            exit_block: cfg.exit_block(),
            code_blocks,
            successors: cfg.get_successors_map(),
            predecessors: pred_map,
        }
    }

    /// Returns all the blocks in the control flow graph
    pub fn blocks(&self) -> Vec<BlockId> {
        self.successors.keys().cloned().collect()
    }

    /// Gets the predecessors of a block
    fn preds(&self, block_id: BlockId) -> &[BlockId] {
        self.predecessors.get(&block_id).expect("predecessors")
    }

    /// Gets the successors of a block
    fn succs(&self, block_id: BlockId) -> &[BlockId] {
        self.successors.get(&block_id).expect("successors")
    }

    /// Gets the predecessors of a block mutably
    fn preds_mut(&mut self, block_id: BlockId) -> &mut Vec<BlockId> {
        self.predecessors.get_mut(&block_id).expect("predecessors")
    }

    /// Gets the successors of a block mutably
    fn succs_mut(&mut self, block_id: BlockId) -> &mut Vec<BlockId> {
        self.successors.get_mut(&block_id).expect("predecessors")
    }

    /// Gets the code of a block mutably
    fn code_block_mut(&mut self, block_id: BlockId) -> &mut Vec<Bytecode> {
        self.code_blocks.get_mut(&block_id).expect("code block")
    }

    /// Iterate over the blocks in DFS order
    /// `visit_all`: determines whether to visit unreachable blocks
    pub fn iter_dfs_left(&self, visit_all: bool) -> impl Iterator<Item = BlockId> + '_ {
        DFSLeft::new(&self.successors, self.entry_block, visit_all)
    }

    /// Generates code from the control flow graph
    /// `visit_all`: determines whether to generate code for unreachable blocks
    pub fn gen_code(self, visit_all: bool) -> Vec<Bytecode> {
        let mut generated = Vec::new();
        let mut iter_dfs_left = self.iter_dfs_left(visit_all).peekable();
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

    /// Generates code for block. The way we generate code effectively does the transformation 3 in the module doc.
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
            let succ_block = self.get_the_non_trivial_successor(block);
            if next_block_to_visit.is_none() || *next_block_to_visit.unwrap() != succ_block {
                self.add_explicit_jump(&mut code_block, succ_block);
            }
        } else if matches!(
            code_block.last().expect("last instruction"),
            Bytecode::Jump(..)
        ) {
            // no need to jump to the next instruction
            let succ_block = self.get_the_non_trivial_successor(block);
            if let Some(next_to_vist) = next_block_to_visit {
                if *next_to_vist == succ_block {
                    remove_tail_jump(&mut code_block);
                }
            }
        }
        code_block
    }

    /// Checks whether a block falls to the next block without jump, branch, abort, or return;
    /// i.e., the block is followed by the next in the original code
    fn falls_to_next_block(code: &[Bytecode]) -> bool {
        let last_instr = code.last().expect("last instr");
        !last_instr.is_always_branching()
    }

    /// Gets the only successor of `block`; panics if there is no successors.
    /// (May not panic in the release version due to the use of debug_assert)
    fn get_the_successor(&self, block: BlockId) -> BlockId {
        let successors = self.successors.get(&block).expect("successors");
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

    /// Adds an explicit jump to `to_block` to the end of `code`
    fn add_explicit_jump(&self, code: &mut Vec<Bytecode>, to_block: BlockId) {
        debug_assert!(!self.is_trivial_block(to_block));
        let to_label = self.get_block_label(to_block).expect("label");
        let attr_id = code.last().expect("instruction").get_attr_id();
        code.push(Bytecode::Jump(attr_id, to_label));
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
        block == self.entry_block || block == self.exit_block
    }

    /// Removes a block from the control flow graph
    /// `pred_action`: action to take for each predecessor of the block
    /// `succ_action`: action to take for each successor of the block
    fn remove_block<F, G>(&mut self, block_to_remove: BlockId, pred_action: F, succ_action: G)
    where
        F: FnOnce(&mut Self, BlockId) + Copy,
        G: FnOnce(&mut Self, BlockId) + Copy,
    {
        let preds = self
            .predecessors
            .get(&block_to_remove)
            .expect("predecessors")
            .clone();
        let succs = self
            .successors
            .get(&block_to_remove)
            .expect("successors")
            .clone();
        for pred in preds {
            pred_action(self, pred);
        }
        for suc in succs {
            succ_action(self, suc);
        }
        self.predecessors
            .remove(&block_to_remove)
            .expect("predecessors");
        self.successors
            .remove(&block_to_remove)
            .expect("successors");
        self.code_blocks.remove(&block_to_remove);
    }

    /// Removes `pred` from the predecessors of `block`
    fn remove_pred(&mut self, block: BlockId, pred: BlockId) {
        let preds = self.predecessors.get_mut(&block).expect("predecessors");
        preds.retain(|p| *p != pred);
    }

    /// Replaces the predecessor `old_pred` of `block` by `new_pred`
    fn replace_preds(&mut self, block: BlockId, old_pred: BlockId, new_pred: BlockId) {
        Self::replace_blocks(self.preds_mut(block), old_pred, new_pred);
    }

    /// Replaces the predecessor `old_pred` of `block` by `new_pred`
    fn replace_succs(&mut self, block: BlockId, old_succ: BlockId, new_succ: BlockId) {
        Self::replace_blocks(self.succs_mut(block), old_succ, new_succ);
    }

    /// Replaces `old` by `new` in `blocks`
    fn replace_blocks(blocks: &mut [BlockId], old: BlockId, new: BlockId) {
        for block in blocks {
            if *block == old {
                *block = new;
            }
        }
    }
}

/// Transformation state for transformation 1 in the module doc
struct EmptyBlockRemover(ControlFlowGraphCodeGenerator);

impl EmptyBlockRemover {
    /// Performs the transformation 1 described in the module doc
    pub fn transform(generator: ControlFlowGraphCodeGenerator) -> ControlFlowGraphCodeGenerator {
        let mut processer = Self::new(generator);
        processer.process();
        processer.0
    }

    /// Wrapper
    fn new(generator: ControlFlowGraphCodeGenerator) -> Self {
        Self(generator)
    }

    /// Performs the transformation 1 described in the module doc
    fn process(&mut self) {
        for block in self.0.blocks() {
            if self.is_empty_block(block) {
                let succ_block = self.0.get_the_non_trivial_successor(block);
                if succ_block != block {
                    self.remove_empty_block(block, succ_block);
                }
            }
        }
    }

    /// Checks if the given block is empty; i.e., only a label and a jump
    fn is_empty_block(&self, block_id: BlockId) -> bool {
        let block_instrs = self.0.block_instrs(block_id);
        block_instrs.len() == 2
            && matches!(block_instrs[0], Bytecode::Label(..))
            && matches!(
                block_instrs.last().expect("instruction"),
                Bytecode::Jump(..)
            )
            || block_instrs.len() == 1 && matches!(block_instrs[0], Bytecode::Label(..))
    }

    /// Removes the block `block_to_remove` and redirects all jumps to it to `redirect_to`
    fn remove_empty_block(&mut self, block_to_remove: BlockId, redirect_to: BlockId) {
        debug_assert!(block_to_remove != redirect_to);
        debug_assert!(!self
            .0
            .successors
            .get(&block_to_remove)
            .expect("successors")
            .contains(&block_to_remove));
        let from = self.0.get_block_label(block_to_remove).expect("label");
        let to = self.0.get_block_label(redirect_to).expect("label");
        self.0.remove_block(
            block_to_remove,
            |this, pred| {
                // for all predecessors of `block_to_remove`, let them jump to `redirect_to` instead
                if pred != this.entry_block {
                    let code = this.code_blocks.get_mut(&pred).expect("code block");
                    Self::redirects_block(code, from, to);
                }
                // update successors of pred by replacing `block_to_remove` with `redirect_to`
                this.replace_succs(pred, block_to_remove, redirect_to);
            },
            // replace `block_to_remove` by predecessors of `block_to_remove`
            // in predecessors of `redirect_to`
            |this, succ| {
                debug_assert!(succ == redirect_to);
                this.remove_pred(succ, block_to_remove);
                let mut preds_of_block_to_remove = this.preds(block_to_remove).to_vec();
                this.preds_mut(succ).append(&mut preds_of_block_to_remove);
            },
        );
    }

    /// Redirects a code block so that it jumps/branches to `to`
    /// where it originally jumps/branches to `from`.
    /// Does nothing if `code` doesn't end with a jump/branch
    /// Requries: `code` not empty
    fn redirects_block(code: &mut [Bytecode], from: Label, to: Label) {
        let last_instr = code.last_mut().expect("last instruction");
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

/// Transformation state for transformation 2 in the module doc
struct RedundantJumpRemover(pub ControlFlowGraphCodeGenerator);

impl RedundantJumpRemover {
    /// Performs the transformation 2 described in the module doc
    pub fn transform(generator: ControlFlowGraphCodeGenerator) -> ControlFlowGraphCodeGenerator {
        let mut processor = Self::new(generator);
        processor.process();
        processor.0
    }

    /// Wrapper
    fn new(cfg_generator: ControlFlowGraphCodeGenerator) -> Self {
        Self(cfg_generator)
    }

    /// Performs the transformation 2 described in the module doc
    fn process(&mut self) {
        for block in self.0.blocks() {
            // the later condition says that `block` is unreachable or has been removed
            if self.0.is_trivial_block(block) || !self.0.predecessors.contains_key(&block) {
                continue;
            }
            self.remove_redundant_edges_from(block)
        }
    }

    /// Removes all redundant edges from `block`
    /// Requires: `block` still in the cfg, `block` is not entry/exit
    fn remove_redundant_edges_from(&mut self, block: BlockId) {
        for suc in self.0.successors.get(&block).expect("successors").clone() {
            if !self.0.is_trivial_block(suc) && self.remove_jump_if_possible(block, suc) {
                // successors of `block` changes
                // consider L0: goto L1; L1: goto L2; L2: goto L3;
                // suc L0: L1
                // after merging block L1 into block L0
                // L0: goto L2; L2: goto L3;
                // suc L0: L2
                // we continue to merge block L3 into block L0
                self.remove_redundant_edges_from(block);
            }
        }
    }

    /// An edge can be removed if `from` has only one successor, and `to` has only one predecessor
    fn can_remove_edge(&self, from: BlockId, to: BlockId) -> bool {
        debug_assert!(!self.0.is_trivial_block(from));
        debug_assert!(!self.0.is_trivial_block(to));
        debug_assert!(self.0.succs(from).contains(&to));
        self.0.succs(from).len() == 1 && self.0.preds(to).len() == 1
    }

    /// If possible, append the code of block `to` to the back of block `from` and remove the `to` block
    /// Returns true if the edge is removed
    fn remove_jump_if_possible(&mut self, from: BlockId, to: BlockId) -> bool {
        debug_assert!(self.0.succs(from).contains(&to));
        debug_assert!(!self.0.is_trivial_block(from));
        debug_assert!(!self.0.is_trivial_block(to));

        if from == to || !self.can_remove_edge(from, to) {
            return false;
        } else {
            self.0.remove_block(
                to,
                |this, pred| {
                    debug_assert!(pred == from);

                    let mut to_code = this.code_blocks.remove(&to).expect("code block");
                    remove_front_label(&mut to_code);
                    let from_code = this.code_block_mut(pred);
                    remove_tail_jump(from_code);
                    from_code.append(&mut to_code);

                    *this.succs_mut(pred) = this.succs(to).to_vec();
                },
                |this, succ| this.replace_preds(succ, to, from),
            );
            true
        }
    }
}

/// Computes the map from a block to its predecessors
fn pred_map(cfg: &StacklessControlFlowGraph) -> BTreeMap<BlockId, Vec<BlockId>> {
    let mut pred_map = BTreeMap::new();
    let blocks = cfg.blocks();
    for block in &blocks {
        for succ_block in cfg.successors(*block) {
            let preds: &mut Vec<BlockId> = pred_map.entry(*succ_block).or_default();
            preds.push(*block);
        }
    }
    for block in &blocks {
        pred_map.entry(*block).or_default();
    }
    pred_map
}

/// Replaces `label` with `to` if `label` equals `from`
fn subst_label(label: &mut Label, from: Label, to: Label) {
    if *label == from {
        *label = to;
    }
}

/// If the tail of `code` is a jump, removes it
fn remove_tail_jump(code: &mut Vec<Bytecode>) {
    if matches!(code.last().expect("last instruction"), Bytecode::Jump(..)) {
        code.pop();
    }
}

/// If the head of `code` is a label, removes it
fn remove_front_label(code: &mut Vec<Bytecode>) {
    if matches!(
        code.first().expect("first instruction"),
        Bytecode::Label(..)
    ) {
        code.remove(0);
    }
}
