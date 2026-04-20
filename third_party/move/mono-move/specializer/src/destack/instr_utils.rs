// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Instruction utilities.
//!
//! Provides read-only slot visitors (`for_each_def`, `for_each_use`,
//! `for_each_slot`, `collect_defs_and_uses`), in-place slot rewriters
//! (`remap_all_slots_with`, `remap_source_slots_with`), and miscellaneous
//! instruction helpers (`extract_imm_value`, `is_commutative`).
//!
//! # Architecture
//!
//! All slot traversal is built on two const-generic core functions
//! (`visit_slots` for reading, `rewrite_instr_slots` for mutation) so that
//! adding a new `Instr` variant requires updating exactly two match blocks.
//! Public functions are thin wrappers that select const-generic parameters.
//!
//! # Performance
//!
//! The design relies on three compile-time optimizations:
//!
//! - **Const generics** (`DEFS`, `USES`, `SKIP_BORROW_LOC_SRC`): branches
//!   guarded by const booleans are eliminated during monomorphization. Each
//!   wrapper compiles to a specialized match with only the relevant arms.
//! - **Const folding of `SlotRole`**: the `def`/`used`/`defs`/`uses` emit
//!   helpers pass a statically known role tag. After inlining, the compiler
//!   constant-folds role checks in closures that filter or dispatch on role.
//! - **Inlining**: the emit helpers and `rewrite_slot`/`rewrite_slots` are
//!   `#[inline]` to ensure they are folded into the core match body. The
//!   caller-provided closures (`impl FnMut`) are monomorphized and inlined
//!   at each call site.

use crate::stackless_exec_ir::{BinaryOp, ImmValue, Instr, Slot};
use smallvec::SmallVec;

/// Most instructions have at most 4 defs or uses.
pub(crate) type SlotList = SmallVec<[Slot; 4]>;

/// Whether a visited slot is a def (written) or a use (read).
#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotRole {
    Def,
    Use,
}

// =============================================================================
// Read-only slot visitors
// =============================================================================

/// Apply `f` to each slot defined (written) by an instruction.
pub(crate) fn for_each_def(instr: &Instr, mut f: impl FnMut(Slot)) {
    visit_slots::<true, false>(instr, |slot, _| f(slot));
}

/// Apply `f` to each slot used (read) by an instruction.
pub(crate) fn for_each_use(instr: &Instr, mut f: impl FnMut(Slot)) {
    visit_slots::<false, true>(instr, |slot, _| f(slot));
}

/// Apply `f` to every slot (defs and uses) in an instruction.
pub(crate) fn for_each_slot(instr: &Instr, mut f: impl FnMut(Slot)) {
    visit_slots::<true, true>(instr, |slot, _| f(slot));
}

/// Collect defs and uses into separate lists in a single pass.
pub(crate) fn collect_defs_and_uses(instr: &Instr) -> (SlotList, SlotList) {
    let mut defs = SlotList::new();
    let mut uses = SlotList::new();
    visit_slots::<true, true>(instr, |slot, role| match role {
        SlotRole::Def => defs.push(slot),
        SlotRole::Use => uses.push(slot),
    });
    (defs, uses)
}

// =============================================================================
// Slot rewriters
// =============================================================================

/// Rewrite all slot operands of an instruction by applying `f`.
///
/// Each slot is rewritten exactly once — `f` is not applied transitively.
pub(crate) fn remap_all_slots_with(instr: &mut Instr, f: impl FnMut(Slot) -> Slot) {
    rewrite_instr_slots::<true, true, false>(instr, f);
}

/// Rewrite source (use) operands of an instruction by applying `f`,
/// skipping defs and BorrowLoc sources.
///
/// Each slot is rewritten exactly once — `f` is not applied transitively.
pub(crate) fn remap_source_slots_with(instr: &mut Instr, f: impl FnMut(Slot) -> Slot) {
    rewrite_instr_slots::<false, true, true>(instr, f);
}

// =============================================================================
// Other instruction utilities
// =============================================================================

