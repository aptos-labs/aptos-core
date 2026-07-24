// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Instruction utilities.
//!
//! Provides read-only slot visitors (`for_each_def`, `for_each_use`,
//! `for_each_slot`, `collect_defs_and_uses`), in-place slot rewriters
//! (`remap_all_slots_with`, `remap_source_slots_with`), and miscellaneous
//! instruction helpers (`call_boundary_rets_and_args`, `is_commutative`).
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

use super::{BinaryOp, FieldPath, Instr, Slot};
use mono_move_core::types::InternedType;
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

/// `(rets, args)` of a call-boundary instruction (`Call`, `CallClosure`),
/// `None` for everything else.
#[inline]
pub(crate) fn call_boundary_rets_and_args(instr: &Instr) -> Option<(&[Slot], &[Slot])> {
    match instr {
        Instr::Call { data } => Some((&data.rets, &data.args)),
        Instr::CallClosure { data } => Some((&data.rets, &data.args)),

        // Every other instruction: not a call boundary.
        Instr::LdConst { .. }
        | Instr::LdImm { .. }
        | Instr::Copy { .. }
        | Instr::Move { .. }
        | Instr::UnaryOp { .. }
        | Instr::BinaryOp { .. }
        | Instr::BinaryOpImm { .. }
        | Instr::Pack { .. }
        | Instr::Unpack { .. }
        | Instr::PackVariant { .. }
        | Instr::UnpackVariant { .. }
        | Instr::TestVariant { .. }
        | Instr::ImmBorrowLoc { .. }
        | Instr::MutBorrowLoc { .. }
        | Instr::ImmBorrowField { .. }
        | Instr::MutBorrowField { .. }
        | Instr::ImmBorrowVariantField { .. }
        | Instr::MutBorrowVariantField { .. }
        | Instr::ReadRef { .. }
        | Instr::WriteRef { .. }
        | Instr::ReadField { .. }
        | Instr::WriteField { .. }
        | Instr::ReadVariantField { .. }
        | Instr::WriteVariantField { .. }
        | Instr::ImmBorrowLocField { .. }
        | Instr::MutBorrowLocField { .. }
        | Instr::ReadLocField { .. }
        | Instr::WriteLocField { .. }
        | Instr::ReadFieldChain { .. }
        | Instr::WriteFieldChain { .. }
        | Instr::ImmBorrowFieldChain { .. }
        | Instr::MutBorrowFieldChain { .. }
        | Instr::ReadLocFieldChain { .. }
        | Instr::WriteLocFieldChain { .. }
        | Instr::ImmBorrowLocFieldChain { .. }
        | Instr::MutBorrowLocFieldChain { .. }
        | Instr::Exists { .. }
        | Instr::MoveFrom { .. }
        | Instr::MoveTo { .. }
        | Instr::ImmBorrowGlobal { .. }
        | Instr::MutBorrowGlobal { .. }
        | Instr::PackClosure { .. }
        | Instr::VecPack { .. }
        | Instr::VecLen { .. }
        | Instr::VecImmBorrow { .. }
        | Instr::VecMutBorrow { .. }
        | Instr::VecPushBack { .. }
        | Instr::VecPopBack { .. }
        | Instr::VecUnpack { .. }
        | Instr::VecSwap { .. }
        | Instr::Branch { .. }
        | Instr::BrTrue { .. }
        | Instr::BrFalse { .. }
        | Instr::BrCmp { .. }
        | Instr::BrCmpImm { .. }
        | Instr::Ret { .. }
        | Instr::Abort { .. }
        | Instr::AbortMsg { .. }
        | Instr::ForceGC => None,
    }
}

/// Call-like instructions (`Call`, `CallClosure`) that clobber Xfer slots.
#[inline]
pub(crate) fn clobbers_xfer(instr: &Instr) -> bool {
    call_boundary_rets_and_args(instr).is_some()
}

