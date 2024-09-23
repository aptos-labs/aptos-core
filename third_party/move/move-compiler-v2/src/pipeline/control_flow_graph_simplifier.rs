// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Prerequisites: none.
//!
//! Simplifies the control flow graph in the following ways:
//! 1. Reduces branch to jump or jump to jump:
//!     Any block
//!
//!     label L1
//!     goto L2
//!
//!     where L1 != L2 is removed, and any block jumping to L1 will jump to L2 directly
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
//!     If there is a block implicitly falls to the block L1, we add an explicit jump to the block
//!
//!     // block L1
//!     xxx
//!     // immediately followed by block L1
//!     label L1
//!     goto L2
//!     =>
//!     xxx
//!     goto L2
//!
//! 2. Removes edges where the source has only one successor and the target has only one predecessor:
//!     If there is block 1
//!
//!     xx
//!     [goto L2]?
//!
//!     (block 1 may not end with a jump to L2 in the case where block 2 follows immediately after block 1)
//!
//!     and block 2
//!
//!     label L2
//!     yy
//!
//!     which has only one predecessor block 1.
//!     Then we move block 2 to the end of block 1 so that block 1 becomes
//!
//!     xx
//!     yy
//!
//!     and block 2 is removed. Note that if block 2 falls implicitly to block 3 with label L3,
//!     we need an explicit jump so that block 1 becomes
//!
//!     xx
//!     yy
//!     goto L3
//!
//! 3. Removes all jumps to the next instruction:
//!     Whenever we have
//!
//!     goto L
//!     L
//!
//!     replace it by simply
//!
//!     L
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
//! We also remove unused labels.
//!
//! Side effects: removes all annotations.

