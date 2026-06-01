// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! [`MicroOp`]-specific implementations of the gas-metering traits defined in
//! [`mono_move_gas`].
//!
//! This is the only place that knows about both the instruction set and the gas
//! framework. Plug in a different ISA by writing an equivalent file.

use super::{CodeOffset, MicroOp};
use mono_move_gas::{GasMeteredInstruction, GasSchedule, HasCfgInfo, InstrCost, RemapTargets};

// ---------------------------------------------------------------------------
// GasMeteredInstruction
// ---------------------------------------------------------------------------

impl GasMeteredInstruction for MicroOp {
    fn charge(cost: u64) -> Self {
        MicroOp::Charge { cost }
    }
}

// ---------------------------------------------------------------------------
// HasCfgInfo
// ---------------------------------------------------------------------------

impl HasCfgInfo for MicroOp {
    fn branch_target(&self) -> Option<usize> {
        match self {
            MicroOp::Jump { target }
            | MicroOp::JumpNotZeroU64 { target, .. }
            | MicroOp::JumpNotZeroByte { target, .. }
            | MicroOp::JumpZeroByte { target, .. }
            | MicroOp::JumpGreaterEqualU64Imm { target, .. }
            | MicroOp::JumpLessU64Imm { target, .. }
            | MicroOp::JumpGreaterU64Imm { target, .. }
            | MicroOp::JumpLessEqualU64Imm { target, .. }
            | MicroOp::JumpLessU64 { target, .. }
            | MicroOp::JumpGreaterEqualU64 { target, .. }
            | MicroOp::JumpNotEqualU64 { target, .. } => Some(target.0 as usize),
            MicroOp::JumpIntCmp(op) => Some(op.target.0 as usize),
            MicroOp::StoreImm8 { .. }
            | MicroOp::StoreImm16 { .. }
            | MicroOp::StoreImm32 { .. }
            | MicroOp::StoreImm1 { .. }
            | MicroOp::Move8 { .. }
            | MicroOp::Move { .. }
            | MicroOp::AddU64 { .. }
            | MicroOp::AddU64Imm { .. }
            | MicroOp::SubU64 { .. }
            | MicroOp::SubU64Imm { .. }
            | MicroOp::RSubU64Imm { .. }
            | MicroOp::MulU64 { .. }
            | MicroOp::MulU64Imm { .. }
            | MicroOp::DivU64 { .. }
            | MicroOp::DivU64Imm { .. }
            | MicroOp::ModU64 { .. }
            | MicroOp::ModU64Imm { .. }
            | MicroOp::BitAndU64 { .. }
            | MicroOp::BitOrU64 { .. }
            | MicroOp::BitXorU64 { .. }
            | MicroOp::ShlU64 { .. }
            | MicroOp::ShlU64Imm { .. }
            | MicroOp::ShrU64 { .. }
            | MicroOp::ShrU64Imm { .. }
            | MicroOp::IntAdd(_)
            | MicroOp::IntSub(_)
            | MicroOp::IntMul(_)
            | MicroOp::IntDiv(_)
            | MicroOp::IntMod(_)
            | MicroOp::IntBitAnd(_)
            | MicroOp::IntBitOr(_)
            | MicroOp::IntBitXor(_)
            | MicroOp::IntShl(_)
            | MicroOp::IntShr(_)
            | MicroOp::IntNegate(_)
            | MicroOp::IntCast(_)
            | MicroOp::Return
            | MicroOp::Abort { .. }
            | MicroOp::AbortMsg { .. }
            | MicroOp::CallIndirect { .. }
            | MicroOp::CallDirect { .. }
            | MicroOp::CallNative { .. }
            | MicroOp::VecNew { .. }
            | MicroOp::VecLen { .. }
            | MicroOp::VecPushBack { .. }
            | MicroOp::VecPopBack { .. }
            | MicroOp::VecLoadElem { .. }
            | MicroOp::VecStoreElem { .. }
            | MicroOp::SlotBorrow { .. }
            | MicroOp::VecBorrow { .. }
            | MicroOp::HeapBorrow { .. }
            | MicroOp::ReadRef { .. }
            | MicroOp::WriteRef { .. }
            | MicroOp::DeriveRefOffsetImm { .. }
            | MicroOp::ReadRefOffset { .. }
            | MicroOp::WriteRefOffset { .. }
            | MicroOp::HeapNew { .. }
            | MicroOp::HeapMoveFrom8 { .. }
            | MicroOp::HeapMoveFrom { .. }
            | MicroOp::HeapMoveTo8 { .. }
            | MicroOp::HeapMoveToImm8 { .. }
            | MicroOp::HeapMoveTo { .. }
            | MicroOp::Charge { .. }
            | MicroOp::StoreRandomU64 { .. }
            | MicroOp::ForceGC
            | MicroOp::PackClosure(_)
            | MicroOp::CallClosure(_)
            | MicroOp::Exists { .. }
            | MicroOp::BorrowGlobal { .. }
            | MicroOp::BorrowGlobalMut { .. }
            | MicroOp::MoveFrom { .. }
            | MicroOp::MoveTo { .. }
            | MicroOp::IntCmp(_)
            | MicroOp::BoolNot { .. }
            | MicroOp::BoolAnd { .. }
            | MicroOp::BoolOr { .. } => None,
        }
    }
}