/// Extract the destination slot and immediate value from a load instruction.
pub(crate) fn extract_imm_value(instr: &Instr) -> Option<(Slot, ImmValue)> {
    match instr {
        Instr::LdTrue(dst) => Some((*dst, ImmValue::Bool(true))),
        Instr::LdFalse(dst) => Some((*dst, ImmValue::Bool(false))),
        Instr::LdU8(dst, val) => Some((*dst, ImmValue::U8(*val))),
        Instr::LdU16(dst, val) => Some((*dst, ImmValue::U16(*val))),
        Instr::LdU32(dst, val) => Some((*dst, ImmValue::U32(*val))),
        Instr::LdU64(dst, val) => Some((*dst, ImmValue::U64(*val))),
        Instr::LdI8(dst, val) => Some((*dst, ImmValue::I8(*val))),
        Instr::LdI16(dst, val) => Some((*dst, ImmValue::I16(*val))),
        Instr::LdI32(dst, val) => Some((*dst, ImmValue::I32(*val))),
        Instr::LdI64(dst, val) => Some((*dst, ImmValue::I64(*val))),

        // Too large or non-numeric — not fusible into BinaryOpImm.
        Instr::LdU128(_, _)
        | Instr::LdU256(_, _)
        | Instr::LdI128(_, _)
        | Instr::LdI256(_, _)
        | Instr::LdConst(_, _) => None,

        // Non-load instructions.
        Instr::Copy(_, _)
        | Instr::Move(_, _)
        | Instr::UnaryOp(_, _, _)
        | Instr::BinaryOp(_, _, _, _)
        | Instr::BinaryOpImm(_, _, _, _)
        | Instr::Pack(_, _, _)
        | Instr::PackGeneric(_, _, _)
        | Instr::Unpack(_, _, _)
        | Instr::UnpackGeneric(_, _, _)
        | Instr::PackVariant(_, _, _)
        | Instr::PackVariantGeneric(_, _, _)
        | Instr::UnpackVariant(_, _, _)
        | Instr::UnpackVariantGeneric(_, _, _)
        | Instr::TestVariant(_, _, _)
        | Instr::TestVariantGeneric(_, _, _)
        | Instr::ImmBorrowLoc(_, _)
        | Instr::MutBorrowLoc(_, _)
        | Instr::ImmBorrowField(_, _, _)
        | Instr::MutBorrowField(_, _, _)
        | Instr::ImmBorrowFieldGeneric(_, _, _)
        | Instr::MutBorrowFieldGeneric(_, _, _)
        | Instr::ImmBorrowVariantField(_, _, _)
        | Instr::MutBorrowVariantField(_, _, _)
        | Instr::ImmBorrowVariantFieldGeneric(_, _, _)
        | Instr::MutBorrowVariantFieldGeneric(_, _, _)
        | Instr::ReadRef(_, _)
        | Instr::WriteRef(_, _)
        | Instr::ReadField(_, _, _)
        | Instr::ReadFieldGeneric(_, _, _)
        | Instr::WriteField(_, _, _)
        | Instr::WriteFieldGeneric(_, _, _)
        | Instr::ReadVariantField(_, _, _)
        | Instr::ReadVariantFieldGeneric(_, _, _)
        | Instr::WriteVariantField(_, _, _)
        | Instr::WriteVariantFieldGeneric(_, _, _)
        | Instr::Exists(_, _, _)
        | Instr::ExistsGeneric(_, _, _)
        | Instr::MoveFrom(_, _, _)
        | Instr::MoveFromGeneric(_, _, _)
        | Instr::MoveTo(_, _, _)
        | Instr::MoveToGeneric(_, _, _)
        | Instr::ImmBorrowGlobal(_, _, _)
        | Instr::ImmBorrowGlobalGeneric(_, _, _)
        | Instr::MutBorrowGlobal(_, _, _)
        | Instr::MutBorrowGlobalGeneric(_, _, _)
        | Instr::Call(_, _, _)
        | Instr::CallGeneric(_, _, _)
        | Instr::PackClosure(_, _, _, _)
        | Instr::PackClosureGeneric(_, _, _, _)
        | Instr::CallClosure(_, _, _)
        | Instr::VecPack(_, _, _, _)
        | Instr::VecLen(_, _, _)
        | Instr::VecImmBorrow(_, _, _, _)
        | Instr::VecMutBorrow(_, _, _, _)
        | Instr::VecPushBack(_, _, _)
        | Instr::VecPopBack(_, _, _)
        | Instr::VecUnpack(_, _, _, _)
        | Instr::VecSwap(_, _, _, _)
        | Instr::Branch(_)
        | Instr::BrTrue(_, _)
        | Instr::BrFalse(_, _)
        | Instr::Ret(_)
        | Instr::Abort(_)
        | Instr::AbortMsg(_, _) => None,
    }
}

