// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module contains a window peephole optimizer for the Move bytecode.
//! As with all peephole optimizers here, it assumes that the bytecode is valid.
//!
//! This optimizer addresses a commonly appearing pattern when involving loads.
//!
//! The pattern is:
//! 1. Load a constant into the stack.
//! 2. Store the constant into a local `u`.
//! 3. A (possibly empty) sequence of instructions that do not involve `u`,
//!    which we name `sequence`. Currently, the only instructions that can
//!    involve `u` are: `CopyLoc`, `MoveLoc`, `StLoc`, `ImmBorrowLoc`,
//!    and `MutBorrowLoc`.
//! 4. A `MoveLoc` of `u`.
//!
//! This pattern can be replaced with:
//! 1. `sequence`.
//! 2. Load the constant into the stack.
//!
//! This transformation leaves the stack in the same state.
//! The local `u` in the original code has been moved from, so later code
//! cannot use it without a subsequent store.
//! So, skipping the store to `u` is safe.

use crate::file_format_generator::peephole_optimizer::optimizers::{
    TransformedCodeChunk, WindowOptimizer,
};
use move_binary_format::file_format::{Bytecode, CodeOffset};
use std::iter;

/// An optimizer for inefficient loads.
pub struct InefficientLoads;

impl InefficientLoads {
    // We need at least 3 instructions, corresponding to points 1, 2, and 4 in the pattern
    // described in the module documentation (at the top of the file).
    const MIN_WINDOW_SIZE: usize = 3;
}

impl WindowOptimizer for InefficientLoads {
    fn optimize_window(&self, window: &[Bytecode]) -> Option<(TransformedCodeChunk, usize)> {
        use Bytecode::*;
        if window.len() < Self::MIN_WINDOW_SIZE {
            return None;
        }
        // Load and Store a constant into `u`.
        let u = match (&window[0], &window[1]) {
            (
                LdU8(_) | LdU16(_) | LdU32(_) | LdU64(_) | LdU128(_) | LdU256(_) | LdConst(_)
                | LdTrue | LdFalse,
                StLoc(u),
            ) => *u,
            _ => return None,
        };
        for (index, instr) in window[2..].iter().enumerate() {
            match instr {
                CopyLoc(v) | StLoc(v) | ImmBorrowLoc(v) | MutBorrowLoc(v) if u == *v => {
                    // We have encountered an instruction that involves `u`.
                    return None;
                },
                MoveLoc(v) if u == *v => {
                    // We have reached the end of the pattern (point 4 in the module documentation).
                    let sequence = &window[2..index + 2];
                    let load_constant = &window[0..1];
                    let transformed_code = [sequence, load_constant].concat();
                    // original_offsets are 2..index+2 (representing `sequence`),
                    // followed by 0 (representing `load_constant`).
                    let original_offsets = (2..(index + 2) as CodeOffset)
                        .chain(iter::once(0))
                        .collect::<Vec<_>>();
                    return Some((
                        TransformedCodeChunk::new(transformed_code, original_offsets),
                        index + Self::MIN_WINDOW_SIZE,
                    ));
                },
                _ => {
                    // Instruction that does not involve `u`, including `MoveLoc` of a different local.
                },
            }
        }
        // The full pattern was not found.
        None
    }
}