use itertools::Itertools;
use move_model::model::FunctionEnv;
use move_stackless_bytecode::{
    function_target::FunctionData,
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{AbortAction, Bytecode, Label},
    stackless_control_flow_graph::{BlockContent, BlockId, DFSLeft, StacklessControlFlowGraph},
};
use std::collections::{BTreeMap, BTreeSet};

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
        let mut code_len = transformer.data.code.len();
        loop {
            transformer.transform();
            let new_code_len = transformer.data.code.len();
            if new_code_len == code_len {
                break;
            } else {
                code_len = new_code_len;
            }
        }
        transformer.data.annotations.clear();
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
        // eliminates the tricky case where a block may contain repeated successor/predecessor blocks
        self.eliminate_branch_to_same_target();
        let cfg1 = ControlFlowGraphCodeGenerator::new(std::mem::take(&mut self.data.code));
        debug_assert!(cfg1.check_consistency());
        // may introduce new redundant blocks, so perform before we remove redundant blocks
        let cfg2 = RedundantJumpRemover::transform(cfg1);
        let cfg3 = RedundantBlockRemover::transform(cfg2);
        debug_assert!(cfg3.check_consistency());
        self.data.code = cfg3.gen_code(false);
        // the above transformation may introduce branches to same target
        // for instance, if the original CFG is BB1 -> BB2, BB1 -> BB3 -> BB2, where BB3 is an redundant block
        self.eliminate_unused_labels();
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

    /// Eliminates labels that are not referred to by any instructions
    fn eliminate_unused_labels(&mut self) {
        let mut labels_used = BTreeSet::new();
        for bytecode in &self.data.code {
            match bytecode {
                Bytecode::Jump(_, l) | Bytecode::Call(.., Some(AbortAction(l, _))) => {
                    labels_used.insert(*l);
                },
                Bytecode::Branch(_, l1, l2, _) => {
                    labels_used.insert(*l1);
                    labels_used.insert(*l2);
                },
                _ => {},
            }
        }
        let code = std::mem::take(&mut self.data.code);
        self.data.code = code
            .into_iter()
            .filter(|bytecode| match bytecode {
                Bytecode::Label(_, l) => labels_used.contains(l),
                _ => true,
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
///
/// Invariants:
/// 1. Fields `code_blocks`, `successors`, and `predecessors` have the same set of keys.
/// 2. Block `A` has successor block `B` iff block `B` has predecessor block `A`, according
/// to fields `successors` and `predecessors` respectively.
/// 3. In `code_blocks`, entry and exit blocks cannot have code (mapped to empty vectors in `code_blocks`),
/// and all other blocks must have code.
/// 4. Entry block has exactly one successor, which is not the exit block.
/// 5. All blocks except for the exit block must have at least one successor.
/// 6. All non-trivial blocks with non-trivial predecessors must start with a label.
/// 7. The entry and exit blocks are distinct.
/// 8. The `successors` doesn't contain duplicate successors for a block.
/// 9. Code blocks are consistent with the successors and predecessors map;
///   - if a block explicitly branches or jumps to label L1 (and L2), then the successors of that block should contain exactly L1 (and L2);
///   - otherwise, the (non-exit) block should have exactly one successor.

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
    /// If a block has no successors, it is mapped to an empty vector,
    /// and the key for that block still exists.
    successors: BTreeMap<BlockId, Vec<BlockId>>,
    /// Maps a basic block to its predecessors.
    /// If a block has no predecessors, it is mapped to an empty vector,
    /// and the key for that block still exists.
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
            .sorted_by(|(_id1, _lower1, upper1), (_id2, _lower2, upper2)| upper2.cmp(upper1))
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
    fn successors_mut(&mut self, block_id: BlockId) -> &mut Vec<BlockId> {
        self.successors.get_mut(&block_id).expect("predecessors")
    }

    /// Gets the code of a block mutably
    fn code_block_mut(&mut self, block_id: BlockId) -> &mut Vec<Bytecode> {
        self.code_blocks.get_mut(&block_id).expect("code block")
    }

    /// Iterate over the blocks in DFS order, always visiting the left child to visit first
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
            // TODO: avoid cloning the code of `block`
            // can't use `.remove` instead to avoid copying,
            // because the following may look at visited block
            // `code_block` is non-empty since `block` is non-trivial
            let mut code_block = self.code_blocks.get(&block).expect("code block").clone();
            code_block = self.gen_code_for_block(code_block, block, iter_dfs_left.peek());
            generated.append(&mut code_block);
        }
        generated
    }

    /// Generates code for block. The way we generate code effectively does the transformation 3 in the module doc.
    /// Requires: `code_block` not empty
    fn gen_code_for_block(
        &self,
        mut code_block: Vec<Bytecode>,
        block: BlockId,
        next_block_to_visit: Option<&BlockId>,
    ) -> Vec<Bytecode> {
        debug_assert!(!code_block.is_empty());
        // since we may generate the blocks in a different order, we may need to add explicit jump or eliminate unnecessary jump.
        if Self::falls_to_next_block(&code_block) {
            // if we have block 0 followed by block 1 without jump/branch
            // and we don't visit block 1 after block 0, then we have to add an explicit jump
            let succ_block = self.get_the_non_trivial_successor(block);
            if next_block_to_visit.map_or_else(|| true, |block| *block != succ_block) {
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

    /// Gets the only successor of `block`; panics if there is no successor.
    fn get_the_successor(&self, block: BlockId) -> BlockId {
        let successors = self.successors.get(&block).expect("successors");
        debug_assert!(successors.len() == 1);
        *successors.first().expect("successor block")
    }

    /// Gets the only predecessor of `block`; panics if there is no predecessor.
    fn get_the_predecessor(&self, block: BlockId) -> BlockId {
        let predecessors = self.predecessors.get(&block).expect("predecessors");
        debug_assert!(predecessors.len() == 1);
        *predecessors.first().expect("predecessor block")
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
        // if the `to_block` is the first block, we know it has a label, since it has a nontrivial predecessor.
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
        Self::replace_blocks(self.successors_mut(block), old_succ, new_succ);
    }

    /// Replaces `old` by `new` in `blocks`
    fn replace_blocks(blocks: &mut Vec<BlockId>, old: BlockId, new: BlockId) {
        for block in blocks.iter_mut() {
            if *block == old {
                *block = new;
            }
        }
        let blocks_dedup = blocks.iter().copied().unique().collect();
        *blocks = blocks_dedup;
    }

    /// Checks if the control flow graph is consistent
    /// - `self.code_blocks`, `self.successors`, and `self.predecessors` have the same set of keys
    /// - `self.successors` is the reverse of `self.predecessors`
    /// - TODO: check if the code blocks are consistent with the successors and predecessors map
    fn check_consistency(&self) -> bool {
        // check invariant 1
        assert!(self.code_blocks.keys().eq(self.successors.keys()));
        assert!(self.code_blocks.keys().eq(self.predecessors.keys()));
        let pred_map_reversed = self.predecessors.iter().fold(
            BTreeMap::new(),
            |mut acc: BTreeMap<BlockId, Vec<BlockId>>, (block, preds)| {
                acc.entry(*block).or_default();
                for pred in preds {
                    acc.entry(*pred).or_default().push(*block);
                }
                acc
            },
        );
        // check invariant 2
        assert!(self.successors.keys().eq(pred_map_reversed.keys()));
        for block in self.successors.keys() {
            assert!(
                self.successors
                    .get(block)
                    .expect("successors")
                    .iter()
                    .collect::<BTreeSet<_>>()
                    == pred_map_reversed
                        .get(block)
                        .expect("pred_map_reversed")
                        .iter()
                        .collect::<BTreeSet<_>>()
            );
        }
        // check invariant 3
        for (block, code) in self.code_blocks.iter() {
            if self.is_trivial_block(*block) {
                assert!(code.is_empty())
            } else {
                assert!(!code.is_empty(), "block {} is empty", block);
            }
        }
        // check invariant 4
        assert!(self.get_the_successor(self.entry_block) != self.exit_block);
        // check invariant 5
        for (block, succs) in self.successors.iter() {
            if block != &self.exit_block {
                assert!(!succs.is_empty(), "block {} has no successors", block);
            }
        }
        // check invariant 6
        for (block, preds) in self.predecessors.iter() {
            if !self.is_trivial_block(*block)
                && preds.iter().any(|pred| !self.is_trivial_block(*pred))
            {
                assert!(matches!(
                    self.code_blocks
                        .get(block)
                        .expect("code block")
                        .first()
                        .expect("first instruction"),
                    Bytecode::Label(..)
                ));
            }
        }
        // check invariant 7
        assert!(self.entry_block != self.exit_block);
        // check invariant 8
        for (_, succs) in self.successors.iter() {
            succs.iter().all_unique();
        }
        // check invariant 9
        for (block, code) in self.code_blocks.iter() {
            if self.is_trivial_block(*block) {
                continue;
            }
            let last_instr = code.last().expect("last instruction");
            match last_instr {
                Bytecode::Branch(_, l0, l1, _) => {
                    let succs_labels: BTreeSet<Label> = self
                        .succs(*block)
                        .iter()
                        .map(|block| self.get_block_label(*block).expect("label"))
                        .collect();
                    assert!(
                        succs_labels.len() == {
                            if l0 == l1 {
                                1
                            } else {
                                2
                            }
                        }
                    );
                    assert!(succs_labels.contains(l0));
                    assert!(succs_labels.contains(l1));
                },
                Bytecode::Jump(_, l) => {
                    let suc_block = self.get_the_non_trivial_successor(*block);
                    let suc_label = self.get_block_label(suc_block).expect("label");
                    assert!(l == &suc_label);
                },
                _ => {
                    assert!(self.successors.get(block).expect("successors").len() == 1);
                },
            }
        }
        true
    }
}

/// Transformation state for transformation 1 in the module doc
struct RedundantBlockRemover(ControlFlowGraphCodeGenerator);

impl RedundantBlockRemover {
    /// Performs the transformation 1 described in the module doc
    /// After the transformation, the control flow graph is guaranteed to have no redundant blocks,
    /// except for empty loops `Label L; goto L`
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
            if self.is_redundant_block(block) {
                let succ_block = self.0.get_the_non_trivial_successor(block);
                if succ_block != block {
                    self.remove_redundant_block(block, succ_block);
                }
            }
        }
    }

    /// Checks if the given block is redundant; i.e., only a label and a jump
    fn is_redundant_block(&self, block_id: BlockId) -> bool {
        let block_instrs = self.0.block_instrs(block_id);
        match block_instrs.len() {
            1 => matches!(block_instrs[0], Bytecode::Label(..)),
            2 => matches!(
                (&block_instrs[0], &block_instrs[1]),
                (Bytecode::Label(..), Bytecode::Jump(..))
            ),
            _ => false,
        }
    }

    /// Removes the block `block_to_remove` and redirects all jumps to it to `redirect_to`
    fn remove_redundant_block(&mut self, block_to_remove: BlockId, redirect_to: BlockId) {
        debug_assert!(block_to_remove != redirect_to);
        debug_assert!(!self
            .0
            .successors
            .get(&block_to_remove)
            .expect("successors")
            .contains(&block_to_remove));
        let from = self.0.get_block_label(block_to_remove);
        let to = self.0.get_block_label(redirect_to);
        self.0.remove_block(
            block_to_remove,
            |this, pred| {
                // for all predecessors of `block_to_remove`, let them jump to `redirect_to` instead
                if pred != this.entry_block {
                    let code = this.code_blocks.get_mut(&pred).expect("code block");
                    if let (Some(from), Some(to)) = (from, to) {
                        Self::redirects_block(code, from, to);
                    }
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
                // we may have removed block
                if !self.0.successors.contains_key(&block) {
                    break;
                }
                // successors of `block` changes
                // consider
                //    L0: goto L1; L1: goto L2; L2: goto L3;
                // the successor of L0 is L1
                // after merging block L1 into block L0 we get
                //     L0: goto L2; L2: goto L3;
                // now the successor of L0 becomes L2
                // we continue to merge block L3 into block L0 ...
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

    /// If possible, append the code of block `to` to the end of block `from` and remove the `to` block
    /// Returns true if the edge is removed
    fn remove_jump_if_possible(&mut self, from: BlockId, to: BlockId) -> bool {
        debug_assert!(self.0.succs(from).contains(&to));
        debug_assert!(!self.0.is_trivial_block(from));
        debug_assert!(!self.0.is_trivial_block(to));

        if from == to || !self.can_remove_edge(from, to) {
            false
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

                    *this.successors_mut(pred) = this.succs(to).to_vec();
                },
                |this, succ| this.replace_preds(succ, to, from),
            );
            // In the extreme case, where the `to` block is only one label, and `from` is only one jump, `from` ends up empty
            if self
                .0
                .code_blocks
                .get(&from)
                .expect("code block")
                .is_empty()
            {
                self.0.remove_block(
                    from,
                    |this, pred| {
                        // since `from` is not a trivial block, it must have a successor block, perhaps the formal exit block
                        this.replace_succs(pred, from, this.get_the_successor(from))
                    },
                    |this, succ| {
                        if !this.preds(from).is_empty() {
                            // since `from` doesn't have a label, it have at most one predecessor
                            this.replace_preds(succ, from, this.get_the_predecessor(from))
                        } else {
                            this.preds_mut(succ).retain(|block| *block != from);
                        }
                    },
                );
            }
            true
        }
    }
}

/// Computes the map from each block to its predecessors
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
