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
//! - **Const generics** (`DEFS`, `USES`, `SKIP_PLACE_USE`): branches
//!   guarded by const booleans are eliminated during monomorphization. Each
//!   wrapper compiles to a specialized match with only the relevant arms.
//! - **Const folding of `SlotRole`**: the `def`/`used`/`defs`/`uses` emit
//!   helpers pass a statically known role tag. After inlining, the compiler
//!   constant-folds role checks in closures that filter or dispatch on role.
//! - **Inlining**: the emit helpers and `rewrite_slot`/`rewrite_slots` are
//!   `#[inline]` to ensure they are folded into the core match body. The
//!   caller-provided closures (`impl FnMut`) are monomorphized and inlined
//!   at each call site.

use super::{BinaryOp, ImmValue, Instr, Slot};
use mono_move_core::{types::InternedType, PreparedModule};
use smallvec::SmallVec;

/// Most instructions have at most 4 defs or uses.
pub(crate) type SlotList = SmallVec<[Slot; 4]>;

/// Whether a visited slot is a def (written) or a use (read).
#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotRole {
    Def,
    /// A value use — the slot's bytes are read into the operation.
    /// Single-use SSA semantics apply: after this instruction the
    /// originally-bound value is consumed.
    ValueUse,
    /// A place use — the frame slot's identity (its location) is
    /// referenced but its bytes are not consumed; the slot stays live
    /// with the same type after the instruction.
    PlaceUse,
}

// =============================================================================
// Read-only slot visitors
// =============================================================================

/// Apply `f` to each slot defined (written) by an instruction.
pub(crate) fn for_each_def(instr: &Instr, mut f: impl FnMut(Slot)) {
    visit_slots::<true, false>(instr, |slot, _| f(slot));
}

/// Apply `f` to each slot used (read) by an instruction. Includes
/// both value uses and place uses — the full union of
/// read-side operands.
pub(crate) fn for_each_use(instr: &Instr, mut f: impl FnMut(Slot)) {
    visit_slots::<false, true>(instr, |slot, _| f(slot));
}

/// Apply `f` to each slot whose value an instruction consumes,
/// skipping place uses.
pub(crate) fn for_each_value_use(instr: &Instr, mut f: impl FnMut(Slot)) {
    visit_slots::<false, true>(instr, |slot, role| {
        if role == SlotRole::ValueUse {
            f(slot);
        }
    });
}

/// Apply `f` to every slot (defs and uses) in an instruction.
pub(crate) fn for_each_slot(instr: &Instr, mut f: impl FnMut(Slot)) {
    visit_slots::<true, true>(instr, |slot, _| f(slot));
}