/// The local whose storage `instr` mutably borrows, if any. A later write
/// through that borrow mutates the local without a def at the write site (a
/// hidden write), so coalescing and copy propagation both derive their
/// guards from this single predicate.
pub(crate) fn mut_local_borrow_target(instr: &Instr) -> Option<Slot> {
    match instr {
        // Mutable borrows of a local's storage.
        Instr::MutBorrowLoc { local, .. }
        | Instr::MutBorrowLocField { local, .. }
        | Instr::MutBorrowLocFieldChain { local, .. } => Some(*local),

        // Every other instruction: no local storage is mutably borrowed.
        Instr::Pack { .. }
        | Instr::Unpack { .. }
        | Instr::PackVariant { .. }
        | Instr::UnpackVariant { .. }
        | Instr::TestVariant { .. }
        | Instr::ImmBorrowField { .. }
        | Instr::MutBorrowField { .. }
        | Instr::ReadField { .. }
        | Instr::WriteField { .. }
        | Instr::ImmBorrowLocField { .. }
        | Instr::ReadLocField { .. }
        | Instr::WriteLocField { .. }
        | Instr::ReadFieldChain { .. }
        | Instr::WriteFieldChain { .. }
        | Instr::ImmBorrowFieldChain { .. }
        | Instr::MutBorrowFieldChain { .. }
        | Instr::ReadLocFieldChain { .. }
        | Instr::WriteLocFieldChain { .. }
        | Instr::ImmBorrowLocFieldChain { .. }
        | Instr::ImmBorrowVariantField { .. }
        | Instr::MutBorrowVariantField { .. }
        | Instr::ReadVariantField { .. }
        | Instr::WriteVariantField { .. }
        | Instr::PackClosure { .. }
        | Instr::CallClosure { .. }
        | Instr::Exists { .. }
        | Instr::MoveFrom { .. }
        | Instr::MoveTo { .. }
        | Instr::ImmBorrowGlobal { .. }
        | Instr::MutBorrowGlobal { .. }
        | Instr::VecPack { .. }
        | Instr::VecLen { .. }
        | Instr::VecImmBorrow { .. }
        | Instr::VecMutBorrow { .. }
        | Instr::VecPushBack { .. }
        | Instr::VecPopBack { .. }
        | Instr::VecUnpack { .. }
        | Instr::VecSwap { .. }
        | Instr::LdConst { .. }
        | Instr::LdImm { .. }
        | Instr::Copy { .. }
        | Instr::Move { .. }
        | Instr::UnaryOp { .. }
        | Instr::BinaryOp { .. }
        | Instr::BinaryOpImm { .. }
        | Instr::ImmBorrowLoc { .. }
        | Instr::ReadRef { .. }
        | Instr::WriteRef { .. }
        | Instr::Call { .. }
        | Instr::Branch { .. }
        | Instr::BrTrue { .. }
        | Instr::BrFalse { .. }
        | Instr::BrCmp { .. }
        | Instr::BrCmpImm { .. }
        | Instr::Ret { .. }
        | Instr::Abort { .. }
        | Instr::AbortMsg { .. }
        | Instr::ForceGC => None,
    }
}