/// Whether a binary operation is commutative (i.e., operands can be swapped
/// without changing the result).
#[inline]
pub(crate) fn is_commutative(op: &BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Add
            | BinaryOp::Mul
            | BinaryOp::BitOr
            | BinaryOp::BitAnd
            | BinaryOp::Xor
            | BinaryOp::Eq
            | BinaryOp::Neq
            | BinaryOp::Or
            | BinaryOp::And
    )
}

// =============================================================================
// Internal: read-only slot visitor core
// =============================================================================

/// Emit a def slot if `ACTIVE` is true.
#[inline]
fn def<const ACTIVE: bool>(slot: Slot, f: &mut impl FnMut(Slot, SlotRole)) {
    if ACTIVE {
        f(slot, SlotRole::Def);
    }
}

/// Emit a use slot if `ACTIVE` is true.
#[inline]
fn used<const ACTIVE: bool>(slot: Slot, f: &mut impl FnMut(Slot, SlotRole)) {
    if ACTIVE {
        f(slot, SlotRole::Use);
    }
}

/// Emit each slot in a slice as defs if `ACTIVE` is true.
#[inline]
fn defs<const ACTIVE: bool>(slots: &[Slot], f: &mut impl FnMut(Slot, SlotRole)) {
    if ACTIVE {
        slots.iter().for_each(|slot| f(*slot, SlotRole::Def));
    }
}

/// Emit each slot in a slice as uses if `ACTIVE` is true.
#[inline]
fn uses<const ACTIVE: bool>(slots: &[Slot], f: &mut impl FnMut(Slot, SlotRole)) {
    if ACTIVE {
        slots.iter().for_each(|slot| f(*slot, SlotRole::Use));
    }
}

