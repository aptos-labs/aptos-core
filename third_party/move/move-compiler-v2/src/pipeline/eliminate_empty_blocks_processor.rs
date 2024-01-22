// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use move_model::model::FunctionEnv;
use move_stackless_bytecode::{
    function_target::FunctionData,
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{Bytecode, Label},
    stackless_control_flow_graph::{
        BlockId, StacklessControlFlowGraph,
    },
};
use std::collections::BTreeMap;

pub struct EliminateEmptyBlocksProcessor {}

impl FunctionTargetProcessor for EliminateEmptyBlocksProcessor {
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
        let mut transformer = EliminateEmptyBlocksTransformation::new(data);
        transformer.transform();
        transformer.data
    }

    fn name(&self) -> String {
        "EliminateEmptyBlocksProcessor".to_owned()
    }
}

struct EliminateEmptyBlocksTransformation {
	data: FunctionData,
    cfg_code_generator: ControlFlowGraphCodeGenerator,
}

impl EliminateEmptyBlocksTransformation {
    pub fn new(data: FunctionData) -> Self {
		let cfg = StacklessControlFlowGraph::new_forward(&data.code);
        let cfg_code_generator = ControlFlowGraphCodeGenerator::new(cfg, &data.code);
		Self {
			data,
			cfg_code_generator
		}
    }

	fn transform(&mut self) {
		self.transform_cfg();
		self.data.code = std::mem::take(&mut self.cfg_code_generator).gen_codes();
	}

	fn transform_cfg(&mut self) {
		for block in self.cfg_code_generator.cfg.blocks() {
			if self.is_empty_block(block) {
				let suc_blocks = self.cfg_code_generator.cfg.successors(block);
				debug_assert!(suc_blocks.len() == 1);
				let suc_block = suc_blocks.first().expect("successor block");
				if *suc_block != block {
					self.cfg_code_generator.remove_empty_block(block, *suc_block);
				}
			}
		}
	}

    /// Returns the instructions of the block
    fn block_instrs(&self, block_id: BlockId) -> &[Bytecode] {
        self.cfg_code_generator.block_instrs(block_id)
    }

    /// Checks if the given block is empty block
    fn is_empty_block(&self, block_id: BlockId) -> bool {
        let block_instrs = self.block_instrs(block_id);
        block_instrs.len() == 2
            && matches!(block_instrs[0], Bytecode::Label(..))
            && matches!(
                block_instrs.last().expect("instruction"),
                Bytecode::Jump(..)
            )
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
    fn gen_codes(self) -> Vec<Bytecode> {
        let mut generated = Vec::new();
        let mut iter_dfs_left = self.cfg.iter_dfs_left().peekable();
        while let Some(block) = iter_dfs_left.next() {
            if block == self.cfg.entry_block() || block == self.cfg.exit_block() {
                continue;
            }
			// can't use `.remove` instead to avoid copying,
			// because the following may look at visited block
            let mut code_block = self.code_blocks.get(&block).expect("code block").clone();
            // if we have block 0 followed by block 1 without jump/branch
            // and we don't visit block 1 after block 0, then we have to add an explicit jump
            if Self::falls_to_next_block(&code_block) {
                debug_assert!(self.cfg.successors(block).len() == 1);
                let suc_block = *self.cfg.successors(block).first().expect("successor block");
                debug_assert!(
                    suc_block != self.cfg.exit_block(),
                    "path ending without return/abort"
                );
                let maybe_next_to_visit = iter_dfs_left.peek();
                if maybe_next_to_visit.is_none() || *maybe_next_to_visit.unwrap() != suc_block {
                    let attr_id = code_block.last().expect("last instr").get_attr_id();
                    // assuming that any block with a non-trivial predecessor block starts with a label
                    code_block.push(Bytecode::Jump(attr_id, self.get_block_label(suc_block).expect("label")));
                }
            }
            generated.append(&mut code_block);
        }
        generated
    }

    /// Checks whether a block falls to the next block without jump, branch, abort, or return
    fn falls_to_next_block(codes: &[Bytecode]) -> bool {
        let last_instr = codes.last().expect("last instr");
        !matches!(
            last_instr,
            Bytecode::Jump(..) | Bytecode::Branch(..) | Bytecode::Ret(..) | Bytecode::Abort(..)
        )
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
}

impl ControlFlowGraphCodeGenerator {
    /// Remove block from the control flow graph, and redirects any block jumpping to it
    /// to `redirect_to` instead
    /// Requires: `block_to_remove` doesn't have itself as a successor;
    fn remove_empty_block(&mut self, block_to_remove: BlockId, redirect_to: BlockId) {
        debug_assert!(block_to_remove != redirect_to);
		debug_assert!(!self.cfg.successors(block_to_remove).contains(&block_to_remove));
        let maybe_preds = self.pred_map.remove(&block_to_remove);
        if let Some(preds) = maybe_preds {
			for pred in preds {
                if pred != self.cfg.entry_block() {
					let from = self.get_block_label(block_to_remove).expect("label");
					let to = self.get_block_label(redirect_to).expect("label");
					let pred_codes = self
						.code_blocks
						.get_mut(&pred)
						.expect("code block");
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
		self.pred_map.remove(&block_to_remove);
		self.cfg.remove_block(block_to_remove);
    }

	/// Redirects a sequence of codes so that it jumps/branches to `to`
	/// where it originally jumps/branches to `from`.
	/// Does nothing if `codes` doesn't end with a jump/branch
	/// Requries: `codes` not empty
	fn redirects_block(codes: &mut [Bytecode], from: Label, to: Label) {
		let last_instr = codes
			.last_mut()
			.expect("last instruction");
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

/// Computes the map from a blcok to its predecessors
fn pred_map(cfg: &StacklessControlFlowGraph) -> BTreeMap<BlockId, Vec<BlockId>> {
    let mut pred_map = BTreeMap::new();
    for block in cfg.blocks() {
        for suc_block in cfg.successors(block) {
            let preds: &mut Vec<BlockId> = pred_map
                .entry(*suc_block)
				.or_default();
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