/// Resource type carried by a global-storage instruction (`exists`,
/// `move_from`, `move_to`, `borrow_global[_mut]`), if any. The returned type
/// is the interned resource nominal embedded in the instruction; it may still
/// contain type parameters that the caller substitutes with the function's
/// type arguments.
pub(crate) fn resource_type_in_instr(instr: &Instr) -> Option<InternedType> {
    match instr {
        // Global-storage ops carry the resource nominal directly.
        Instr::Exists { resource_ty, .. }
        | Instr::MoveFrom { resource_ty, .. }
        | Instr::MoveTo { resource_ty, .. }
        | Instr::ImmBorrowGlobal { resource_ty, .. }
        | Instr::MutBorrowGlobal { resource_ty, .. } => Some(*resource_ty),

        // Every other instruction: no resource type involved.
        Instr::LdImm { .. }
        | Instr::Pack { .. }
        | Instr::Unpack { .. }
        | Instr::PackVariant { .. }
        | Instr::UnpackVariant { .. }
        | Instr::TestVariant { .. }
        | Instr::ImmBorrowField { .. }
        | Instr::MutBorrowField { .. }
        | Instr::ReadField { .. }
        | Instr::WriteField { .. }
        | Instr::ImmBorrowLocField { .. }
        | Instr::MutBorrowLocField { .. }
        | Instr::ReadLocField { .. }
        | Instr::WriteLocField { .. }
        | Instr::ReadFieldChain { .. }
        | Instr::WriteFieldChain { .. }
        | Instr::ImmBorrowFieldChain { .. }
        | Instr::MutBorrowFieldChain { .. }
        | Instr::ReadLocFieldChain { .. }
        | Instr::WriteLocFieldChain { .. }
        | Instr::ImmBorrowLocFieldChain { .. }
        | Instr::MutBorrowLocFieldChain { .. }
        | Instr::ImmBorrowVariantField { .. }
        | Instr::MutBorrowVariantField { .. }
        | Instr::ReadVariantField { .. }
        | Instr::WriteVariantField { .. }
        | Instr::PackClosure { .. }
        | Instr::CallClosure { .. }
        | Instr::VecPack { .. }
        | Instr::VecLen { .. }
        | Instr::VecImmBorrow { .. }
        | Instr::VecMutBorrow { .. }
        | Instr::VecPushBack { .. }
        | Instr::VecPopBack { .. }
        | Instr::VecUnpack { .. }
        | Instr::VecSwap { .. }
        | Instr::LdConst { .. }
        | Instr::Copy { .. }
        | Instr::Move { .. }
        | Instr::UnaryOp { .. }
        | Instr::BinaryOp { .. }
        | Instr::BinaryOpImm { .. }
        | Instr::ImmBorrowLoc { .. }
        | Instr::MutBorrowLoc { .. }
        | Instr::ReadRef { .. }
        | Instr::WriteRef { .. }
        | Instr::Call { .. }
        | Instr::Branch { .. }
        | Instr::BrTrue { .. }
        | Instr::BrFalse { .. }
        | Instr::BrCmp { .. }
        | Instr::BrCmpImm { .. }
        | Instr::Ret { .. }
        | Instr::Abort { .. }
        | Instr::AbortMsg { .. }
        | Instr::ForceGC => None,
    }
}

/// Whether a nominal type is a struct or an enum.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum NominalKind {
    Struct,
    Enum,
}

/// Returns the instantiated struct/enum whose field or variant layout `instr`
/// uses for construction, destruction, or field/variant access, with its kind;
/// `None` if no such nominal is used. In generic functions, the type may
/// contain the enclosing function's `TypeParam`s.
///
/// `None` also covers global-storage ops (whole-value size only), vector ops
/// (element stride only), and closure ops (separate capture-object layout);
/// their nominals are found via `resource_type_in_instr`, the slot-type walk,
/// and captured-data discovery.
///
/// TODO(cleanup): reconcile various type-in-instr discovery methods.
pub(crate) fn field_layout_nominal_in_instr(instr: &Instr) -> Option<(InternedType, NominalKind)> {
    match instr {
        // Carry the instantiated struct type directly.
        Instr::Pack { struct_ty, .. } | Instr::Unpack { struct_ty, .. } => {
            Some((*struct_ty, NominalKind::Struct))
        },

        // Field ops carry the instantiated owner struct type.
        Instr::ImmBorrowField { owner_ty, .. }
        | Instr::MutBorrowField { owner_ty, .. }
        | Instr::ReadField { owner_ty, .. }
        | Instr::WriteField { owner_ty, .. }
        | Instr::ImmBorrowLocField { owner_ty, .. }
        | Instr::MutBorrowLocField { owner_ty, .. }
        | Instr::ReadLocField { owner_ty, .. }
        | Instr::WriteLocField { owner_ty, .. } => Some((*owner_ty, NominalKind::Struct)),

        // A struct chain's first owner contains every later owner as an inline
        // field (each later owner is the previous hop's field type), so
        // discovering it transitively lays out the whole path. The containment
        // invariant is debug-checked at the discovery site.
        Instr::ReadFieldChain { path, .. }
        | Instr::WriteFieldChain { path, .. }
        | Instr::ImmBorrowFieldChain { path, .. }
        | Instr::MutBorrowFieldChain { path, .. }
        | Instr::ReadLocFieldChain { path, .. }
        | Instr::WriteLocFieldChain { path, .. }
        | Instr::ImmBorrowLocFieldChain { path, .. }
        | Instr::MutBorrowLocFieldChain { path, .. } => {
            path.first().map(|(owner, _)| (*owner, NominalKind::Struct))
        },

        // Variant ops carry the instantiated enum type directly.
        Instr::PackVariant { enum_ty, .. }
        | Instr::UnpackVariant { enum_ty, .. }
        | Instr::TestVariant { enum_ty, .. } => Some((*enum_ty, NominalKind::Enum)),

        // Variant-field ops carry the instantiated owner enum type.
        Instr::ImmBorrowVariantField { owner_ty, .. }
        | Instr::MutBorrowVariantField { owner_ty, .. }
        | Instr::ReadVariantField { owner_ty, .. }
        | Instr::WriteVariantField { owner_ty, .. } => Some((*owner_ty, NominalKind::Enum)),

        // Global-resource ops surface their type via `resource_type_in_instr`
        // (they need whole-value size, not field offsets); vector ops need only
        // element stride and closure ops a capture-object layout, so none has a
        // field-layout nominal to report here.
        Instr::Exists { .. }
        | Instr::MoveFrom { .. }
        | Instr::MoveTo { .. }
        | Instr::ImmBorrowGlobal { .. }
        | Instr::MutBorrowGlobal { .. }
        | Instr::PackClosure { .. }
        | Instr::CallClosure { .. }
        | Instr::VecPack { .. }
        | Instr::VecLen { .. }
        | Instr::VecImmBorrow { .. }
        | Instr::VecMutBorrow { .. }
        | Instr::VecPushBack { .. }
        | Instr::VecPopBack { .. }
        | Instr::VecUnpack { .. }
        | Instr::VecSwap { .. } => None,

        // No struct type involved.
        Instr::LdConst { .. }
        | Instr::LdImm { .. }
        | Instr::Copy { .. }
        | Instr::Move { .. }
        | Instr::UnaryOp { .. }
        | Instr::BinaryOp { .. }
        | Instr::BinaryOpImm { .. }
        | Instr::ImmBorrowLoc { .. }
        | Instr::MutBorrowLoc { .. }
        | Instr::ReadRef { .. }
        | Instr::WriteRef { .. }
        | Instr::Call { .. }
        | Instr::Branch { .. }
        | Instr::BrTrue { .. }
        | Instr::BrFalse { .. }
        | Instr::BrCmp { .. }
        | Instr::BrCmpImm { .. }
        | Instr::Ret { .. }
        | Instr::Abort { .. }
        | Instr::AbortMsg { .. }
        | Instr::ForceGC => None,
    }
}

