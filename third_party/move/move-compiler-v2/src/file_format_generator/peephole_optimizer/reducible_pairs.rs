// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module contains a fixed window peephole optimizer for the Move bytecode.
//! As with all peephole optimizers here, it assumes that the bytecode is valid.
//!
//! We consider fixed windows of size 2 for this optimizer.
//!
//! To reason about the correctness of the optimizations, we need to think about the
//! effect on (1) the stack, (2) the locals, (3) control flow behavior.
//!
//! Below are the implemented optimizations (which all retain the control flow behavior):
//! 1. `StLoc` and `MoveLoc` of the same local `l`: Remove the pair.
//!    - stack is left unaffected (the top remains the same)
//!    - local `l` would not be accessed again (without a future store), because before
//!      the transformation, the value in it has been moved from, leaving it invalid.
//! 2. `CopyLoc` and `StLoc` of the same local `l`: Remove the pair.
//!    - stack is left unaffected
//!    - local `l` has the same valid value as before.
//! 3. `MoveLoc` and `StLoc` of the same local `l`: Remove the pair.
//!    - stack is left unaffected
//!    - local `l` has the same valid value as before.
//! 4. `CopyLoc` followed by `Pop`: Remove the pair.
//!    - stack is left unaffected (value is copied to the top and then removed)
//!    - local is unaffected: it still has a valid value because of copy.
//! 5. [`LdTrue`, `BrTrue`] or [`LdFalse`, `BrFalse`]: Replace with `Branch` to the same
//!    target.
//!    - stack is left unaffected (the first instruction pushes a constant, the second
//!      takes it off).
//! 6. [`LdTrue`, `BrFalse`] or [`LdFalse`, `BrTrue`]: Remove the pair.
//!    - stack is left unaffected.
//!    - locals are unaffected.
//!    - basic blocks are merged.
//! 7. [`Not`, `BrFalse`] or [`Not`, `BrTrue`]: Replace with `BrTrue` or `BrFalse`.
//!    - stack is left unaffected (first instruction negates the top, second takes it
//!      off, vs. just take off the top).
//!    - locals are unaffected.
//!
//! Finally, note that fixed window optimizations are performed on windows within a basic
//! block, not spanning across multiple basic blocks.

use crate::file_format_generator::peephole_optimizer::optimizers::FixedWindowOptimizer;
use move_binary_format::file_format::Bytecode;

pub struct ReduciblePairs;

impl FixedWindowOptimizer for ReduciblePairs {
    fn fixed_window_size(&self) -> usize {
        2
    }

    fn optimize_fixed_window(&self, window: &[Bytecode]) -> Option<Vec<Bytecode>> {
        use Bytecode::*;
        // See module documentation for the reasoning behind these optimizations.
        match (&window[0], &window[1]) {
            (StLoc(u), MoveLoc(v)) | (CopyLoc(u), StLoc(v)) | (MoveLoc(u), StLoc(v))
                if *u == *v =>
            {
                Some(vec![])
            },
            (CopyLoc(_), Pop) => Some(vec![]),
            (LdTrue, BrTrue(target)) | (LdFalse, BrFalse(target)) => Some(vec![Branch(*target)]),
            (LdTrue, BrFalse(_)) | (LdFalse, BrTrue(_)) => Some(vec![]),
            (Not, BrFalse(target)) => Some(vec![BrTrue(*target)]),
            (Not, BrTrue(target)) => Some(vec![BrFalse(*target)]),
            _ => None,
        }
    }
}
