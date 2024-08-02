// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module contains setup for basic block peephole optimizers.

use move_binary_format::file_format::Bytecode;

/// A basic block optimizer that optimizes a basic block of bytecode.
pub trait BasicBlockOptimizer {
    /// Given a basic `block`, return its optimized version.
    fn optimize(&self, block: &[Bytecode]) -> Vec<Bytecode>;
}

/// An optimizer for a fixed window of bytecode.
/// The fixed window can be assumed to be within a basic block.
pub trait FixedWindowOptimizer {
    /// The fixed window size for this optimizer.
    fn fixed_window_size(&self) -> usize;

    /// Given a fixed `window` of bytecode of size `self.fixed_window_size()`,
    /// optionally return its optimized version.
    /// If `None` is returned, the `window` is not optimized.
    fn optimize_fixed_window(&self, window: &[Bytecode]) -> Option<Vec<Bytecode>>;
}

/// A processor to perform fixed window optimizations of a particular style on a basic block.
pub struct FixedWindowProcessor<T: FixedWindowOptimizer>(T);

impl<T: FixedWindowOptimizer> BasicBlockOptimizer for FixedWindowProcessor<T> {
    fn optimize(&self, block: &[Bytecode]) -> Vec<Bytecode> {
        let mut old_block = block.to_vec();
        // Run single passes until code stops changing.
        while let Some(new_block) = self.optimize_single_pass(&old_block) {
            old_block = new_block;
        }
        old_block
    }
}

impl<T: FixedWindowOptimizer> FixedWindowProcessor<T> {
    /// Create a new `FixedWindowProcessor` with the given `optimizer`.
    pub fn new(optimizer: T) -> Self {
        Self(optimizer)
    }

    /// Run a single pass of fixed window peephole optimization on the given basic `block`.
    /// If the block cannot be optimized further, return `None`.
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