/// Collect defs and uses into separate lists in a single pass.
/// Place uses are grouped with value uses.
pub(crate) fn collect_defs_and_uses(instr: &Instr) -> (SlotList, SlotList) {
    let mut defs = SlotList::new();
    let mut uses = SlotList::new();
    visit_slots::<true, true>(instr, |slot, role| match role {
        SlotRole::Def => defs.push(slot),
        SlotRole::ValueUse | SlotRole::PlaceUse => uses.push(slot),
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

/// Call-like instructions (`Call`, `CallGeneric`, `CallClosure`) that clobber
/// Xfer slots.
#[inline]
pub(crate) fn clobbers_xfer(instr: &Instr) -> bool {
    matches!(
        instr,
        Instr::Call(..) | Instr::CallGeneric(..) | Instr::CallClosure(..)
    )
}

/// Resource type carried by a global-storage instruction (`exists`,
/// `move_from`, `move_to`, `borrow_global[_mut]`), if any. The returned type
/// is the interned resource nominal embedded in the instruction; it may still
/// contain type parameters that the caller substitutes with the function's
/// type arguments.
pub(crate) fn resource_type_in_instr(instr: &Instr) -> Option<InternedType> {
    match instr {
        Instr::Exists(_, ty, _)
        | Instr::ExistsGeneric(_, ty, _)
        | Instr::MoveFrom(_, ty, _)
        | Instr::MoveFromGeneric(_, ty, _)
        | Instr::MoveTo(ty, _, _)
        | Instr::MoveToGeneric(ty, _, _)
        | Instr::ImmBorrowGlobal(_, ty, _)
        | Instr::ImmBorrowGlobalGeneric(_, ty, _)
        | Instr::MutBorrowGlobal(_, ty, _)
        | Instr::MutBorrowGlobalGeneric(_, ty, _) => Some(*ty),
        _ => None,
    }
}

/// Concrete nominal (struct or enum) type whose layout `instr`'s
/// lowering needs, if any.
/// TODO: complete the function over all instructions.
pub(crate) fn nominal_type_in_instr(
    module: &PreparedModule,
    instr: &Instr,
) -> Option<InternedType> {
    use move_binary_format::access::ModuleAccess;
    match instr {
        // Carry the concrete struct type directly.
        Instr::Pack(_, ty, _) | Instr::Unpack(_, ty, _) => Some(*ty),

        // Carry a field handle resolving to the owning concrete struct.
        Instr::ImmBorrowField(_, fh, _)
        | Instr::MutBorrowField(_, fh, _)
        | Instr::ReadField(_, fh, _)
        | Instr::WriteField(fh, _, _)
        | Instr::ImmBorrowLocField(_, fh, _)
        | Instr::MutBorrowLocField(_, fh, _)
        | Instr::ReadLocalField(_, fh, _)
        | Instr::WriteLocalField(fh, _, _) => {
            let owner = module.field_handle_at(*fh).owner;
            Some(module.interned_nominal_def_type_at(owner))
        },

        // Generic struct/field, variant (enum), global-resource, vector,
        // and closure ops also carry types, but their lowering isn't
        // supported yet — none reference a concrete inline struct here.
        Instr::PackGeneric(..)
        | Instr::UnpackGeneric(..)
        | Instr::PackVariant(..)
        | Instr::PackVariantGeneric(..)
        | Instr::UnpackVariant(..)
        | Instr::UnpackVariantGeneric(..)
        | Instr::TestVariant(..)
        | Instr::TestVariantGeneric(..)
        | Instr::ImmBorrowFieldGeneric(..)
        | Instr::MutBorrowFieldGeneric(..)
        | Instr::ImmBorrowVariantField(..)
        | Instr::MutBorrowVariantField(..)
        | Instr::ImmBorrowVariantFieldGeneric(..)
        | Instr::MutBorrowVariantFieldGeneric(..)
        | Instr::ReadFieldGeneric(..)
        | Instr::WriteFieldGeneric(..)
        | Instr::ReadVariantField(..)
        | Instr::ReadVariantFieldGeneric(..)
        | Instr::WriteVariantField(..)
        | Instr::WriteVariantFieldGeneric(..)
        | Instr::Exists(..)
        | Instr::ExistsGeneric(..)
        | Instr::MoveFrom(..)
        | Instr::MoveFromGeneric(..)
        | Instr::MoveTo(..)
        | Instr::MoveToGeneric(..)
        | Instr::ImmBorrowGlobal(..)
        | Instr::ImmBorrowGlobalGeneric(..)
        | Instr::MutBorrowGlobal(..)
        | Instr::MutBorrowGlobalGeneric(..)
        | Instr::PackClosure(..)
        | Instr::PackClosureGeneric(..)
        | Instr::CallClosure(..)
        | Instr::VecPack(..)
        | Instr::VecLen(..)
        | Instr::VecImmBorrow(..)
        | Instr::VecMutBorrow(..)
        | Instr::VecPushBack(..)
        | Instr::VecPopBack(..)
        | Instr::VecUnpack(..)
        | Instr::VecSwap(..) => None,

        // No struct type involved.
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
        | Instr::LdI256(..)
        | Instr::Copy(..)
        | Instr::Move(..)
        | Instr::UnaryOp(..)
        | Instr::BinaryOp(..)
        | Instr::BinaryOpImm(..)
        | Instr::ImmBorrowLoc(..)
        | Instr::MutBorrowLoc(..)
        | Instr::ReadRef(..)
        | Instr::WriteRef(..)
        | Instr::Call(..)
        | Instr::CallGeneric(..)
        | Instr::Branch(..)
        | Instr::BrTrue(..)
        | Instr::BrFalse(..)
        | Instr::BrCmp(..)
        | Instr::BrCmpImm(..)
        | Instr::Ret(..)
        | Instr::Abort(..)
        | Instr::AbortMsg(..) => None,
    }
}

/// Whether `instr` is a terminator that falls through to the next block.
#[inline]
pub(crate) fn is_fallthrough_terminator(instr: &Instr) -> bool {
    match instr {
        Instr::BrTrue(..) | Instr::BrFalse(..) | Instr::BrCmp(..) | Instr::BrCmpImm(..) => true,

        Instr::Branch(..)
        | Instr::Ret(..)
        | Instr::Abort(..)
        | Instr::AbortMsg(..)
        | Instr::LdConst(..)
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
        | Instr::LdI256(..)
        | Instr::Copy(..)
        | Instr::Move(..)
        | Instr::UnaryOp(..)
        | Instr::BinaryOp(..)
        | Instr::BinaryOpImm(..)
        | Instr::Pack(..)
        | Instr::PackGeneric(..)
        | Instr::Unpack(..)
        | Instr::UnpackGeneric(..)
        | Instr::PackVariant(..)
        | Instr::PackVariantGeneric(..)
        | Instr::UnpackVariant(..)
        | Instr::UnpackVariantGeneric(..)
        | Instr::TestVariant(..)
        | Instr::TestVariantGeneric(..)
        | Instr::ImmBorrowLoc(..)
        | Instr::MutBorrowLoc(..)
        | Instr::ImmBorrowField(..)
        | Instr::MutBorrowField(..)
        | Instr::ImmBorrowFieldGeneric(..)
        | Instr::MutBorrowFieldGeneric(..)
        | Instr::ImmBorrowVariantField(..)
        | Instr::MutBorrowVariantField(..)
        | Instr::ImmBorrowVariantFieldGeneric(..)
        | Instr::MutBorrowVariantFieldGeneric(..)
        | Instr::ReadRef(..)
        | Instr::WriteRef(..)
        | Instr::ReadField(..)
        | Instr::ReadFieldGeneric(..)
        | Instr::WriteField(..)
        | Instr::WriteFieldGeneric(..)
        | Instr::ReadVariantField(..)
        | Instr::ReadVariantFieldGeneric(..)
        | Instr::WriteVariantField(..)
        | Instr::WriteVariantFieldGeneric(..)
        | Instr::ImmBorrowLocField(..)
        | Instr::MutBorrowLocField(..)
        | Instr::ReadLocalField(..)
        | Instr::WriteLocalField(..)
        | Instr::Exists(..)
        | Instr::ExistsGeneric(..)
        | Instr::MoveFrom(..)
        | Instr::MoveFromGeneric(..)
        | Instr::MoveTo(..)
        | Instr::MoveToGeneric(..)
        | Instr::ImmBorrowGlobal(..)
        | Instr::ImmBorrowGlobalGeneric(..)
        | Instr::MutBorrowGlobal(..)
        | Instr::MutBorrowGlobalGeneric(..)
        | Instr::Call(..)
        | Instr::CallGeneric(..)
        | Instr::PackClosure(..)
        | Instr::PackClosureGeneric(..)
        | Instr::CallClosure(..)
        | Instr::VecPack(..)
        | Instr::VecLen(..)
        | Instr::VecImmBorrow(..)
        | Instr::VecMutBorrow(..)
        | Instr::VecPushBack(..)
        | Instr::VecPopBack(..)
        | Instr::VecUnpack(..)
        | Instr::VecSwap(..) => false,
    }
}

/// Extract the destination slot and immediate value from a load instruction.
// TODO: the wide arms (`LdU128`/`LdU256`/`LdI128`/`LdI256`) each allocate
// a `Box` here, even when `try_fuse_immediate_binop` decides not to fuse.
// Consider splitting `extract_imm_value` into a cheap "would this fuse?"
// check + a separate constructor, or otherwise pulling allocation behind
// the fusion-eligibility check.
pub(crate) fn extract_imm_value(instr: &Instr) -> Option<(Slot, ImmValue)> {
    match instr {
        Instr::LdTrue(dst) => Some((*dst, ImmValue::Bool(true))),
        Instr::LdFalse(dst) => Some((*dst, ImmValue::Bool(false))),
        Instr::LdU8(dst, val) => Some((*dst, ImmValue::U8(*val))),
        Instr::LdU16(dst, val) => Some((*dst, ImmValue::U16(*val))),
        Instr::LdU32(dst, val) => Some((*dst, ImmValue::U32(*val))),
        Instr::LdU64(dst, val) => Some((*dst, ImmValue::U64(*val))),
        Instr::LdU128(dst, val) => Some((*dst, ImmValue::U128(Box::new(*val)))),
        Instr::LdU256(dst, val) => Some((*dst, ImmValue::U256(Box::new(*val)))),
        Instr::LdI8(dst, val) => Some((*dst, ImmValue::I8(*val))),
        Instr::LdI16(dst, val) => Some((*dst, ImmValue::I16(*val))),
        Instr::LdI32(dst, val) => Some((*dst, ImmValue::I32(*val))),
        Instr::LdI64(dst, val) => Some((*dst, ImmValue::I64(*val))),
        Instr::LdI128(dst, val) => Some((*dst, ImmValue::I128(Box::new(*val)))),
        Instr::LdI256(dst, val) => Some((*dst, ImmValue::I256(Box::new(*val)))),

        // `LdConst` loads from the constant pool — its payload isn't a
        // fixed-width integer literal, so it's never fusible into
        // `BinaryOpImm`.
        Instr::LdConst(_, _) => None,

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
        | Instr::PackVariant(_, _, _, _)
        | Instr::PackVariantGeneric(_, _, _, _)
        | Instr::UnpackVariant(_, _, _, _)
        | Instr::UnpackVariantGeneric(_, _, _, _)
        | Instr::TestVariant(_, _, _, _)
        | Instr::TestVariantGeneric(_, _, _, _)
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
        | Instr::ImmBorrowLocField(_, _, _)
        | Instr::MutBorrowLocField(_, _, _)
        | Instr::ReadLocalField(_, _, _)
        | Instr::WriteLocalField(_, _, _)
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
        | Instr::BrCmp(_, _, _, _)
        | Instr::BrCmpImm(_, _, _, _)
        | Instr::Ret(_)
        | Instr::Abort(_)
        | Instr::AbortMsg(_, _) => None,
    }
}

/// Whether a binary operation is commutative (i.e., operands can be swapped
/// without changing the result).
#[inline]
pub(crate) fn is_commutative(op: &BinaryOp) -> bool {
    use crate::stackless_exec_ir::CmpKind;
    matches!(
        op,
        BinaryOp::Add
            | BinaryOp::Mul
            | BinaryOp::BitOr
            | BinaryOp::BitAnd
            | BinaryOp::BitXor
            | BinaryOp::Cmp(CmpKind::Eq)
            | BinaryOp::Cmp(CmpKind::Neq)
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
        f(slot, SlotRole::ValueUse);
    }
}

/// Emit a place use slot if `ACTIVE` is true. The slot's
/// identity matters but its bytes are NOT consumed by the instruction.
#[inline]
fn storage_use<const ACTIVE: bool>(slot: Slot, f: &mut impl FnMut(Slot, SlotRole)) {
    if ACTIVE {
        f(slot, SlotRole::PlaceUse);
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
        slots.iter().for_each(|slot| f(*slot, SlotRole::ValueUse));
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

        Instr::Pack(dst, _, fields) => {
            def::<DEFS>(*dst, &mut f);
            uses::<USES>(fields, &mut f);
        },
        Instr::PackGeneric(dst, _, fields) | Instr::PackVariant(dst, _, _, fields) => {
            def::<DEFS>(*dst, &mut f);
            uses::<USES>(fields, &mut f);
        },
        Instr::PackVariantGeneric(dst, _, _, fields) => {
            def::<DEFS>(*dst, &mut f);
            uses::<USES>(fields, &mut f);
        },
        Instr::Unpack(dsts, _, src) => {
            defs::<DEFS>(dsts, &mut f);
            used::<USES>(*src, &mut f);
        },
        Instr::UnpackGeneric(dsts, _, src) | Instr::UnpackVariant(dsts, _, _, src) => {
            defs::<DEFS>(dsts, &mut f);
            used::<USES>(*src, &mut f);
        },
        Instr::UnpackVariantGeneric(dsts, _, _, src) => {
            defs::<DEFS>(dsts, &mut f);
            used::<USES>(*src, &mut f);
        },
        Instr::TestVariant(dst, _, _, src) => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*src, &mut f);
        },
        Instr::TestVariantGeneric(dst, _, _, src) => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*src, &mut f);
        },

        // `src` is a place use: the local's identity is
        // taken, its bytes are not consumed.
        Instr::ImmBorrowLoc(dst, src) | Instr::MutBorrowLoc(dst, src) => {
            def::<DEFS>(*dst, &mut f);
            storage_use::<USES>(*src, &mut f);
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

        // `local` names the inline-struct frame slot, not a reference:
        // a place use.
        Instr::ImmBorrowLocField(dst, _, local)
        | Instr::MutBorrowLocField(dst, _, local)
        | Instr::ReadLocalField(dst, _, local) => {
            def::<DEFS>(*dst, &mut f);
            storage_use::<USES>(*local, &mut f);
        },
        // `local` is both a def (a field is written in-place) and a
        // place use (the other fields persist, so the slot
        // stays live with the same type after the write).
        Instr::WriteLocalField(_, local, val) => {
            def::<DEFS>(*local, &mut f);
            storage_use::<USES>(*local, &mut f);
            used::<USES>(*val, &mut f);
        },

        Instr::Exists(dst, _, addr) | Instr::MoveFrom(dst, _, addr) => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*addr, &mut f);
        },
        Instr::ExistsGeneric(dst, _, addr) | Instr::MoveFromGeneric(dst, _, addr) => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*addr, &mut f);
        },
        Instr::MoveTo(_, signer, val) => {
            used::<USES>(*signer, &mut f);
            used::<USES>(*val, &mut f);
        },
        Instr::MoveToGeneric(_, signer, val) => {
            used::<USES>(*signer, &mut f);
            used::<USES>(*val, &mut f);
        },
        Instr::ImmBorrowGlobal(dst, _, addr) | Instr::MutBorrowGlobal(dst, _, addr) => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*addr, &mut f);
        },
        Instr::ImmBorrowGlobalGeneric(dst, _, addr)
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
        Instr::BrCmp(_, _, lhs, rhs) => {
            used::<USES>(*lhs, &mut f);
            used::<USES>(*rhs, &mut f);
        },
        Instr::BrCmpImm(_, _, src, _) => used::<USES>(*src, &mut f),
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
/// - `SKIP_PLACE_USE`: when true, place uses are not
///   rewritten.
fn rewrite_instr_slots<const DEFS: bool, const USES: bool, const SKIP_PLACE_USE: bool>(
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

        Instr::Pack(dst, _, fields) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slots(fields, &mut f);
            }
        },
        Instr::PackGeneric(dst, _, fields) | Instr::PackVariant(dst, _, _, fields) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slots(fields, &mut f);
            }
        },
        Instr::PackVariantGeneric(dst, _, _, fields) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slots(fields, &mut f);
            }
        },
        Instr::Unpack(dsts, _, src) => {
            if DEFS {
                rewrite_slots(dsts, &mut f);
            }
            if USES {
                rewrite_slot(src, &mut f);
            }
        },
        Instr::UnpackGeneric(dsts, _, src) | Instr::UnpackVariant(dsts, _, _, src) => {
            if DEFS {
                rewrite_slots(dsts, &mut f);
            }
            if USES {
                rewrite_slot(src, &mut f);
            }
        },
        Instr::UnpackVariantGeneric(dsts, _, _, src) => {
            if DEFS {
                rewrite_slots(dsts, &mut f);
            }
            if USES {
                rewrite_slot(src, &mut f);
            }
        },
        Instr::TestVariant(dst, _, _, src) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(src, &mut f);
            }
        },
        Instr::TestVariantGeneric(dst, _, _, src) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(src, &mut f);
            }
        },

        // `src` is a place use; skip it under SKIP_PLACE_USE.
        Instr::ImmBorrowLoc(dst, src) | Instr::MutBorrowLoc(dst, src) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES && !SKIP_PLACE_USE {
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

        // `local` is a place use, so skip it under
        // SKIP_PLACE_USE.
        Instr::ImmBorrowLocField(dst, _, local)
        | Instr::MutBorrowLocField(dst, _, local)
        | Instr::ReadLocalField(dst, _, local) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES && !SKIP_PLACE_USE {
                rewrite_slot(local, &mut f);
            }
        },
        Instr::WriteLocalField(_, local, val) => {
            // `local` is both a def and a place use of one
            // operand: rewrite once when either role is active. Under
            // SKIP_PLACE_USE only the use side is suppressed.
            if DEFS || (USES && !SKIP_PLACE_USE) {
                rewrite_slot(local, &mut f);
            }
            if USES {
                rewrite_slot(val, &mut f);
            }
        },

        Instr::Exists(dst, _, addr) | Instr::MoveFrom(dst, _, addr) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(addr, &mut f);
            }
        },
        Instr::ExistsGeneric(dst, _, addr) | Instr::MoveFromGeneric(dst, _, addr) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(addr, &mut f);
            }
        },
        Instr::MoveTo(_, signer, val) => {
            if USES {
                rewrite_slot(signer, &mut f);
                rewrite_slot(val, &mut f);
            }
        },
        Instr::MoveToGeneric(_, signer, val) => {
            if USES {
                rewrite_slot(signer, &mut f);
                rewrite_slot(val, &mut f);
            }
        },
        Instr::ImmBorrowGlobal(dst, _, addr) | Instr::MutBorrowGlobal(dst, _, addr) => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(addr, &mut f);
            }
        },
        Instr::ImmBorrowGlobalGeneric(dst, _, addr)
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
        Instr::BrCmp(_, _, lhs, rhs) => {
            if USES {
                rewrite_slot(lhs, &mut f);
                rewrite_slot(rhs, &mut f);
            }
        },
        Instr::BrCmpImm(_, _, src, _) => {
            if USES {
                rewrite_slot(src, &mut f);
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