/// Visit slots of an instruction, calling `f(slot, role)` for each.
///
/// `DEFS`/`USES` select which slots to visit. The `def`/`used`/`defs`/`uses`
/// helpers pair the role tag with the const generic by convention.
fn visit_slots<const DEFS: bool, const USES: bool>(
    instr: &Instr,
    mut f: impl FnMut(Slot, SlotRole),
) {
    match instr {
        Instr::LdConst(dst, _)
        | Instr::LdTrue(dst)
        | Instr::LdFalse(dst)
        | Instr::LdU8(dst, _)
        | Instr::LdU16(dst, _)
        | Instr::LdU32(dst, _)
        | Instr::LdU64(dst, _)
        | Instr::LdU128(dst, _)
        | Instr::LdU256(dst, _)
        | Instr::LdI8(dst, _)
        | Instr::LdI16(dst, _)
        | Instr::LdI32(dst, _)
        | Instr::LdI64(dst, _)
        | Instr::LdI128(dst, _)
        | Instr::LdI256(dst, _) => def::<DEFS>(*dst, &mut f),

        Instr::Copy(dst, src) | Instr::Move(dst, src) | Instr::UnaryOp(dst, _, src) => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*src, &mut f);
        },
        Instr::BinaryOp(dst, _, lhs, rhs) => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*lhs, &mut f);
            used::<USES>(*rhs, &mut f);
        },
        Instr::BinaryOpImm(dst, _, lhs, _) => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*lhs, &mut f);
        },

        Instr::Pack(dst, _, fields)
        | Instr::PackGeneric(dst, _, fields)
        | Instr::PackVariant(dst, _, fields)
        | Instr::PackVariantGeneric(dst, _, fields) => {
            def::<DEFS>(*dst, &mut f);
            uses::<USES>(fields, &mut f);
        },
        Instr::Unpack(dsts, _, src)
        | Instr::UnpackGeneric(dsts, _, src)
        | Instr::UnpackVariant(dsts, _, src)
        | Instr::UnpackVariantGeneric(dsts, _, src) => {
            defs::<DEFS>(dsts, &mut f);
            used::<USES>(*src, &mut f);
        },
        Instr::TestVariant(dst, _, src) | Instr::TestVariantGeneric(dst, _, src) => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*src, &mut f);
        },

        Instr::ImmBorrowLoc(dst, src)
        | Instr::MutBorrowLoc(dst, src)
        | Instr::ImmBorrowField(dst, _, src)
        | Instr::MutBorrowField(dst, _, src)
        | Instr::ImmBorrowFieldGeneric(dst, _, src)
        | Instr::MutBorrowFieldGeneric(dst, _, src)
        | Instr::ImmBorrowVariantField(dst, _, src)
        | Instr::MutBorrowVariantField(dst, _, src)
        | Instr::ImmBorrowVariantFieldGeneric(dst, _, src)
        | Instr::MutBorrowVariantFieldGeneric(dst, _, src)
        | Instr::ReadRef(dst, src) => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*src, &mut f);
        },
        Instr::WriteRef(ref_slot, val) => {
            used::<USES>(*ref_slot, &mut f);
            used::<USES>(*val, &mut f);
        },

        Instr::ReadField(dst, _, src)
        | Instr::ReadFieldGeneric(dst, _, src)
        | Instr::ReadVariantField(dst, _, src)
        | Instr::ReadVariantFieldGeneric(dst, _, src) => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*src, &mut f);
        },
        Instr::WriteField(_, ref_slot, val)
        | Instr::WriteFieldGeneric(_, ref_slot, val)
        | Instr::WriteVariantField(_, ref_slot, val)
        | Instr::WriteVariantFieldGeneric(_, ref_slot, val) => {
            used::<USES>(*ref_slot, &mut f);
            used::<USES>(*val, &mut f);
        },

        Instr::Exists(dst, _, addr)
        | Instr::ExistsGeneric(dst, _, addr)
        | Instr::MoveFrom(dst, _, addr)
        | Instr::MoveFromGeneric(dst, _, addr) => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*addr, &mut f);
        },
        Instr::MoveTo(_, signer, val) | Instr::MoveToGeneric(_, signer, val) => {
            used::<USES>(*signer, &mut f);
            used::<USES>(*val, &mut f);
        },
        Instr::ImmBorrowGlobal(dst, _, addr)
        | Instr::ImmBorrowGlobalGeneric(dst, _, addr)
        | Instr::MutBorrowGlobal(dst, _, addr)
        | Instr::MutBorrowGlobalGeneric(dst, _, addr) => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*addr, &mut f);
        },

        Instr::Call(rets, _, args)
        | Instr::CallGeneric(rets, _, args)
        | Instr::CallClosure(rets, _, args) => {
            defs::<DEFS>(rets, &mut f);
            uses::<USES>(args, &mut f);
        },
        Instr::PackClosure(dst, _, _, captured)
        | Instr::PackClosureGeneric(dst, _, _, captured) => {
            def::<DEFS>(*dst, &mut f);
            uses::<USES>(captured, &mut f);
        },

        Instr::VecPack(dst, _, _, elems) => {
            def::<DEFS>(*dst, &mut f);
            uses::<USES>(elems, &mut f);
        },
        Instr::VecLen(dst, _, src) | Instr::VecPopBack(dst, _, src) => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*src, &mut f);
        },
        Instr::VecImmBorrow(dst, _, vec_ref, idx) | Instr::VecMutBorrow(dst, _, vec_ref, idx) => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*vec_ref, &mut f);
            used::<USES>(*idx, &mut f);
        },
        Instr::VecPushBack(_, vec_ref, val) => {
            used::<USES>(*vec_ref, &mut f);
            used::<USES>(*val, &mut f);
        },
        Instr::VecUnpack(dsts, _, _, src) => {
            defs::<DEFS>(dsts, &mut f);
            used::<USES>(*src, &mut f);
        },
        Instr::VecSwap(_, vec_ref, idx_a, idx_b) => {
            used::<USES>(*vec_ref, &mut f);
            used::<USES>(*idx_a, &mut f);
            used::<USES>(*idx_b, &mut f);
        },

        Instr::Branch(_) => {},
        Instr::BrTrue(_, cond) | Instr::BrFalse(_, cond) => used::<USES>(*cond, &mut f),
        Instr::Ret(rets) => uses::<USES>(rets, &mut f),
        Instr::Abort(code) => used::<USES>(*code, &mut f),
        Instr::AbortMsg(code, msg) => {
            used::<USES>(*code, &mut f);
            used::<USES>(*msg, &mut f);
        },
    }
}

