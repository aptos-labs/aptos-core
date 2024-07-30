// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::file_format_generator::peephole_optimizer::optimizers::FixedWindowOptimizer;
use move_binary_format::file_format::Bytecode;

pub struct RedundantPairs;

impl FixedWindowOptimizer for RedundantPairs {
    fn fixed_window_size(&self) -> usize {
        2
    }

    fn optimize_fixed_window(&self, window: &[Bytecode]) -> Option<Vec<Bytecode>> {
        use Bytecode::*;
        match (&window[0], &window[1]) {
            (StLoc(u), MoveLoc(v)) | (CopyLoc(u), StLoc(v)) | (MoveLoc(u), StLoc(v))
                if *u == *v =>
            {
                Some(vec![])
            },
            (LdTrue, BrTrue(target)) | (LdFalse, BrFalse(target)) => Some(vec![Branch(*target)]),
            (CopyLoc(_), Pop) => Some(vec![]),
            _ => None,
        }
    }
}
