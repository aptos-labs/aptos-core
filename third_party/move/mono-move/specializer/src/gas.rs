// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Gas cost schedule for the stackless execution IR.
//!
//! A cost is either fixed (constant, e.g. arithmetic and branches) or
//! size-dependent (scaling with a type's byte size, e.g. moves and reference
//! reads/writes). Size-dependent costs still resolve to a constant here, since
//! types are concrete by the time costs are computed, so every instruction
//! ends up with a single constant cost. The costs over-approximate the work,
//! and the numbers are placeholders.
//!
//! A cost may also have a runtime-dependent component, knowable only during
//! execution (e.g. the IO of a global-storage operation). Only the fixed and
//! size-dependent parts are computed here; the runtime charges the rest.
//!
//! TODO: split cost computation from resolution. Emit size-dependent costs as
//! formulas (e.g. `a + b * size(T)`) and resolve them later, rather than
//! resolving each as it is computed.

use crate::{
    lower::context::{concrete_type_size, ref_pointee_size},
    stackless_exec_ir::{Instr, Slot},
};
use anyhow::{bail, Result};
use mono_move_core::types::{strip_ref, InternedType};
use move_binary_format::file_format::FieldHandleIndex;

// --- Loads ---
const LD: u64 = 2;

// --- Data movement ---
fn move_bytes(num_bytes: u32) -> u64 {
    2 + 3 * num_bytes as u64
}

// --- Operators ---
/// Any unary or binary operator: arithmetic, bitwise, shift, negate,
/// comparison, or boolean.
const OP: u64 = 5;

// --- Structs / variants ---
/// Generic pack/unpack and variant pack/unpack.
const PACK_UNPACK: u64 = 8;
const TEST_VARIANT: u64 = 4;

// --- References ---
const BORROW: u64 = 2;
fn read_write_ref(num_bytes: u32) -> u64 {
    2 + 3 * num_bytes as u64
}
/// A field read whose size isn't known here (generic and variant forms).
const FIELD_READ: u64 = 5;

// --- Globals ---
const GLOBAL: u64 = 15;

// --- Calls ---
/// Dispatch only; argument and return moves are charged separately.
const CALL: u64 = 10;
const PACK_CLOSURE: u64 = 9;

// --- Vector ---
const VEC_NEW: u64 = 10;
const VEC_LEN: u64 = 2;
const VEC_BORROW: u64 = 3;
fn vec_elem(elem_size: u32) -> u64 {
    4 + 3 * elem_size as u64
}

// --- Control flow ---
const RETURN: u64 = 2;
const ABORT: u64 = 2;
const ABORT_MSG: u64 = 5;
const JUMP: u64 = 2;
const COND_JUMP: u64 = 3;
const FORCE_GC: u64 = 100;

/// Operand size and type queries used by [`instr_cost`]. A slot's size comes
/// from the resolved layout, not its type, which may be polymorphic.
pub(crate) trait CostContext {
    /// Monomorphised byte size of the type bound to `slot`.
    fn slot_size(&self, slot: Slot) -> Result<u32>;

    /// Interned type bound to `slot` (may be polymorphic).
    fn slot_ty(&self, slot: Slot) -> Result<InternedType>;

    /// Byte size of field `fh` of struct type `struct_ty`.
    fn field_size(&self, struct_ty: InternedType, fh: FieldHandleIndex) -> Result<u32>;
}

/// Total move cost over a list of source slots.
fn sum_move(cx: &impl CostContext, slots: &[Slot]) -> Result<u64> {
    let mut total = 0;
    for slot in slots {
        total += move_bytes(cx.slot_size(*slot)?);
    }
    Ok(total)
}

