// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::file_format::Bytecode;
use std::mem;

pub trait BasicBlockOptimizer {
    fn optimize(&self, block: &[Bytecode]) -> Vec<Bytecode>;
}

pub struct FixedWindowProcessor<T: FixedWindowOptimizer>(T);

impl<T: FixedWindowOptimizer> BasicBlockOptimizer for FixedWindowProcessor<T> {
    fn optimize(&self, block: &[Bytecode]) -> Vec<Bytecode> {
        let mut old_block = block.to_vec();
        while let Some(mut new_block) = self.optimize_single_pass(&old_block) {
            old_block = mem::take(&mut new_block);
        }
        old_block
    }
}

impl<T: FixedWindowOptimizer> FixedWindowProcessor<T> {
    pub fn new(optimizer: T) -> Self {
        Self(optimizer)
    }

    fn optimize_single_pass(&self, block: &[Bytecode]) -> Option<Vec<Bytecode>> {
        let window_size = self.0.fixed_window_size();
        let mut changed = false;
        let mut new_block: Vec<Bytecode> = vec![];
        let mut left = 0;
        while left < block.len() {
            let right = left + window_size;
            if right > block.len() {
                // At the end, not enough instructions to form a fixed window.
                new_block.extend(block[left..].to_vec());
                break;
            }
            let window = &block[left..right];
            if let Some(optimized_window) = self.0.optimize_fixed_window(window) {
                new_block.extend(optimized_window);
                left = right;
                changed = true;
            } else {
                new_block.push(block[left].clone());
                left += 1;
            }
        }
        if changed {
            Some(new_block)
        } else {
            None
        }
    }
}

pub trait FixedWindowOptimizer {
    fn fixed_window_size(&self) -> usize;

    fn optimize_fixed_window(&self, window: &[Bytecode]) -> Option<Vec<Bytecode>>;
}
