// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module contains setup for basic block peephole optimizers.

use move_binary_format::file_format::{Bytecode, CodeOffset};

/// A contiguous chunk of bytecode that may have been transformed from some
/// other "original" contiguous chunk of bytecode.
#[derive(PartialEq, Eq)]
pub struct TransformedCodeChunk {
    /// The transformed bytecode.
    pub code: Vec<Bytecode>,
    /// Mapping to the original offsets.
    /// The instruction in `code[i]` corresponds to the instruction at
    /// `original_offsets[i]` in the original bytecode.
    pub original_offsets: Vec<CodeOffset>,
}

impl TransformedCodeChunk {
    /// Create an instance of `TransformedCodeChunk` from the given `code`
    /// and `original_offsets`.
    pub fn new(code: Vec<Bytecode>, original_offsets: Vec<CodeOffset>) -> Self {
        debug_assert_eq!(code.len(), original_offsets.len());
        Self {
            code,
            original_offsets,
        }
    }

    /// Create an empty chunk.
    pub fn empty() -> Self {
        Self::new(vec![], vec![])
    }

    /// Extract a contiguous sub-chunk from this chunk,
    pub fn extract(&self, start: CodeOffset, end: CodeOffset) -> TransformedCodeChunk {
        let new_code = self.code[start as usize..=end as usize].to_vec();
        let new_offsets = self.original_offsets[start as usize..=end as usize].to_vec();
        TransformedCodeChunk::new(new_code, new_offsets)
    }

    /// Extend this chunk with another `other` chunk.
    /// The `original_offsets` for the `other` chunk are incremented by `adjust`.
    pub fn extend(&mut self, other: TransformedCodeChunk, adjust: CodeOffset) {
        self.code.extend(other.code);
        self.original_offsets
            .extend(other.original_offsets.into_iter().map(|off| off + adjust));
    }

    /// Make a new chunk from the given `code`.
    pub fn make_from(code: &[Bytecode]) -> Self {
        Self::new(code.to_vec(), Vec::from_iter(0..code.len() as CodeOffset))
    }

    /// Remap the original offsets using the given `previous_offsets`.
    pub fn remap(self, previous_offsets: Vec<CodeOffset>) -> Self {
        Self::new(
            self.code,
            self.original_offsets
                .into_iter()
                .map(|off| previous_offsets[off as usize])
                .collect(),
        )
    }
}

/// A basic block optimizer that optimizes a basic block of bytecode.
pub trait BasicBlockOptimizer {
    /// Given a basic `block`, return its optimized version [*].
    ///
    /// [*] The optimized version returned via `TransformedCodeChunk` maintains a
    /// mapping to the original offsets in `block`.
    fn optimize(&self, block: &[Bytecode]) -> TransformedCodeChunk;
}

/// An optimizer for a window of bytecode within a basic block.
/// The window is always a suffix of a basic block.
pub trait WindowOptimizer {
    /// Given a `window` of bytecode, return a tuple containing:
    ///   1. an optimized version of a non-empty prefix of the `window`. [*]
    ///   2. size of this prefix (should be non-zero).
    /// If `None` is returned, the `window` is not optimized.
    ///
    /// [*] When an optimized version is returned, the corresponding `TransformedCodeChunk`
    /// maintains a mapping to the original offsets of `window`.
    fn optimize_window(&self, window: &[Bytecode]) -> Option<(TransformedCodeChunk, usize)>;
}

/// A processor to perform window optimizations of a particular style on a basic block.
pub struct WindowProcessor<T: WindowOptimizer>(T);

impl<T: WindowOptimizer> BasicBlockOptimizer for WindowProcessor<T> {
    fn optimize(&self, block: &[Bytecode]) -> TransformedCodeChunk {
        let mut old_block = TransformedCodeChunk::make_from(block);
        // Run single passes until code stops changing.
        while let Some(new_chunk) = self.optimize_single_pass(&old_block.code) {
            old_block = new_chunk.remap(old_block.original_offsets);
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
    fn optimize_single_pass(&self, block: &[Bytecode]) -> Option<TransformedCodeChunk> {
        let mut changed = false;
        let mut new_block = TransformedCodeChunk::empty();
        let mut left = 0;
        while left < block.len() {
            let window = &block[left..];
            if let Some((optimized_window, consumed)) = self.0.optimize_window(window) {
                debug_assert!(consumed != 0);
                new_block.extend(optimized_window, left as CodeOffset);
                left += consumed;
                changed = true;
            } else {
                new_block.extend(
                    TransformedCodeChunk::make_from(&block[left..left + 1]),
                    left as CodeOffset,
                );
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