/// Gas cost of `instr` at the IR level.
pub(crate) fn instr_cost(instr: &Instr, cx: &impl CostContext) -> Result<u64> {
    Ok(match instr {
        // --- Loads ---
        Instr::LdConst(..)
        | Instr::LdTrue(..)
        | Instr::LdFalse(..)
        | Instr::LdU8(..)
        | Instr::LdU16(..)
        | Instr::LdU32(..)
        | Instr::LdU64(..)
        | Instr::LdU128(..)
        | Instr::LdU256(..)
        | Instr::LdI8(..)
        | Instr::LdI16(..)
        | Instr::LdI32(..)
        | Instr::LdI64(..)
        | Instr::LdI128(..)
        | Instr::LdI256(..) => LD,

        // --- Slot ops ---
        Instr::Copy(_, src) | Instr::Move(_, src) => move_bytes(cx.slot_size(*src)?),

        // --- Unary / Binary ---
        Instr::UnaryOp(..) | Instr::BinaryOp(..) | Instr::BinaryOpImm(..) => OP,

        // --- Structs ---
        Instr::Pack(_, struct_ty, _) | Instr::Unpack(_, struct_ty, _) => {
            move_bytes(concrete_type_size(*struct_ty, "struct type")?)
        },
        Instr::PackGeneric(..) | Instr::UnpackGeneric(..) => PACK_UNPACK,

        // --- Enums ---
        Instr::PackVariant(..)
        | Instr::PackVariantGeneric(..)
        | Instr::UnpackVariant(..)
        | Instr::UnpackVariantGeneric(..) => PACK_UNPACK,
        Instr::TestVariant(..) | Instr::TestVariantGeneric(..) => TEST_VARIANT,

        // --- References ---
        Instr::ImmBorrowLoc(..)
        | Instr::MutBorrowLoc(..)
        | Instr::ImmBorrowField(..)
        | Instr::MutBorrowField(..)
        | Instr::ImmBorrowFieldGeneric(..)
        | Instr::MutBorrowFieldGeneric(..)
        | Instr::ImmBorrowVariantField(..)
        | Instr::MutBorrowVariantField(..)
        | Instr::ImmBorrowVariantFieldGeneric(..)
        | Instr::MutBorrowVariantFieldGeneric(..) => BORROW,
        Instr::ReadRef(_, ref_src) => read_write_ref(ref_pointee_size(cx.slot_ty(*ref_src)?)?),
        Instr::WriteRef(ref_dst, _) => read_write_ref(ref_pointee_size(cx.slot_ty(*ref_dst)?)?),

        // --- Fused field access (borrow + read/write) ---
        Instr::ReadField(_, fh, src) => {
            read_write_ref(cx.field_size(strip_ref(cx.slot_ty(*src)?)?, *fh)?)
        },
        Instr::ReadFieldGeneric(..) => FIELD_READ,
        Instr::WriteField(_, _, val) | Instr::WriteFieldGeneric(_, _, val) => {
            read_write_ref(cx.slot_size(*val)?)
        },
        Instr::ReadVariantField(..) | Instr::ReadVariantFieldGeneric(..) => FIELD_READ,
        Instr::WriteVariantField(_, _, val) | Instr::WriteVariantFieldGeneric(_, _, val) => {
            read_write_ref(cx.slot_size(*val)?)
        },

        // --- Fused inline-struct field access ---
        Instr::ImmBorrowLocField(..) | Instr::MutBorrowLocField(..) => BORROW,
        Instr::ReadLocalField(_, fh, local) => move_bytes(cx.field_size(cx.slot_ty(*local)?, *fh)?),
        Instr::WriteLocalField(_, _, val) => move_bytes(cx.slot_size(*val)?),

        // --- Globals ---
        Instr::Exists(..)
        | Instr::ExistsGeneric(..)
        | Instr::MoveFrom(..)
        | Instr::MoveFromGeneric(..)
        | Instr::MoveTo(..)
        | Instr::MoveToGeneric(..)
        | Instr::ImmBorrowGlobal(..)
        | Instr::ImmBorrowGlobalGeneric(..)
        | Instr::MutBorrowGlobal(..)
        | Instr::MutBorrowGlobalGeneric(..) => GLOBAL,

        // --- Calls ---
        Instr::Call(rets, _, args) | Instr::CallGeneric(rets, _, args) => {
            call_cost(cx, args, rets)?
        },

        // --- Closures ---
        Instr::PackClosure(_, _, _, args) | Instr::PackClosureGeneric(_, _, _, args) => {
            PACK_CLOSURE + sum_move(cx, args)?
        },
        Instr::CallClosure(rets, _, args) => call_cost(cx, args, rets)?,

        // --- Vector ---
        Instr::VecPack(_, _, _, elems) => VEC_NEW + sum_move(cx, elems)?,
        Instr::VecLen(..) => VEC_LEN,
        Instr::VecImmBorrow(..) | Instr::VecMutBorrow(..) => VEC_BORROW,
        Instr::VecPushBack(elem_ty, _, _) | Instr::VecPopBack(_, elem_ty, _) => {
            vec_elem(concrete_type_size(*elem_ty, "vector elem type")?)
        },
        Instr::VecUnpack(..) => VEC_NEW,
        Instr::VecSwap(..) => VEC_BORROW,

        // --- Control flow ---
        Instr::Branch(..) => JUMP,
        Instr::BrTrue(..) | Instr::BrFalse(..) | Instr::BrCmp(..) | Instr::BrCmpImm(..) => {
            COND_JUMP
        },
        // 2x per slot upper-bounds the cycle-breaking scratch moves
        // `emit_parallel_copy` adds for a cyclic (e.g. swap-style) return.
        Instr::Ret(slots) => RETURN + 2 * sum_move(cx, slots)?,
        Instr::Abort(..) => ABORT,
        Instr::AbortMsg(..) => ABORT_MSG,

        Instr::ForceGC => FORCE_GC,
    })
}

/// Cost of a call: the dispatch, a move per argument, and a move per
/// Home-slot return.
fn call_cost(cx: &impl CostContext, args: &[Slot], rets: &[Slot]) -> Result<u64> {
    let mut cost = CALL + sum_move(cx, args)?;
    for ret in rets {
        match *ret {
            Slot::Xfer(_) => {
                // Placed without a copy.
            },
            Slot::Home(_) => cost += move_bytes(cx.slot_size(*ret)?),
            Slot::Vid(_) => bail!("Vid slot in post-allocation IR"),
        }
    }
    Ok(cost)
}
