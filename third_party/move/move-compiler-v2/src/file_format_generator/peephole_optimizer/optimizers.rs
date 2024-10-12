// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module contains setup for basic block peephole optimizers.

use move_binary_format::file_format::Bytecode;

/// A basic block optimizer that optimizes a basic block of bytecode.
pub trait BasicBlockOptimizer {
    /// Given a basic `block`, return its optimized version.
    fn optimize(&self, block: &[Bytecode]) -> Vec<Bytecode>;
}

/// An optimizer for a window of bytecode within a basic block.
/// The window is always a suffix of a basic block.
pub trait WindowOptimizer {
    /// Given a `window` of bytecode, return a tuple containing:
    ///   1. an optimized version of a non-empty prefix of the `window`.
    ///   2. size of this prefix (should be non-zero).
    /// If `None` is returned, the `window` is not optimized.
    fn optimize_window(&self, window: &[Bytecode]) -> Option<(Vec<Bytecode>, usize)>;
}

/// A processor to perform window optimizations of a particular style on a basic block.
pub struct WindowProcessor<T: WindowOptimizer>(T);

impl<T: WindowOptimizer> BasicBlockOptimizer for WindowProcessor<T> {
    fn optimize(&self, block: &[Bytecode]) -> Vec<Bytecode> {
        let mut old_block = block.to_vec();
        // Run single passes until code stops changing.
        while let Some(new_block) = self.optimize_single_pass(&old_block) {
            old_block = new_block;
        }
        old_block
    }
}

impl<T: WindowOptimizer> WindowProcessor<T> {
    /// Create a new `WindowProcessor` with the given `optimizer`.
    pub fn new(optimizer: T) -> Self {
        Self(optimizer)
    }

    /// Run a single pass of the window peephole optimization on the given basic `block`.
    /// If the block cannot be optimized further, return `None`.
    fn optimize_single_pass(&self, block: &[Bytecode]) -> Option<Vec<Bytecode>> {
        let mut changed = false;
        let mut new_block: Vec<Bytecode> = vec![];
        let mut left = 0;
        while left < block.len() {
            let window = &block[left..];
            if let Some((optimized_window, consumed)) = self.0.optimize_window(window) {
                debug_assert!(consumed != 0);
                new_block.extend(optimized_window);
                left += consumed;
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
