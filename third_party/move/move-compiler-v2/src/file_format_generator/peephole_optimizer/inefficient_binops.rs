// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::file_format_generator::peephole_optimizer::optimizers::FixedWindowOptimizer;
use move_binary_format::file_format::Bytecode;

pub struct TransformInefficientBinops;

impl FixedWindowOptimizer for TransformInefficientBinops {
    fn fixed_window_size(&self) -> usize {
        5
    }

    fn optimize_fixed_window(&self, window: &[Bytecode]) -> Option<Vec<Bytecode>> {
        use Bytecode::*;
        match (&window[0], &window[1], &window[2], &window[3], &window[4]) {
            (
                LdU8(_) | LdU16(_) | LdU32(_) | LdU64(_) | LdU128(_) | LdU256(_) | LdConst(_)
                | LdTrue | LdFalse,
                StLoc(u),
                CopyLoc(_) | MoveLoc(_),
                MoveLoc(v),
                Add | Sub | Mul | Mod | Div | BitOr | BitAnd | Xor | Or | And | Eq | Neq | Lt | Gt
                | Le | Ge | Shl | Shr,
            ) if *u == *v => Some(vec![
                window[2].clone(),
                window[0].clone(),
                window[4].clone(),
            ]),
            _ => None,
        }
    }
}