/// The fused-chain `FieldPath` carried by `instr`, if any.
pub(crate) fn chain_field_path(instr: &Instr) -> Option<&FieldPath> {
    match instr {
        Instr::ReadFieldChain { path, .. }
        | Instr::WriteFieldChain { path, .. }
        | Instr::ImmBorrowFieldChain { path, .. }
        | Instr::MutBorrowFieldChain { path, .. }
        | Instr::ReadLocFieldChain { path, .. }
        | Instr::WriteLocFieldChain { path, .. }
        | Instr::ImmBorrowLocFieldChain { path, .. }
        | Instr::MutBorrowLocFieldChain { path, .. } => Some(path),

        // Every other instruction: no fused chain.
        Instr::Pack { .. }
        | Instr::Unpack { .. }
        | Instr::PackVariant { .. }
        | Instr::UnpackVariant { .. }
        | Instr::TestVariant { .. }
        | Instr::ImmBorrowField { .. }
        | Instr::MutBorrowField { .. }
        | Instr::ReadField { .. }
        | Instr::WriteField { .. }
        | Instr::ImmBorrowLocField { .. }
        | Instr::MutBorrowLocField { .. }
        | Instr::ReadLocField { .. }
        | Instr::WriteLocField { .. }
        | Instr::ImmBorrowVariantField { .. }
        | Instr::MutBorrowVariantField { .. }
        | Instr::ReadVariantField { .. }
        | Instr::WriteVariantField { .. }
        | Instr::PackClosure { .. }
        | Instr::CallClosure { .. }
        | Instr::Exists { .. }
        | Instr::MoveFrom { .. }
        | Instr::MoveTo { .. }
        | Instr::ImmBorrowGlobal { .. }
        | Instr::MutBorrowGlobal { .. }
        | Instr::VecPack { .. }
        | Instr::VecLen { .. }
        | Instr::VecImmBorrow { .. }
        | Instr::VecMutBorrow { .. }
        | Instr::VecPushBack { .. }
        | Instr::VecPopBack { .. }
        | Instr::VecUnpack { .. }
        | Instr::VecSwap { .. }
        | Instr::LdConst { .. }
        | Instr::LdImm { .. }
        | Instr::Copy { .. }
        | Instr::Move { .. }
        | Instr::UnaryOp { .. }
        | Instr::BinaryOp { .. }
        | Instr::BinaryOpImm { .. }
        | Instr::ImmBorrowLoc { .. }
        | Instr::MutBorrowLoc { .. }
        | Instr::ReadRef { .. }
        | Instr::WriteRef { .. }
        | Instr::Call { .. }
        | Instr::Branch { .. }
        | Instr::BrTrue { .. }
        | Instr::BrFalse { .. }
        | Instr::BrCmp { .. }
        | Instr::BrCmpImm { .. }
        | Instr::Ret { .. }
        | Instr::Abort { .. }
        | Instr::AbortMsg { .. }
        | Instr::ForceGC => None,
    }
}

