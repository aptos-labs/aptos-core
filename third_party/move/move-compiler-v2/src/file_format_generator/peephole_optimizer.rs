// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module contains the peephole optimizer for the Move file format bytecode.
//! Peephole optimizations assume that the bytecode is valid, and all user-facing
//! error checks have already been performed.

pub mod inefficient_loads;
pub mod optimizers;
pub mod reducible_pairs;

use inefficient_loads::InefficientLoads;
use move_binary_format::{
    control_flow_graph::{ControlFlowGraph, VMControlFlowGraph},
    file_format::{Bytecode, CodeOffset},
};
use optimizers::{BasicBlockOptimizer, TransformedCodeChunk, WindowProcessor};
use reducible_pairs::ReduciblePairs;
use std::collections::BTreeMap;

/// Pre-requisite: `code` should not have spec block associations.
/// Run peephole optimizers on the given `code`, possibly modifying it.
/// Returns the optimized code, along with mapping to original offsets
/// in `code`.
pub fn optimize(code: &[Bytecode]) -> TransformedCodeChunk {
    BasicBlockOptimizerPipeline::default().optimize(code)
}

/// A pipeline of basic block optimizers.
/// Each optimizer is applied to each basic block in the code, in order.
struct BasicBlockOptimizerPipeline {
    optimizers: Vec<Box<dyn BasicBlockOptimizer>>,
}

impl BasicBlockOptimizerPipeline {
    /// Default optimization pipeline of basic block optimizers.
    pub fn default() -> Self {
        Self {
            optimizers: vec![
                Box::new(WindowProcessor::new(ReduciblePairs)),
                Box::new(WindowProcessor::new(InefficientLoads)),
            ],
        }
    }

    /// Run the basic block optimization pipeline on the given `code`,
    /// returning new (possibly optimized) code.
    pub fn optimize(&self, code: &[Bytecode]) -> TransformedCodeChunk {
        let mut code_chunk = TransformedCodeChunk::make_from(code);
        let mut cfg = VMControlFlowGraph::new(&code_chunk.code);
        loop {
            let optimized_blocks = self.get_optimized_blocks(&code_chunk, &cfg);
            let optimized_code_chunk = Self::flatten_blocks(optimized_blocks);
            let optimized_cfg = VMControlFlowGraph::new(&optimized_code_chunk.code);
            if optimized_cfg.num_blocks() == cfg.num_blocks() {
                // Proxy for convergence of basic block optimizations.
                // This is okay for peephole optimizations that merge basic blocks.
                // But may need to revisit if we have peephole optimizations that can
                // split a basic block.
                return optimized_code_chunk;
            } else {
                // Number of basic blocks changed, re-run the basic-block
                // optimization pipeline again on the new basic blocks.
                cfg = optimized_cfg;
                code_chunk = optimized_code_chunk.remap(code_chunk.original_offsets);
            }
        }
    }

    /// Returns a mapping from the original code's basic block start offsets to the optimized
    /// basic blocks.
    fn get_optimized_blocks(
        &self,
        original_block: &TransformedCodeChunk,
        cfg: &VMControlFlowGraph,
    ) -> BTreeMap<CodeOffset, TransformedCodeChunk> {
        let mut optimized_blocks = BTreeMap::new();
        for block_id in cfg.blocks() {
            let start = cfg.block_start(block_id);
            let end = cfg.block_end(block_id); // `end` is inclusive
            let mut block = original_block.extract(start, end);
            for bb_optimizer in self.optimizers.iter() {
                block = bb_optimizer
                    .optimize(&block.code)
                    .remap(block.original_offsets);
            }
            optimized_blocks.insert(start, block);
        }
        optimized_blocks
    }

    /// Flatten the individually optimized basic blocks into a single code vector.
    fn flatten_blocks(
        optimized_blocks: BTreeMap<CodeOffset, TransformedCodeChunk>,
    ) -> TransformedCodeChunk {
        let mut optimized_code = TransformedCodeChunk::empty();
        let mut block_mapping = BTreeMap::new();
        for (offset, block) in optimized_blocks {
            block_mapping.insert(offset, optimized_code.code.len() as CodeOffset);
            optimized_code.extend(block, 0);
        }
        Self::remap_branch_targets(&mut optimized_code.code, &block_mapping);
        optimized_code
    }

    /// Use `remap` to update branch targets in the given `code`.
    fn remap_branch_targets(code: &mut [Bytecode], remap: &BTreeMap<CodeOffset, CodeOffset>) {
        for bc in code.iter_mut() {
            match bc {
                Bytecode::Branch(offset) | Bytecode::BrTrue(offset) | Bytecode::BrFalse(offset) => {
                    *offset = *remap.get(offset).expect("mapping should exist");
                },
                _ => {},
            }
        }
    }
}