// =============================================================================
// Internal: mutable slot rewriter core
// =============================================================================

/// Rewrite a slot in-place by applying `f`.
#[inline]
fn rewrite_slot(slot: &mut Slot, f: &mut impl FnMut(Slot) -> Slot) {
    *slot = f(*slot);
}

/// Rewrite each slot in a slice by applying `f`.
#[inline]
fn rewrite_slots(slots: &mut [Slot], f: &mut impl FnMut(Slot) -> Slot) {
    for slot in slots.iter_mut() {
        rewrite_slot(slot, f);
    }
}

/// Rewrite slot operands of an instruction in-place.
///
/// - `DEFS` / `USES`: select which slots to rewrite (compile-time).
/// - `SKIP_BORROW_LOC_SRC`: when true, BorrowLoc source operands are not
///   rewritten — needed for copy propagation where substituting BorrowLoc
///   sources would change which frame slot the reference points to (unsound).
fn rewrite_instr_slots<const DEFS: bool, const USES: bool, const SKIP_BORROW_LOC_SRC: bool>(
    instr: &mut Instr,
    mut f: impl FnMut(Slot) -> Slot,
) {
    match instr {
        Instr::LdConst(dst, _)
        | Instr::LdTrue(dst)
        | Instr::LdFalse(dst)
        | Instr::LdU8(dst, _)
        | Instr::LdU16(dst, _)
        | Instr::LdU32(dst, _)
        | Instr::LdU64(dst, _)
        | Instr::LdU128(dst, _)
        | Instr::LdU256(dst, _)
        | Instr::LdI8(dst, _)
        | Instr::LdI16(dst, _)
        | Instr::LdI32(dst, _)
        | Instr::LdI64(dst, _)
        | Instr::LdI128(dst, _)
        | Instr::LdI256(dst, _) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
        },

        Instr::Copy(dst, src) | Instr::Move(dst, src) | Instr::UnaryOp(dst, _, src) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(src, &mut f);
            }
        },
        Instr::BinaryOp(dst, _, lhs, rhs) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(lhs, &mut f);
                rewrite_slot(rhs, &mut f);
            }
        },
        Instr::BinaryOpImm(dst, _, lhs, _) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(lhs, &mut f);
            }
        },

        Instr::Pack(dst, _, fields)
        | Instr::PackGeneric(dst, _, fields)
        | Instr::PackVariant(dst, _, fields)
        | Instr::PackVariantGeneric(dst, _, fields) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slots(fields, &mut f);
            }
        },
        Instr::Unpack(dsts, _, src)
        | Instr::UnpackGeneric(dsts, _, src)
        | Instr::UnpackVariant(dsts, _, src)
        | Instr::UnpackVariantGeneric(dsts, _, src) => {
            if DEFS {
                rewrite_slots(dsts, &mut f);
            }
            if USES {
                rewrite_slot(src, &mut f);
            }
        },
        Instr::TestVariant(dst, _, src) | Instr::TestVariantGeneric(dst, _, src) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(src, &mut f);
            }
        },

        // BorrowLoc: when SKIP_BORROW_LOC_SRC is true, the source operand is a
        // storage-location use (identity of the slot matters) and must not be rewritten.
        Instr::ImmBorrowLoc(dst, src) | Instr::MutBorrowLoc(dst, src) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES && !SKIP_BORROW_LOC_SRC {
                rewrite_slot(src, &mut f);
            }
        },
        Instr::ImmBorrowField(dst, _, src)
        | Instr::MutBorrowField(dst, _, src)
        | Instr::ImmBorrowFieldGeneric(dst, _, src)
        | Instr::MutBorrowFieldGeneric(dst, _, src)
        | Instr::ImmBorrowVariantField(dst, _, src)
        | Instr::MutBorrowVariantField(dst, _, src)
        | Instr::ImmBorrowVariantFieldGeneric(dst, _, src)
        | Instr::MutBorrowVariantFieldGeneric(dst, _, src)
        | Instr::ReadRef(dst, src) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(src, &mut f);
            }
        },
        Instr::WriteRef(ref_slot, val) => {
            if USES {
                rewrite_slot(ref_slot, &mut f);
                rewrite_slot(val, &mut f);
            }
        },

        Instr::ReadField(dst, _, src)
        | Instr::ReadFieldGeneric(dst, _, src)
        | Instr::ReadVariantField(dst, _, src)
        | Instr::ReadVariantFieldGeneric(dst, _, src) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(src, &mut f);
            }
        },
        Instr::WriteField(_, ref_slot, val)
        | Instr::WriteFieldGeneric(_, ref_slot, val)
        | Instr::WriteVariantField(_, ref_slot, val)
        | Instr::WriteVariantFieldGeneric(_, ref_slot, val) => {
            if USES {
                rewrite_slot(ref_slot, &mut f);
                rewrite_slot(val, &mut f);
            }
        },

        Instr::Exists(dst, _, addr)
        | Instr::ExistsGeneric(dst, _, addr)
        | Instr::MoveFrom(dst, _, addr)
        | Instr::MoveFromGeneric(dst, _, addr) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(addr, &mut f);
            }
        },
        Instr::MoveTo(_, signer, val) | Instr::MoveToGeneric(_, signer, val) => {
            if USES {
                rewrite_slot(signer, &mut f);
                rewrite_slot(val, &mut f);
            }
        },
        Instr::ImmBorrowGlobal(dst, _, addr)
        | Instr::ImmBorrowGlobalGeneric(dst, _, addr)
        | Instr::MutBorrowGlobal(dst, _, addr)
        | Instr::MutBorrowGlobalGeneric(dst, _, addr) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(addr, &mut f);
            }
        },

        Instr::Call(rets, _, args)
        | Instr::CallGeneric(rets, _, args)
        | Instr::CallClosure(rets, _, args) => {
            if DEFS {
                rewrite_slots(rets, &mut f);
            }
            if USES {
                rewrite_slots(args, &mut f);
            }
        },
        Instr::PackClosure(dst, _, _, captured)
        | Instr::PackClosureGeneric(dst, _, _, captured) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slots(captured, &mut f);
            }
        },

        Instr::VecPack(dst, _, _, elems) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slots(elems, &mut f);
            }
        },
        Instr::VecLen(dst, _, src) | Instr::VecPopBack(dst, _, src) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(src, &mut f);
            }
        },
        Instr::VecImmBorrow(dst, _, vec_ref, idx) | Instr::VecMutBorrow(dst, _, vec_ref, idx) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(vec_ref, &mut f);
                rewrite_slot(idx, &mut f);
            }
        },
        Instr::VecPushBack(_, vec_ref, val) => {
            if USES {
                rewrite_slot(vec_ref, &mut f);
                rewrite_slot(val, &mut f);
            }
        },
        Instr::VecUnpack(dsts, _, _, src) => {
            if DEFS {
                rewrite_slots(dsts, &mut f);
            }
            if USES {
                rewrite_slot(src, &mut f);
            }
        },
        Instr::VecSwap(_, vec_ref, idx_a, idx_b) => {
            if USES {
                rewrite_slot(vec_ref, &mut f);
                rewrite_slot(idx_a, &mut f);
                rewrite_slot(idx_b, &mut f);
            }
        },

        Instr::Branch(_) => {},
        Instr::BrTrue(_, cond) | Instr::BrFalse(_, cond) => {
            if USES {
                rewrite_slot(cond, &mut f);
            }
        },
        Instr::Ret(rets) => {
            if USES {
                rewrite_slots(rets, &mut f);
            }
        },
        Instr::Abort(code) => {
            if USES {
                rewrite_slot(code, &mut f);
            }
        },
        Instr::AbortMsg(code, msg) => {
            if USES {
                rewrite_slot(code, &mut f);
                rewrite_slot(msg, &mut f);
            }
        },
    }
}