/// Whether `instr` is a terminator that falls through to the next block.
#[inline]
pub(crate) fn is_fallthrough_terminator(instr: &Instr) -> bool {
    match instr {
        Instr::BrTrue { .. }
        | Instr::BrFalse { .. }
        | Instr::BrCmp { .. }
        | Instr::BrCmpImm { .. } => true,

        Instr::Branch { .. }
        | Instr::Ret { .. }
        | Instr::Abort { .. }
        | Instr::AbortMsg { .. }
        | Instr::LdConst { .. }
        | Instr::LdImm { .. }
        | Instr::Copy { .. }
        | Instr::Move { .. }
        | Instr::UnaryOp { .. }
        | Instr::BinaryOp { .. }
        | Instr::BinaryOpImm { .. }
        | Instr::Pack { .. }
        | Instr::Unpack { .. }
        | Instr::PackVariant { .. }
        | Instr::UnpackVariant { .. }
        | Instr::TestVariant { .. }
        | Instr::ImmBorrowLoc { .. }
        | Instr::MutBorrowLoc { .. }
        | Instr::ImmBorrowField { .. }
        | Instr::MutBorrowField { .. }
        | Instr::ImmBorrowVariantField { .. }
        | Instr::MutBorrowVariantField { .. }
        | Instr::ReadRef { .. }
        | Instr::WriteRef { .. }
        | Instr::ReadField { .. }
        | Instr::WriteField { .. }
        | Instr::ReadVariantField { .. }
        | Instr::WriteVariantField { .. }
        | Instr::ImmBorrowLocField { .. }
        | Instr::MutBorrowLocField { .. }
        | Instr::ReadLocField { .. }
        | Instr::WriteLocField { .. }
        | Instr::ReadFieldChain { .. }
        | Instr::WriteFieldChain { .. }
        | Instr::ImmBorrowFieldChain { .. }
        | Instr::MutBorrowFieldChain { .. }
        | Instr::ReadLocFieldChain { .. }
        | Instr::WriteLocFieldChain { .. }
        | Instr::ImmBorrowLocFieldChain { .. }
        | Instr::MutBorrowLocFieldChain { .. }
        | Instr::Exists { .. }
        | Instr::MoveFrom { .. }
        | Instr::MoveTo { .. }
        | Instr::ImmBorrowGlobal { .. }
        | Instr::MutBorrowGlobal { .. }
        | Instr::Call { .. }
        | Instr::PackClosure { .. }
        | Instr::CallClosure { .. }
        | Instr::VecPack { .. }
        | Instr::VecLen { .. }
        | Instr::VecImmBorrow { .. }
        | Instr::VecMutBorrow { .. }
        | Instr::VecPushBack { .. }
        | Instr::VecPopBack { .. }
        | Instr::VecUnpack { .. }
        | Instr::VecSwap { .. }
        | Instr::ForceGC => false,
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
        Instr::LdConst { dst, .. } | Instr::LdImm { dst, .. } => def::<DEFS>(*dst, &mut f),

        Instr::Copy { dst, src } | Instr::Move { dst, src } | Instr::UnaryOp { dst, src, .. } => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*src, &mut f);
        },
        Instr::BinaryOp { dst, lhs, rhs, .. } => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*lhs, &mut f);
            used::<USES>(*rhs, &mut f);
        },
        Instr::BinaryOpImm { dst, lhs, .. } => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*lhs, &mut f);
        },

        Instr::Pack { dst, srcs, .. } | Instr::PackVariant { dst, srcs, .. } => {
            def::<DEFS>(*dst, &mut f);
            uses::<USES>(srcs, &mut f);
        },
        Instr::Unpack { dsts, src, .. } | Instr::UnpackVariant { dsts, src, .. } => {
            defs::<DEFS>(dsts, &mut f);
            used::<USES>(*src, &mut f);
        },
        Instr::TestVariant { dst, src, .. } => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*src, &mut f);
        },

        // `local` is a place use: the local's identity is
        // taken, its bytes are not consumed.
        Instr::ImmBorrowLoc { dst, local } | Instr::MutBorrowLoc { dst, local } => {
            def::<DEFS>(*dst, &mut f);
            storage_use::<USES>(*local, &mut f);
        },
        // Field chains share the slot shape of their single-field counterparts;
        // only the carried `path` differs.
        Instr::ImmBorrowField { dst, src, .. }
        | Instr::MutBorrowField { dst, src, .. }
        | Instr::ImmBorrowVariantField { dst, src, .. }
        | Instr::MutBorrowVariantField { dst, src, .. }
        | Instr::ImmBorrowFieldChain { dst, src, .. }
        | Instr::MutBorrowFieldChain { dst, src, .. }
        | Instr::ReadRef { dst, src } => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*src, &mut f);
        },
        Instr::WriteRef { dst_ref, val } => {
            used::<USES>(*dst_ref, &mut f);
            used::<USES>(*val, &mut f);
        },

        Instr::ReadField { dst, src, .. }
        | Instr::ReadVariantField { dst, src, .. }
        | Instr::ReadFieldChain { dst, src, .. } => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*src, &mut f);
        },
        Instr::WriteField { dst_ref, val, .. }
        | Instr::WriteVariantField { dst_ref, val, .. }
        | Instr::WriteFieldChain { dst_ref, val, .. } => {
            used::<USES>(*dst_ref, &mut f);
            used::<USES>(*val, &mut f);
        },

        // `local` names the inline-struct frame slot, not a reference:
        // a place use.
        Instr::ImmBorrowLocField { dst, local, .. }
        | Instr::MutBorrowLocField { dst, local, .. }
        | Instr::ReadLocField { dst, local, .. }
        | Instr::ImmBorrowLocFieldChain { dst, local, .. }
        | Instr::MutBorrowLocFieldChain { dst, local, .. }
        | Instr::ReadLocFieldChain { dst, local, .. } => {
            def::<DEFS>(*dst, &mut f);
            storage_use::<USES>(*local, &mut f);
        },
        // `local` is both a def (a field is written in-place) and a
        // place use (the other fields persist, so the slot
        // stays live with the same type after the write).
        Instr::WriteLocField { local, val, .. } | Instr::WriteLocFieldChain { local, val, .. } => {
            def::<DEFS>(*local, &mut f);
            storage_use::<USES>(*local, &mut f);
            used::<USES>(*val, &mut f);
        },

        Instr::Exists { dst, addr, .. } | Instr::MoveFrom { dst, addr, .. } => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*addr, &mut f);
        },
        Instr::MoveTo { signer, val, .. } => {
            used::<USES>(*signer, &mut f);
            used::<USES>(*val, &mut f);
        },
        Instr::ImmBorrowGlobal { dst, addr, .. } | Instr::MutBorrowGlobal { dst, addr, .. } => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*addr, &mut f);
        },

        Instr::Call { data } => {
            defs::<DEFS>(&data.rets, &mut f);
            uses::<USES>(&data.args, &mut f);
        },
        Instr::CallClosure { data } => {
            defs::<DEFS>(&data.rets, &mut f);
            uses::<USES>(&data.args, &mut f);
        },
        Instr::PackClosure { data } => {
            def::<DEFS>(data.dst, &mut f);
            uses::<USES>(&data.captured, &mut f);
        },

        Instr::VecPack { dst, srcs, .. } => {
            def::<DEFS>(*dst, &mut f);
            uses::<USES>(srcs, &mut f);
        },
        Instr::VecLen { dst, vec_ref, .. } | Instr::VecPopBack { dst, vec_ref, .. } => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*vec_ref, &mut f);
        },
        Instr::VecImmBorrow {
            dst, vec_ref, idx, ..
        }
        | Instr::VecMutBorrow {
            dst, vec_ref, idx, ..
        } => {
            def::<DEFS>(*dst, &mut f);
            used::<USES>(*vec_ref, &mut f);
            used::<USES>(*idx, &mut f);
        },
        Instr::VecPushBack { vec_ref, val, .. } => {
            used::<USES>(*vec_ref, &mut f);
            used::<USES>(*val, &mut f);
        },
        Instr::VecUnpack { dsts, src, .. } => {
            defs::<DEFS>(dsts, &mut f);
            used::<USES>(*src, &mut f);
        },
        Instr::VecSwap {
            vec_ref,
            idx_a,
            idx_b,
            ..
        } => {
            used::<USES>(*vec_ref, &mut f);
            used::<USES>(*idx_a, &mut f);
            used::<USES>(*idx_b, &mut f);
        },

        Instr::Branch { .. } => {},
        Instr::BrTrue { cond, .. } | Instr::BrFalse { cond, .. } => used::<USES>(*cond, &mut f),
        Instr::BrCmp { lhs, rhs, .. } => {
            used::<USES>(*lhs, &mut f);
            used::<USES>(*rhs, &mut f);
        },
        Instr::BrCmpImm { lhs, .. } => used::<USES>(*lhs, &mut f),
        Instr::Ret { srcs } => uses::<USES>(srcs, &mut f),
        Instr::Abort { code } => used::<USES>(*code, &mut f),
        Instr::AbortMsg { code, msg } => {
            used::<USES>(*code, &mut f);
            used::<USES>(*msg, &mut f);
        },

        // No slot operands.
        Instr::ForceGC => {},
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
        Instr::LdConst { dst, .. } | Instr::LdImm { dst, .. } => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
        },

        Instr::Copy { dst, src } | Instr::Move { dst, src } | Instr::UnaryOp { dst, src, .. } => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(src, &mut f);
            }
        },
        Instr::BinaryOp { dst, lhs, rhs, .. } => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(lhs, &mut f);
                rewrite_slot(rhs, &mut f);
            }
        },
        Instr::BinaryOpImm { dst, lhs, .. } => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(lhs, &mut f);
            }
        },

        Instr::Pack { dst, srcs, .. } | Instr::PackVariant { dst, srcs, .. } => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slots(srcs, &mut f);
            }
        },
        Instr::Unpack { dsts, src, .. } | Instr::UnpackVariant { dsts, src, .. } => {
            if DEFS {
                rewrite_slots(dsts, &mut f);
            }
            if USES {
                rewrite_slot(src, &mut f);
            }
        },
        Instr::TestVariant { dst, src, .. } => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(src, &mut f);
            }
        },

        // `local` is a place use; skip it under SKIP_PLACE_USE.
        Instr::ImmBorrowLoc { dst, local } | Instr::MutBorrowLoc { dst, local } => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES && !SKIP_PLACE_USE {
                rewrite_slot(local, &mut f);
            }
        },
        // Field chains share the slot shape of their single-field counterparts.
        Instr::ImmBorrowField { dst, src, .. }
        | Instr::MutBorrowField { dst, src, .. }
        | Instr::ImmBorrowVariantField { dst, src, .. }
        | Instr::MutBorrowVariantField { dst, src, .. }
        | Instr::ImmBorrowFieldChain { dst, src, .. }
        | Instr::MutBorrowFieldChain { dst, src, .. }
        | Instr::ReadRef { dst, src } => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(src, &mut f);
            }
        },
        Instr::WriteRef { dst_ref, val } => {
            if USES {
                rewrite_slot(dst_ref, &mut f);
                rewrite_slot(val, &mut f);
            }
        },

        Instr::ReadField { dst, src, .. }
        | Instr::ReadVariantField { dst, src, .. }
        | Instr::ReadFieldChain { dst, src, .. } => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(src, &mut f);
            }
        },
        Instr::WriteField { dst_ref, val, .. }
        | Instr::WriteVariantField { dst_ref, val, .. }
        | Instr::WriteFieldChain { dst_ref, val, .. } => {
            if USES {
                rewrite_slot(dst_ref, &mut f);
                rewrite_slot(val, &mut f);
            }
        },

        // `local` is a place use, so skip it under
        // SKIP_PLACE_USE.
        Instr::ImmBorrowLocField { dst, local, .. }
        | Instr::MutBorrowLocField { dst, local, .. }
        | Instr::ReadLocField { dst, local, .. }
        | Instr::ImmBorrowLocFieldChain { dst, local, .. }
        | Instr::MutBorrowLocFieldChain { dst, local, .. }
        | Instr::ReadLocFieldChain { dst, local, .. } => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES && !SKIP_PLACE_USE {
                rewrite_slot(local, &mut f);
            }
        },
        Instr::WriteLocField { local, val, .. } | Instr::WriteLocFieldChain { local, val, .. } => {
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

        Instr::Exists { dst, addr, .. } | Instr::MoveFrom { dst, addr, .. } => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(addr, &mut f);
            }
        },
        Instr::MoveTo { signer, val, .. } => {
            if USES {
                rewrite_slot(signer, &mut f);
                rewrite_slot(val, &mut f);
            }
        },
        Instr::ImmBorrowGlobal { dst, addr, .. } | Instr::MutBorrowGlobal { dst, addr, .. } => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(addr, &mut f);
            }
        },

        Instr::Call { data } => {
            if DEFS {
                rewrite_slots(&mut data.rets, &mut f);
            }
            if USES {
                rewrite_slots(&mut data.args, &mut f);
            }
        },
        Instr::CallClosure { data } => {
            if DEFS {
                rewrite_slots(&mut data.rets, &mut f);
            }
            if USES {
                rewrite_slots(&mut data.args, &mut f);
            }
        },
        Instr::PackClosure { data } => {
            if DEFS {
                rewrite_slot(&mut data.dst, &mut f);
            }
            if USES {
                rewrite_slots(&mut data.captured, &mut f);
            }
        },

        Instr::VecPack { dst, srcs, .. } => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slots(srcs, &mut f);
            }
        },
        Instr::VecLen { dst, vec_ref, .. } | Instr::VecPopBack { dst, vec_ref, .. } => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(vec_ref, &mut f);
            }
        },
        Instr::VecImmBorrow {
            dst, vec_ref, idx, ..
        }
        | Instr::VecMutBorrow {
            dst, vec_ref, idx, ..
        } => {
            if DEFS {
                rewrite_slot(dst, &mut f);
            }
            if USES {
                rewrite_slot(vec_ref, &mut f);
                rewrite_slot(idx, &mut f);
            }
        },
        Instr::VecPushBack { vec_ref, val, .. } => {
            if USES {
                rewrite_slot(vec_ref, &mut f);
                rewrite_slot(val, &mut f);
            }
        },
        Instr::VecUnpack { dsts, src, .. } => {
            if DEFS {
                rewrite_slots(dsts, &mut f);
            }
            if USES {
                rewrite_slot(src, &mut f);
            }
        },
        Instr::VecSwap {
            vec_ref,
            idx_a,
            idx_b,
            ..
        } => {
            if USES {
                rewrite_slot(vec_ref, &mut f);
                rewrite_slot(idx_a, &mut f);
                rewrite_slot(idx_b, &mut f);
            }
        },

        Instr::Branch { .. } => {},
        Instr::BrTrue { cond, .. } | Instr::BrFalse { cond, .. } => {
            if USES {
                rewrite_slot(cond, &mut f);
            }
        },
        Instr::BrCmp { lhs, rhs, .. } => {
            if USES {
                rewrite_slot(lhs, &mut f);
                rewrite_slot(rhs, &mut f);
            }
        },
        Instr::BrCmpImm { lhs, .. } => {
            if USES {
                rewrite_slot(lhs, &mut f);
            }
        },
        Instr::Ret { srcs } => {
            if USES {
                rewrite_slots(srcs, &mut f);
            }
        },
        Instr::Abort { code } => {
            if USES {
                rewrite_slot(code, &mut f);
            }
        },
        Instr::AbortMsg { code, msg } => {
            if USES {
                rewrite_slot(code, &mut f);
                rewrite_slot(msg, &mut f);
            }
        },

        // No slot operands.
        Instr::ForceGC => {},
    }
}