// ---------------------------------------------------------------------------
// RemapTargets
// ---------------------------------------------------------------------------

impl RemapTargets for MicroOp {
    fn remap_targets(self, remap: impl Fn(usize) -> usize) -> Self {
        let co = |c: CodeOffset| CodeOffset(remap(c.0 as usize) as u32);
        match self {
            MicroOp::Jump { target } => MicroOp::Jump { target: co(target) },
            MicroOp::JumpNotZeroU64 { target, src } => MicroOp::JumpNotZeroU64 {
                target: co(target),
                src,
            },
            MicroOp::JumpNotZeroByte { target, src } => MicroOp::JumpNotZeroByte {
                target: co(target),
                src,
            },
            MicroOp::JumpZeroByte { target, src } => MicroOp::JumpZeroByte {
                target: co(target),
                src,
            },
            MicroOp::JumpIntCmp(mut op) => {
                op.target = co(op.target);
                MicroOp::JumpIntCmp(op)
            },
            MicroOp::JumpGreaterEqualU64Imm { target, src, imm } => {
                MicroOp::JumpGreaterEqualU64Imm {
                    target: co(target),
                    src,
                    imm,
                }
            },
            MicroOp::JumpLessU64 { target, lhs, rhs } => MicroOp::JumpLessU64 {
                target: co(target),
                lhs,
                rhs,
            },
            MicroOp::JumpLessU64Imm { target, src, imm } => MicroOp::JumpLessU64Imm {
                target: co(target),
                src,
                imm,
            },
            MicroOp::JumpGreaterU64Imm { target, src, imm } => MicroOp::JumpGreaterU64Imm {
                target: co(target),
                src,
                imm,
            },
            MicroOp::JumpLessEqualU64Imm { target, src, imm } => MicroOp::JumpLessEqualU64Imm {
                target: co(target),
                src,
                imm,
            },
            MicroOp::JumpGreaterEqualU64 { target, lhs, rhs } => MicroOp::JumpGreaterEqualU64 {
                target: co(target),
                lhs,
                rhs,
            },
            MicroOp::JumpNotEqualU64 { target, lhs, rhs } => MicroOp::JumpNotEqualU64 {
                target: co(target),
                lhs,
                rhs,
            },
            op @ (MicroOp::StoreImm8 { .. }
            | MicroOp::StoreImm16 { .. }
            | MicroOp::StoreImm32 { .. }
            | MicroOp::StoreImm1 { .. }
            | MicroOp::Move8 { .. }
            | MicroOp::Move { .. }
            | MicroOp::AddU64 { .. }
            | MicroOp::AddU64Imm { .. }
            | MicroOp::SubU64 { .. }
            | MicroOp::SubU64Imm { .. }
            | MicroOp::RSubU64Imm { .. }
            | MicroOp::MulU64 { .. }
            | MicroOp::MulU64Imm { .. }
            | MicroOp::DivU64 { .. }
            | MicroOp::DivU64Imm { .. }
            | MicroOp::ModU64 { .. }
            | MicroOp::ModU64Imm { .. }
            | MicroOp::BitAndU64 { .. }
            | MicroOp::BitOrU64 { .. }
            | MicroOp::BitXorU64 { .. }
            | MicroOp::ShlU64 { .. }
            | MicroOp::ShlU64Imm { .. }
            | MicroOp::ShrU64 { .. }
            | MicroOp::ShrU64Imm { .. }
            | MicroOp::IntAdd(_)
            | MicroOp::IntSub(_)
            | MicroOp::IntMul(_)
            | MicroOp::IntDiv(_)
            | MicroOp::IntMod(_)
            | MicroOp::IntBitAnd(_)
            | MicroOp::IntBitOr(_)
            | MicroOp::IntBitXor(_)
            | MicroOp::IntShl(_)
            | MicroOp::IntShr(_)
            | MicroOp::IntNegate(_)
            | MicroOp::IntCast(_)
            | MicroOp::Return
            | MicroOp::Abort { .. }
            | MicroOp::AbortMsg { .. }
            | MicroOp::CallIndirect { .. }
            | MicroOp::CallDirect { .. }
            | MicroOp::CallNative { .. }
            | MicroOp::VecNew { .. }
            | MicroOp::VecLen { .. }
            | MicroOp::VecPushBack { .. }
            | MicroOp::VecPopBack { .. }
            | MicroOp::VecLoadElem { .. }
            | MicroOp::VecStoreElem { .. }
            | MicroOp::SlotBorrow { .. }
            | MicroOp::VecBorrow { .. }
            | MicroOp::HeapBorrow { .. }
            | MicroOp::ReadRef { .. }
            | MicroOp::WriteRef { .. }
            | MicroOp::DeriveRefOffsetImm { .. }
            | MicroOp::ReadRefOffset { .. }
            | MicroOp::WriteRefOffset { .. }
            | MicroOp::HeapNew { .. }
            | MicroOp::HeapMoveFrom8 { .. }
            | MicroOp::HeapMoveFrom { .. }
            | MicroOp::HeapMoveTo8 { .. }
            | MicroOp::HeapMoveToImm8 { .. }
            | MicroOp::HeapMoveTo { .. }
            | MicroOp::Charge { .. }
            | MicroOp::StoreRandomU64 { .. }
            | MicroOp::ForceGC
            | MicroOp::PackClosure(_)
            | MicroOp::CallClosure(_)
            | MicroOp::Exists { .. }
            | MicroOp::BorrowGlobal { .. }
            | MicroOp::BorrowGlobalMut { .. }
            | MicroOp::MoveFrom { .. }
            | MicroOp::MoveTo { .. }
            | MicroOp::IntCmp(_)
            | MicroOp::BoolNot { .. }
            | MicroOp::BoolAnd { .. }
            | MicroOp::BoolOr { .. }) => op,
        }
    }
}

