// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Display for lowered micro-ops in test baselines.

use crate::lowering_context::LoweringContext;
use mono_move_micro_ops::MicroOp;
use std::fmt;

pub struct MicroOpsFunctionDisplay<'a> {
    pub func_name: &'a str,
    pub ctx: &'a LoweringContext,
    pub ops: &'a [MicroOp],
}

impl fmt::Display for MicroOpsFunctionDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "fun {}() {{", self.func_name)?;
        writeln!(f, "  frame_data_size: {}", self.ctx.frame_data_size)?;
        writeln!(f, "  code:")?;
        for (i, op) in self.ops.iter().enumerate() {
            write!(f, "    {}: ", i)?;
            display_micro_op(f, op)?;
            writeln!(f)?;
        }
        writeln!(f, "}}")
    }
}

fn display_micro_op(f: &mut fmt::Formatter<'_>, op: &MicroOp) -> fmt::Result {
    match op {
        MicroOp::StoreImm8 { dst, imm } => {
            write!(f, "StoreImm8 [{}] <- #{}", dst.0, imm)
        },
        MicroOp::Move8 { dst, src } => {
            write!(f, "Move8 [{}] <- [{}]", dst.0, src.0)
        },
        MicroOp::Move { dst, src, size } => {
            write!(f, "Move({}) [{}] <- [{}]", size, dst.0, src.0)
        },
        MicroOp::AddU64 { dst, lhs, rhs } => {
            write!(f, "AddU64 [{}] <- [{}] + [{}]", dst.0, lhs.0, rhs.0)
        },
        MicroOp::AddU64Imm { dst, src, imm } => {
            write!(f, "AddU64Imm [{}] <- [{}] + #{}", dst.0, src.0, imm)
        },
        MicroOp::SubU64Imm { dst, src, imm } => {
            write!(f, "SubU64Imm [{}] <- [{}] - #{}", dst.0, src.0, imm)
        },
        MicroOp::RSubU64Imm { dst, src, imm } => {
            write!(f, "RSubU64Imm [{}] <- #{} - [{}]", dst.0, imm, src.0)
        },
        MicroOp::ShrU64Imm { dst, src, imm } => {
            write!(f, "ShrU64Imm [{}] <- [{}] >> #{}", dst.0, src.0, imm)
        },
        MicroOp::ModU64 { dst, lhs, rhs } => {
            write!(f, "ModU64 [{}] <- [{}] % [{}]", dst.0, lhs.0, rhs.0)
        },
        MicroOp::CallFunc { func_id } => {
            write!(f, "CallFunc #{}", func_id)
        },
        MicroOp::CallLocalFunc { ptr } => {
            write!(f, "CallLocalFunc {:p}", ptr)
        },
        MicroOp::Return => {
            write!(f, "Return")
        },
        MicroOp::Jump { target } => {
            write!(f, "Jump @{}", target.0)
        },
        MicroOp::JumpNotZeroU64 { target, src } => {
            write!(f, "JumpNotZeroU64 @{} [{}]", target.0, src.0)
        },
        MicroOp::JumpGreaterEqualU64Imm { target, src, imm } => {
            write!(
                f,
                "JumpGreaterEqualU64Imm @{} [{}] >= #{}",
                target.0, src.0, imm
            )
        },
        MicroOp::JumpLessU64 { target, lhs, rhs } => {
            write!(f, "JumpLessU64 @{} [{}] < [{}]", target.0, lhs.0, rhs.0)
        },
        // Vector ops
        MicroOp::VecNew { dst } => {
            write!(f, "VecNew [{}]", dst.0)
        },
        MicroOp::VecLen { dst, vec_ref } => {
            write!(f, "VecLen [{}] <- vec_len([{}])", dst.0, vec_ref.0)
        },
        MicroOp::VecPushBack {
            vec_ref,
            elem,
            elem_size,
            descriptor_id,
        } => {
            write!(
                f,
                "VecPushBack [{}].push([{}], size={}, desc={})",
                vec_ref.0, elem.0, elem_size, descriptor_id
            )
        },
        MicroOp::VecPopBack {
            dst,
            vec_ref,
            elem_size,
        } => {
            write!(
                f,
                "VecPopBack [{}] <- [{}].pop(size={})",
                dst.0, vec_ref.0, elem_size
            )
        },
        MicroOp::VecLoadElem {
            dst,
            vec_ref,
            idx,
            elem_size,
        } => {
            write!(
                f,
                "VecLoadElem [{}] <- [{}][[{}]] (size={})",
                dst.0, vec_ref.0, idx.0, elem_size
            )
        },
        MicroOp::VecStoreElem {
            vec_ref,
            idx,
            src,
            elem_size,
        } => {
            write!(
                f,
                "VecStoreElem [{}][[{}]] <- [{}] (size={})",
                vec_ref.0, idx.0, src.0, elem_size
            )
        },
        // Reference ops
        MicroOp::SlotBorrow { dst, local } => {
            write!(f, "SlotBorrow [{}] <- &[{}]", dst.0, local.0)
        },
        MicroOp::VecBorrow {
            dst,
            vec_ref,
            idx,
            elem_size,
        } => {
            write!(
                f,
                "VecBorrow [{}] <- &[{}][[{}]] (elem_size={})",
                dst.0, vec_ref.0, idx.0, elem_size
            )
        },
        MicroOp::HeapBorrow {
            dst,
            obj_ref,
            offset,
        } => {
            write!(f, "HeapBorrow [{}] <- &[{}]+{}", dst.0, obj_ref.0, offset)
        },
        MicroOp::ReadRef { dst, ref_ptr, size } => {
            write!(f, "ReadRef [{}] <- *[{}] (size={})", dst.0, ref_ptr.0, size)
        },
        MicroOp::WriteRef { ref_ptr, src, size } => {
            write!(
                f,
                "WriteRef *[{}] <- [{}] (size={})",
                ref_ptr.0, src.0, size
            )
        },
        // Heap object ops
        MicroOp::HeapNew { dst, descriptor_id } => {
            write!(f, "HeapNew [{}] desc={}", dst.0, descriptor_id)
        },
        MicroOp::HeapMoveFrom8 {
            dst,
            heap_ptr,
            offset,
        } => {
            write!(
                f,
                "HeapMoveFrom8 [{}] <- [{}]+{}",
                dst.0, heap_ptr.0, offset
            )
        },
        MicroOp::HeapMoveFrom {
            dst,
            heap_ptr,
            offset,
            size,
        } => {
            write!(
                f,
                "HeapMoveFrom [{}] <- [{}]+{} (size={})",
                dst.0, heap_ptr.0, offset, size
            )
        },
        MicroOp::HeapMoveTo8 {
            heap_ptr,
            offset,
            src,
        } => {
            write!(f, "HeapMoveTo8 [{}]+{} <- [{}]", heap_ptr.0, offset, src.0)
        },
        MicroOp::HeapMoveToImm8 {
            heap_ptr,
            offset,
            imm,
        } => {
            write!(f, "HeapMoveToImm8 [{}]+{} <- #{}", heap_ptr.0, offset, imm)
        },
        MicroOp::HeapMoveTo {
            heap_ptr,
            offset,
            src,
            size,
        } => {
            write!(
                f,
                "HeapMoveTo [{}]+{} <- [{}] (size={})",
                heap_ptr.0, offset, src.0, size
            )
        },
        // Debug ops
        MicroOp::StoreRandomU64 { dst } => {
            write!(f, "StoreRandomU64 [{}]", dst.0)
        },
        MicroOp::XorU64 { dst, lhs, rhs } => {
            write!(f, "XorU64 [{}] <- [{}] ^ [{}]", dst.0, lhs.0, rhs.0)
        },
        MicroOp::JumpLessU64Imm { target, src, imm } => {
            write!(f, "JumpLessU64Imm @{} [{}] < #{}", target.0, src.0, imm)
        },
        MicroOp::JumpGreaterEqualU64 { target, lhs, rhs } => {
            write!(
                f,
                "JumpGreaterEqualU64 @{} [{}] >= [{}]",
                target.0, lhs.0, rhs.0
            )
        },
        MicroOp::JumpNotEqualU64 { target, lhs, rhs } => {
            write!(
                f,
                "JumpNotEqualU64 @{} [{}] != [{}]",
                target.0, lhs.0, rhs.0
            )
        },
        MicroOp::ForceGC => {
            write!(f, "ForceGC")
        },
        MicroOp::Charge { cost } => {
            write!(f, "Charge #{}", cost)
        },
    }
}
