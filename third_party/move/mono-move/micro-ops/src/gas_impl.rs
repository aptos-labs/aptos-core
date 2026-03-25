// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! [`MicroOp`]-specific implementations of the gas-metering traits defined in
//! [`mono_move_gas`].
//!
//! This is the only place that knows about both the instruction set and the gas
//! framework. Plug in a different ISA by writing an equivalent file.

use crate::instruction::{CodeOffset, MicroOp};
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
            | MicroOp::JumpGreaterEqualU64Imm { target, .. }
            | MicroOp::JumpLessU64Imm { target, .. }
            | MicroOp::JumpLessU64 { target, .. }
            | MicroOp::JumpGreaterEqualU64 { target, .. }
            | MicroOp::JumpNotEqualU64 { target, .. } => Some(target.0 as usize),
            MicroOp::StoreImm8 { .. }
            | MicroOp::Move8 { .. }
            | MicroOp::Move { .. }
            | MicroOp::AddU64 { .. }
            | MicroOp::AddU64Imm { .. }
            | MicroOp::SubU64Imm { .. }
            | MicroOp::RSubU64Imm { .. }
            | MicroOp::XorU64 { .. }
            | MicroOp::ShrU64Imm { .. }
            | MicroOp::ModU64 { .. }
            | MicroOp::Return
            | MicroOp::CallFunc { .. }
            | MicroOp::CallLocalFunc { .. }
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
            | MicroOp::HeapNew { .. }
            | MicroOp::HeapMoveFrom8 { .. }
            | MicroOp::HeapMoveFrom { .. }
            | MicroOp::HeapMoveTo8 { .. }
            | MicroOp::HeapMoveToImm8 { .. }
            | MicroOp::HeapMoveTo { .. }
            | MicroOp::Charge { .. }
            | MicroOp::StoreRandomU64 { .. }
            | MicroOp::ForceGC => None,
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
            | MicroOp::Move8 { .. }
            | MicroOp::Move { .. }
            | MicroOp::AddU64 { .. }
            | MicroOp::AddU64Imm { .. }
            | MicroOp::SubU64Imm { .. }
            | MicroOp::RSubU64Imm { .. }
            | MicroOp::XorU64 { .. }
            | MicroOp::ShrU64Imm { .. }
            | MicroOp::ModU64 { .. }
            | MicroOp::Return
            | MicroOp::CallFunc { .. }
            | MicroOp::CallLocalFunc { .. }
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
            | MicroOp::HeapNew { .. }
            | MicroOp::HeapMoveFrom8 { .. }
            | MicroOp::HeapMoveFrom { .. }
            | MicroOp::HeapMoveTo8 { .. }
            | MicroOp::HeapMoveToImm8 { .. }
            | MicroOp::HeapMoveTo { .. }
            | MicroOp::Charge { .. }
            | MicroOp::StoreRandomU64 { .. }
            | MicroOp::ForceGC) => op,
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
            MicroOp::Move8 { .. } => 2,
            MicroOp::Move { size, .. } => 2 + 3 * *size as u64,

            // --- Arithmetic ---
            MicroOp::AddU64 { .. }
            | MicroOp::AddU64Imm { .. }
            | MicroOp::SubU64Imm { .. }
            | MicroOp::RSubU64Imm { .. }
            | MicroOp::XorU64 { .. }
            | MicroOp::ShrU64Imm { .. } => 3,
            MicroOp::ModU64 { .. } => 5,

            // --- Control flow ---
            MicroOp::CallFunc { .. } | MicroOp::CallLocalFunc { .. } => 10,
            MicroOp::Return => 2,
            MicroOp::Jump { .. } => 2,
            MicroOp::JumpNotZeroU64 { .. }
            | MicroOp::JumpGreaterEqualU64Imm { .. }
            | MicroOp::JumpLessU64Imm { .. }
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
        })
    }
}
