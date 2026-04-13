// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Intermediate SSA representation and pre-allocation fusion passes.
//!
//! SSA is intra-block and applies only to value IDs, not to params or locals
//! (which are mutable across blocks). Because the operand stack is empty at
//! block boundaries, no phi nodes are needed — each value ID is defined exactly
//! once within its block and never crosses a block boundary.

use super::instr_utils::{extract_imm_value, is_commutative};
use crate::stackless_exec_ir::{BasicBlock, BinaryOp, Instr};
use move_vm_types::loaded_data::runtime_types::Type;

/// Intermediate SSA representation of a single function, before slot allocation.
pub(crate) struct SSAFunction {
    /// Basic blocks in SSA form.
    pub blocks: Vec<BasicBlock>,
    /// Type of each value ID, indexed directly by the value ID number.
    pub vid_types: Vec<Type>,
    /// Types of all locals (params ++ declared locals).
    pub local_types: Vec<Type>,
}

impl SSAFunction {
    /// Run all pre-allocation instruction fusion passes.
    pub(crate) fn with_fusion_passes(mut self) -> Self {
        // [TODO]: right now, we have each different fusion operation to be a separate pass.
        // This is easier to reason about, but we could make it more efficient by
        // combining the passes.
        for block in &mut self.blocks {
            fuse_pairs(&mut block.instrs, try_fuse_field_access);
            fuse_pairs(&mut block.instrs, try_fuse_immediate_binop);
            // Must run after try_fuse_immediate_binop so that BinaryOpImm is
            // available for the BrCmpImm variant.
            fuse_pairs(&mut block.instrs, try_fuse_compare_branch);
        }
        self
    }
}

/// In-place compaction that fuses consecutive instruction pairs.
///
/// For each position, calls `try_fuse(&instrs[r], &instrs[r+1])`. If it returns
/// `Some(fused)`, the pair is replaced by the single fused instruction. Otherwise
/// the instruction is kept as-is. Uses a write-cursor so no allocation is needed.
fn fuse_pairs(instrs: &mut Vec<Instr>, try_fuse: fn(&Instr, &Instr) -> Option<Instr>) {
    let mut write = 0;
    let mut read = 0;
    while read < instrs.len() {
        let fused = instrs
            .get(read + 1)
            .and_then(|next| try_fuse(&instrs[read], next));

        match fused {
            Some(fused_instr) => {
                instrs[write] = fused_instr;
                read += 2;
            },
            None => {
                if write != read {
                    instrs.swap(write, read);
                }
                read += 1;
            },
        }
        write += 1;
    }
    instrs.truncate(write);
}

/// Try to fuse a borrow+deref pair into a combined field access instruction.
fn try_fuse_field_access(first: &Instr, second: &Instr) -> Option<Instr> {
    match (first, second) {
        (Instr::ImmBorrowField(ref_r, fld, src), Instr::ReadRef(dst, read_src))
            if *ref_r == *read_src =>
        {
            Some(Instr::ReadField(*dst, *fld, *src))
        },
        (Instr::ImmBorrowFieldGeneric(ref_r, fld, src), Instr::ReadRef(dst, read_src))
            if *ref_r == *read_src =>
        {
            Some(Instr::ReadFieldGeneric(*dst, *fld, *src))
        },
        (Instr::MutBorrowField(ref_r, fld, dst_ref), Instr::WriteRef(write_ref, val))
            if *ref_r == *write_ref =>
        {
            Some(Instr::WriteField(*fld, *dst_ref, *val))
        },
        (Instr::MutBorrowFieldGeneric(ref_r, fld, dst_ref), Instr::WriteRef(write_ref, val))
            if *ref_r == *write_ref =>
        {
            Some(Instr::WriteFieldGeneric(*fld, *dst_ref, *val))
        },
        (Instr::ImmBorrowVariantField(ref_r, fld, src), Instr::ReadRef(dst, read_src))
            if *ref_r == *read_src =>
        {
            Some(Instr::ReadVariantField(*dst, *fld, *src))
        },
        (Instr::ImmBorrowVariantFieldGeneric(ref_r, fld, src), Instr::ReadRef(dst, read_src))
            if *ref_r == *read_src =>
        {
            Some(Instr::ReadVariantFieldGeneric(*dst, *fld, *src))
        },
        (Instr::MutBorrowVariantField(ref_r, fld, dst_ref), Instr::WriteRef(write_ref, val))
            if *ref_r == *write_ref =>
        {
            Some(Instr::WriteVariantField(*fld, *dst_ref, *val))
        },
        (
            Instr::MutBorrowVariantFieldGeneric(ref_r, fld, dst_ref),
            Instr::WriteRef(write_ref, val),
        ) if *ref_r == *write_ref => Some(Instr::WriteVariantFieldGeneric(*fld, *dst_ref, *val)),
        _ => None,
    }
}

/// Try to fuse a comparison + conditional branch pair into a single `BrCmp`/`BrCmpImm`.
///
/// Handles both `BrTrue` (keeps the comparison operator) and `BrFalse` (negates it).
fn try_fuse_compare_branch(first: &Instr, second: &Instr) -> Option<Instr> {
    match (first, second) {
        // BinaryOp(dst, Cmp(cmp), lhs, rhs) + BrTrue(label, dst)
        (Instr::BinaryOp(dst, BinaryOp::Cmp(cmp), lhs, rhs), Instr::BrTrue(label, cond))
            if *dst == *cond =>
        {
            Some(Instr::BrCmp(*label, *cmp, *lhs, *rhs))
        },
        // BinaryOp(dst, Cmp(cmp), lhs, rhs) + BrFalse(label, dst)
        (Instr::BinaryOp(dst, BinaryOp::Cmp(cmp), lhs, rhs), Instr::BrFalse(label, cond))
            if *dst == *cond =>
        {
            Some(Instr::BrCmp(*label, cmp.negate(), *lhs, *rhs))
        },
        // BinaryOpImm(dst, Cmp(cmp), src, imm) + BrTrue(label, dst)
        (Instr::BinaryOpImm(dst, BinaryOp::Cmp(cmp), src, imm), Instr::BrTrue(label, cond))
            if *dst == *cond =>
        {
            Some(Instr::BrCmpImm(*label, *cmp, *src, *imm))
        },
        // BinaryOpImm(dst, Cmp(cmp), src, imm) + BrFalse(label, dst)
        (Instr::BinaryOpImm(dst, BinaryOp::Cmp(cmp), src, imm), Instr::BrFalse(label, cond))
            if *dst == *cond =>
        {
            Some(Instr::BrCmpImm(*label, cmp.negate(), *src, *imm))
        },
        _ => None,
    }
}

/// Try to fuse a `Ld*` + `BinaryOp` pair into a `BinaryOpImm` instruction.
fn try_fuse_immediate_binop(first: &Instr, second: &Instr) -> Option<Instr> {
    let (tmp, imm) = extract_imm_value(first)?;
    match second {
        Instr::BinaryOp(dst, op, lhs, rhs) if *rhs == tmp => {
            Some(Instr::BinaryOpImm(*dst, *op, *lhs, imm))
        },
        Instr::BinaryOp(dst, op, lhs, rhs) if *lhs == tmp && is_commutative(op) => {
            Some(Instr::BinaryOpImm(*dst, *op, *rhs, imm))
        },
        _ => None,
    }
}