// ---------------------------------------------------------------------------
// GasSchedule
// ---------------------------------------------------------------------------

/// Default gas schedule for [`MicroOp`].
///
/// All costs are dummy placeholder values for now.
pub struct MicroOpGasSchedule;

impl GasSchedule<MicroOp> for MicroOpGasSchedule {
    fn cost(&self, instr: &MicroOp) -> InstrCost<MicroOp> {
        InstrCost::constant(match instr {
            // --- Data movement ---
            MicroOp::StoreImm8 { .. } => 2,
            MicroOp::StoreImm16 { .. } => 3,
            MicroOp::StoreImm32 { .. } => 4,
            MicroOp::StoreImm1 { .. } => 2,
            MicroOp::Move8 { .. } => 2,
            MicroOp::Move { size, .. } => 2 + 3 * *size as u64,

            // --- Arithmetic ---
            MicroOp::AddU64 { .. }
            | MicroOp::AddU64Imm { .. }
            | MicroOp::SubU64 { .. }
            | MicroOp::SubU64Imm { .. }
            | MicroOp::RSubU64Imm { .. }
            | MicroOp::BitAndU64 { .. }
            | MicroOp::BitOrU64 { .. }
            | MicroOp::BitXorU64 { .. }
            | MicroOp::ShlU64 { .. }
            | MicroOp::ShlU64Imm { .. }
            | MicroOp::ShrU64 { .. }
            | MicroOp::ShrU64Imm { .. } => 3,
            MicroOp::MulU64 { .. } | MicroOp::MulU64Imm { .. } => 4,
            MicroOp::DivU64 { .. }
            | MicroOp::DivU64Imm { .. }
            | MicroOp::ModU64 { .. }
            | MicroOp::ModU64Imm { .. } => 5,

            // --- Unspecialized integer ops ---
            // Placeholder constant. Revisit once we have profiling data on
            // the per-width / per-kind cost of the non-inlined dispatch.
            MicroOp::IntAdd(_)
            | MicroOp::IntSub(_)
            | MicroOp::IntMul(_)
            | MicroOp::IntDiv(_)
            | MicroOp::IntMod(_)
            | MicroOp::IntBitAnd(_)
            | MicroOp::IntBitOr(_)
            | MicroOp::IntBitXor(_)
            | MicroOp::IntShl(_)
            | MicroOp::IntShr(_)
            | MicroOp::IntNegate(_)
            | MicroOp::IntCast(_) => 5,

            // --- Comparison & boolean logic ---
            MicroOp::IntCmp(_) => 3,
            MicroOp::BoolNot { .. } | MicroOp::BoolAnd { .. } | MicroOp::BoolOr { .. } => 2,

            // --- Control flow ---
            MicroOp::CallIndirect { .. } | MicroOp::CallDirect { .. } => 10,
            MicroOp::CallNative { .. } => 10,
            MicroOp::Return => 2,
            MicroOp::Abort { .. } => 2,
            MicroOp::AbortMsg { .. } => 5,
            MicroOp::Jump { .. } => 2,
            MicroOp::JumpNotZeroU64 { .. }
            | MicroOp::JumpNotZeroByte { .. }
            | MicroOp::JumpZeroByte { .. }
            | MicroOp::JumpIntCmp(_)
            | MicroOp::JumpGreaterEqualU64Imm { .. }
            | MicroOp::JumpLessU64Imm { .. }
            | MicroOp::JumpGreaterU64Imm { .. }
            | MicroOp::JumpLessEqualU64Imm { .. }
            | MicroOp::JumpLessU64 { .. }
            | MicroOp::JumpGreaterEqualU64 { .. }
            | MicroOp::JumpNotEqualU64 { .. } => 3,

            // --- Vector operations ---
            MicroOp::VecNew { .. } => 10,
            MicroOp::VecLen { .. } => 2,
            MicroOp::VecPushBack { elem_size, .. }
            | MicroOp::VecPopBack { elem_size, .. }
            | MicroOp::VecLoadElem { elem_size, .. }
            | MicroOp::VecStoreElem { elem_size, .. } => 4 + 3 * *elem_size as u64,

            // --- Reference operations ---
            MicroOp::SlotBorrow { .. } => 2,
            MicroOp::VecBorrow { .. } => 3,
            MicroOp::HeapBorrow { .. } => 2,
            MicroOp::ReadRef { size, .. } | MicroOp::WriteRef { size, .. } => 2 + 3 * *size as u64,
            MicroOp::DeriveRefOffsetImm { .. } => 2,
            MicroOp::ReadRefOffset { size, .. } | MicroOp::WriteRefOffset { size, .. } => {
                2 + 3 * *size as u64
            },

            // --- Heap object operations ---
            MicroOp::HeapNew { .. } => 8,
            MicroOp::HeapMoveFrom8 { .. }
            | MicroOp::HeapMoveTo8 { .. }
            | MicroOp::HeapMoveToImm8 { .. } => 2,
            MicroOp::HeapMoveFrom { size, .. } | MicroOp::HeapMoveTo { size, .. } => {
                2 + 3 * *size as u64
            },

            // --- Gas metering (inserted by instrumentation; not in input) ---
            MicroOp::Charge { .. } => 0,

            // --- Debug ---
            MicroOp::StoreRandomU64 { .. } => 1,
            MicroOp::ForceGC => 100,

            // --- Closures ---
            MicroOp::PackClosure(_) => 20,
            MicroOp::CallClosure(_) => 15,

            MicroOp::Exists { .. } => 10,
            MicroOp::BorrowGlobal { .. } => 10,
            MicroOp::BorrowGlobalMut { .. } => 20,
            MicroOp::MoveFrom { .. } => 20,
            MicroOp::MoveTo { .. } => 20,
        })
    }
}
