// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module contains a fixed window peephole optimizer for the Move bytecode.
//! As with all peephole optimizers here, it assumes that the bytecode is valid.
//!
//! We consider fixed windows of size 5 for this optimizer.
//!
//! This optimizer addresses a commonly appearing pattern when compiling binary
//! operations where the second operand to the operation is a constant.
//!
//! The pattern is:
//! 1. Load the constant (second operand) on the stack.
//! 2. Store the constant in a local `l`.
//! 3. Copy or Move some value on the stack (first operand).
//! 4. Move the constant from `l` back to the stack (second operand).
//! 5. Perform the binary operation.
//!
//! We replace it with:
//! 1. Copy or Move the first operand to the stack.
//! 2. Load the constant (second operand) on the stack.
//! 3. Perform the binary operation.
//!
//! This transformation leaves the stack in the same state.
//! The local `l` in the original code has been moved from, so later code
//! cannot use it without a subsequent store. So, not writing back to `l` is safe.

use crate::file_format_generator::peephole_optimizer::optimizers::FixedWindowOptimizer;
use move_binary_format::file_format::Bytecode;

pub struct TransformInefficientBinops;

impl FixedWindowOptimizer for TransformInefficientBinops {
    fn fixed_window_size(&self) -> usize {
        5
    }

    fn optimize_fixed_window(&self, window: &[Bytecode]) -> Option<Vec<Bytecode>> {
        use Bytecode::*;
        // See module documentation for more behind the rationale.
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
