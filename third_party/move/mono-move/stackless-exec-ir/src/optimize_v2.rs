// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! V2 optimization passes (post-allocation cleanup).
//!
//! - Instruction fusion (field access, immediate binops)
//! - Copy propagation
//! - Identity move elimination
//! - Dead instruction elimination
//! - Register renumbering

use crate::ir::{FunctionIR, Instr, ModuleIR};
use crate::optimize_v1::{
    copy_propagation, dead_instruction_elimination, fuse_field_access, fuse_immediate_binops,
    renumber_registers,
};

/// Optimize all functions in a module IR using the v2 pipeline.
pub fn optimize_module_v2(module_ir: &mut ModuleIR) {
    for func in &mut module_ir.functions {
        fuse_field_access(func);
        copy_propagation(func);
        fuse_immediate_binops(func);
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
