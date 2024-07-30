// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod inefficient_binops;
pub mod optimizers;
pub mod redundant_pairs;

use inefficient_binops::TransformInefficientBinops;
use move_binary_format::{
    control_flow_graph::{ControlFlowGraph, VMControlFlowGraph},
    file_format::{Bytecode, CodeOffset, CodeUnit},
};
use optimizers::{BasicBlockOptimizer, FixedWindowProcessor};
use redundant_pairs::RedundantPairs;
use std::{collections::BTreeMap, mem};

// Note: `code` should not have spec block associations.
pub fn run(code: &mut CodeUnit) {
    let original_code = mem::take(&mut code.code);
    code.code = BasicBlockOptimizerPipeline::default().optimize(&original_code);
}

struct BasicBlockOptimizerPipeline {
    optimizers: Vec<Box<dyn BasicBlockOptimizer>>,
}

impl BasicBlockOptimizerPipeline {
    pub fn default() -> Self {
        Self {
            optimizers: vec![
                Box::new(FixedWindowProcessor::new(RedundantPairs)),
                Box::new(FixedWindowProcessor::new(TransformInefficientBinops)),
            ],
        }
    }

    pub fn optimize(&self, code: &[Bytecode]) -> Vec<Bytecode> {
        Self::flatten_blocks(self.get_optimized_blocks(code))
    }

    fn get_optimized_blocks(&self, code: &[Bytecode]) -> BTreeMap<CodeOffset, Vec<Bytecode>> {
        let cfg = VMControlFlowGraph::new(code);
        let mut optimized_blocks = BTreeMap::new();
        for block_id in cfg.blocks() {
            let start = cfg.block_start(block_id);
            let end = cfg.block_end(block_id); // `end` is inclusive
            let mut block = code[start as usize..=end as usize].to_vec();
            for bb_optimizer in self.optimizers.iter() {
                block = bb_optimizer.optimize(&block);
            }
            optimized_blocks.insert(start, block);
        }
        optimized_blocks
    }

    fn flatten_blocks(optimized_blocks: BTreeMap<CodeOffset, Vec<Bytecode>>) -> Vec<Bytecode> {
        let mut optimized_code = vec![];
        let mut block_mapping = BTreeMap::new();
        for (offset, mut block) in optimized_blocks {
            block_mapping.insert(offset, optimized_code.len() as CodeOffset);
            optimized_code.append(&mut block);
        }
        Self::remap_branch_targets(&mut optimized_code, &block_mapping);
        optimized_code
    }

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
