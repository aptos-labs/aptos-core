// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! V2 optimization passes (post-allocation cleanup).
//!
//! - Instruction fusion (reused from optimize.rs)
//! - Identity move elimination
//! - Dead instruction elimination (reused from optimize.rs)
//! - Register renumbering (reused from optimize.rs)

use crate::ir::{FunctionIR, Instr, ModuleIR};
use crate::optimize_v1::{
    dead_instruction_elimination, fuse_field_access, renumber_registers,
};

/// Optimize all functions in a module IR using the v2 pipeline.
pub fn optimize_module_v2(module_ir: &mut ModuleIR) {
    for func in &mut module_ir.functions {
        fuse_field_access(func);
        eliminate_identity_moves(func);
        dead_instruction_elimination(func);
        renumber_registers(func);
    }
}

/// Remove `Move(r, r)` and `Copy(r, r)` instructions (identity moves).
fn eliminate_identity_moves(func: &mut FunctionIR) {
    func.instrs.retain(|instr| {
        !matches!(instr, Instr::Move(d, s) | Instr::Copy(d, s) if d == s)
    });
}
