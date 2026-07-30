// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Instruction utilities.
//!
//! Provides read-only slot visitors (`for_each_def`, `for_each_use`,
//! `for_each_slot`, `collect_defs_and_uses`), in-place slot rewriters
//! (`remap_all_slots_with`, `remap_source_slots_with`), the consuming
//! slot-form converter (`try_map_slots`), and miscellaneous instruction
//! helpers (`call_boundary_rets_and_args`, `is_commutative`). Everything is
//! generic over the slot form `SlotForm` of `Instr<SlotForm>`.
//!
//! # Architecture
//!
//! All slot traversal is built on three core functions (`visit_slots` for
//! reading, `rewrite_instr_slots` for in-place mutation, `try_map_slots` for
//! consuming form conversion) so that adding a new `Instr` variant
//! requires updating exactly three match blocks. Public functions are thin
//! wrappers that select const-generic parameters.
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

use super::{BinaryOp, CallClosureData, CallData, FieldPath, Instr, PackClosureData};
use mono_move_core::types::InternedType;
use smallvec::SmallVec;

/// Most instructions have at most 4 defs or uses.
pub(crate) type SlotList<SlotForm> = SmallVec<[SlotForm; 4]>;

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
pub(crate) fn for_each_def<SlotForm: Copy>(instr: &Instr<SlotForm>, mut f: impl FnMut(SlotForm)) {
    visit_slots::<true, false, SlotForm>(instr, |slot, _| f(slot));
}

/// Apply `f` to each slot used (read) by an instruction. Includes
/// both value uses and place uses — the full union of
/// read-side operands.
pub(crate) fn for_each_use<SlotForm: Copy>(instr: &Instr<SlotForm>, mut f: impl FnMut(SlotForm)) {
    visit_slots::<false, true, SlotForm>(instr, |slot, _| f(slot));
}

/// Apply `f` to each slot whose value an instruction consumes,
/// skipping place uses.
pub(crate) fn for_each_value_use<SlotForm: Copy>(
    instr: &Instr<SlotForm>,
    mut f: impl FnMut(SlotForm),
) {
    visit_slots::<false, true, SlotForm>(instr, |slot, role| {
        if role == SlotRole::ValueUse {
            f(slot);
        }
    });
}

/// Apply `f` to every slot (defs and uses) in an instruction.
pub(crate) fn for_each_slot<SlotForm: Copy>(instr: &Instr<SlotForm>, mut f: impl FnMut(SlotForm)) {
    visit_slots::<true, true, SlotForm>(instr, |slot, _| f(slot));
}

/// Collect defs and uses into separate lists in a single pass.
/// Place uses are grouped with value uses.
pub(crate) fn collect_defs_and_uses<SlotForm: Copy>(
    instr: &Instr<SlotForm>,
) -> (SlotList<SlotForm>, SlotList<SlotForm>) {
    let mut defs = SlotList::new();
    let mut uses = SlotList::new();
    visit_slots::<true, true, SlotForm>(instr, |slot, role| match role {
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
pub(crate) fn remap_all_slots_with<SlotForm: Copy>(
    instr: &mut Instr<SlotForm>,
    f: impl FnMut(SlotForm) -> SlotForm,
) {
    rewrite_instr_slots::<true, true, false, SlotForm>(instr, f);
}

/// Rewrite source (use) operands of an instruction by applying `f`,
/// skipping defs and BorrowLoc sources.
///
/// Each slot is rewritten exactly once — `f` is not applied transitively.
pub(crate) fn remap_source_slots_with<SlotForm: Copy>(
    instr: &mut Instr<SlotForm>,
    f: impl FnMut(SlotForm) -> SlotForm,
) {
    rewrite_instr_slots::<false, true, true, SlotForm>(instr, f);
}

// =============================================================================
// Other instruction utilities
// =============================================================================

/// `(rets, args)` of a call-boundary instruction (`Call`, `CallClosure`),
/// `None` for everything else.
#[inline]
pub(crate) fn call_boundary_rets_and_args<SlotForm>(
    instr: &Instr<SlotForm>,
) -> Option<(&[SlotForm], &[SlotForm])> {
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

/// Call-like instructions (`Call`, `CallClosure`) that clobber Transfer
/// slots.
#[inline]
pub(crate) fn clobbers_transfer<SlotForm>(instr: &Instr<SlotForm>) -> bool {
    call_boundary_rets_and_args(instr).is_some()
}

/// The local whose storage `instr` mutably borrows, if any. A later write
/// through that borrow mutates the local without a def at the write site (a
/// hidden write), so coalescing and copy propagation both derive their
/// guards from this single predicate.
pub(crate) fn mut_local_borrow_target<SlotForm: Copy>(instr: &Instr<SlotForm>) -> Option<SlotForm> {
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
pub(crate) fn resource_type_in_instr<SlotForm>(instr: &Instr<SlotForm>) -> Option<InternedType> {
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
pub(crate) fn field_layout_nominal_in_instr<SlotForm>(
    instr: &Instr<SlotForm>,
) -> Option<(InternedType, NominalKind)> {
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
pub(crate) fn chain_field_path<SlotForm>(instr: &Instr<SlotForm>) -> Option<&FieldPath> {
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
pub(crate) fn is_fallthrough_terminator<SlotForm>(instr: &Instr<SlotForm>) -> bool {
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
fn def<const ACTIVE: bool, SlotForm>(slot: SlotForm, f: &mut impl FnMut(SlotForm, SlotRole)) {
    if ACTIVE {
        f(slot, SlotRole::Def);
    }
}

/// Emit a use slot if `ACTIVE` is true.
#[inline]
fn used<const ACTIVE: bool, SlotForm>(slot: SlotForm, f: &mut impl FnMut(SlotForm, SlotRole)) {
    if ACTIVE {
        f(slot, SlotRole::ValueUse);
    }
}

/// Emit a place use slot if `ACTIVE` is true. The slot's
/// identity matters but its bytes are NOT consumed by the instruction.
#[inline]
fn storage_use<const ACTIVE: bool, SlotForm>(
    slot: SlotForm,
    f: &mut impl FnMut(SlotForm, SlotRole),
) {
    if ACTIVE {
        f(slot, SlotRole::PlaceUse);
    }
}

/// Emit each slot in a slice as defs if `ACTIVE` is true.
#[inline]
fn defs<const ACTIVE: bool, SlotForm: Copy>(
    slots: &[SlotForm],
    f: &mut impl FnMut(SlotForm, SlotRole),
) {
    if ACTIVE {
        slots.iter().for_each(|slot| f(*slot, SlotRole::Def));
    }
}

/// Emit each slot in a slice as uses if `ACTIVE` is true.
#[inline]
fn uses<const ACTIVE: bool, SlotForm: Copy>(
    slots: &[SlotForm],
    f: &mut impl FnMut(SlotForm, SlotRole),
) {
    if ACTIVE {
        slots.iter().for_each(|slot| f(*slot, SlotRole::ValueUse));
    }
}

/// Visit slots of an instruction, calling `f(slot, role)` for each.
///
/// `DEFS`/`USES` select which slots to visit. The `def`/`used`/`defs`/`uses`
/// helpers pair the role tag with the const generic by convention.
fn visit_slots<const DEFS: bool, const USES: bool, SlotForm: Copy>(
    instr: &Instr<SlotForm>,
    mut f: impl FnMut(SlotForm, SlotRole),
) {
    match instr {
        Instr::LdConst { dst, .. } | Instr::LdImm { dst, .. } => def::<DEFS, _>(*dst, &mut f),

        Instr::Copy { dst, src } | Instr::Move { dst, src } | Instr::UnaryOp { dst, src, .. } => {
            def::<DEFS, _>(*dst, &mut f);
            used::<USES, _>(*src, &mut f);
        },
        Instr::BinaryOp { dst, lhs, rhs, .. } => {
            def::<DEFS, _>(*dst, &mut f);
            used::<USES, _>(*lhs, &mut f);
            used::<USES, _>(*rhs, &mut f);
        },
        Instr::BinaryOpImm { dst, lhs, .. } => {
            def::<DEFS, _>(*dst, &mut f);
            used::<USES, _>(*lhs, &mut f);
        },

        Instr::Pack { dst, srcs, .. } | Instr::PackVariant { dst, srcs, .. } => {
            def::<DEFS, _>(*dst, &mut f);
            uses::<USES, _>(srcs, &mut f);
        },
        Instr::Unpack { dsts, src, .. } | Instr::UnpackVariant { dsts, src, .. } => {
            defs::<DEFS, _>(dsts, &mut f);
            used::<USES, _>(*src, &mut f);
        },
        Instr::TestVariant { dst, src, .. } => {
            def::<DEFS, _>(*dst, &mut f);
            used::<USES, _>(*src, &mut f);
        },

        // `local` is a place use: the local's identity is
        // taken, its bytes are not consumed.
        Instr::ImmBorrowLoc { dst, local } | Instr::MutBorrowLoc { dst, local } => {
            def::<DEFS, _>(*dst, &mut f);
            storage_use::<USES, _>(*local, &mut f);
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
            def::<DEFS, _>(*dst, &mut f);
            used::<USES, _>(*src, &mut f);
        },
        Instr::WriteRef { dst_ref, val } => {
            used::<USES, _>(*dst_ref, &mut f);
            used::<USES, _>(*val, &mut f);
        },

        Instr::ReadField { dst, src, .. }
        | Instr::ReadVariantField { dst, src, .. }
        | Instr::ReadFieldChain { dst, src, .. } => {
            def::<DEFS, _>(*dst, &mut f);
            used::<USES, _>(*src, &mut f);
        },
        Instr::WriteField { dst_ref, val, .. }
        | Instr::WriteVariantField { dst_ref, val, .. }
        | Instr::WriteFieldChain { dst_ref, val, .. } => {
            used::<USES, _>(*dst_ref, &mut f);
            used::<USES, _>(*val, &mut f);
        },

        // `local` names the inline-struct frame slot, not a reference:
        // a place use.
        Instr::ImmBorrowLocField { dst, local, .. }
        | Instr::MutBorrowLocField { dst, local, .. }
        | Instr::ReadLocField { dst, local, .. }
        | Instr::ImmBorrowLocFieldChain { dst, local, .. }
        | Instr::MutBorrowLocFieldChain { dst, local, .. }
        | Instr::ReadLocFieldChain { dst, local, .. } => {
            def::<DEFS, _>(*dst, &mut f);
            storage_use::<USES, _>(*local, &mut f);
        },
        // `local` is both a def (a field is written in-place) and a
        // place use (the other fields persist, so the slot
        // stays live with the same type after the write).
        Instr::WriteLocField { local, val, .. } | Instr::WriteLocFieldChain { local, val, .. } => {
            def::<DEFS, _>(*local, &mut f);
            storage_use::<USES, _>(*local, &mut f);
            used::<USES, _>(*val, &mut f);
        },

        Instr::Exists { dst, addr, .. } | Instr::MoveFrom { dst, addr, .. } => {
            def::<DEFS, _>(*dst, &mut f);
            used::<USES, _>(*addr, &mut f);
        },
        Instr::MoveTo { signer, val, .. } => {
            used::<USES, _>(*signer, &mut f);
            used::<USES, _>(*val, &mut f);
        },
        Instr::ImmBorrowGlobal { dst, addr, .. } | Instr::MutBorrowGlobal { dst, addr, .. } => {
            def::<DEFS, _>(*dst, &mut f);
            used::<USES, _>(*addr, &mut f);
        },

        Instr::Call { data } => {
            defs::<DEFS, _>(&data.rets, &mut f);
            uses::<USES, _>(&data.args, &mut f);
        },
        Instr::CallClosure { data } => {
            defs::<DEFS, _>(&data.rets, &mut f);
            uses::<USES, _>(&data.args, &mut f);
        },
        Instr::PackClosure { data } => {
            def::<DEFS, _>(data.dst, &mut f);
            uses::<USES, _>(&data.captured, &mut f);
        },

        Instr::VecPack { dst, srcs, .. } => {
            def::<DEFS, _>(*dst, &mut f);
            uses::<USES, _>(srcs, &mut f);
        },
        Instr::VecLen { dst, vec_ref, .. } | Instr::VecPopBack { dst, vec_ref, .. } => {
            def::<DEFS, _>(*dst, &mut f);
            used::<USES, _>(*vec_ref, &mut f);
        },
        Instr::VecImmBorrow {
            dst, vec_ref, idx, ..
        }
        | Instr::VecMutBorrow {
            dst, vec_ref, idx, ..
        } => {
            def::<DEFS, _>(*dst, &mut f);
            used::<USES, _>(*vec_ref, &mut f);
            used::<USES, _>(*idx, &mut f);
        },
        Instr::VecPushBack { vec_ref, val, .. } => {
            used::<USES, _>(*vec_ref, &mut f);
            used::<USES, _>(*val, &mut f);
        },
        Instr::VecUnpack { dsts, src, .. } => {
            defs::<DEFS, _>(dsts, &mut f);
            used::<USES, _>(*src, &mut f);
        },
        Instr::VecSwap {
            vec_ref,
            idx_a,
            idx_b,
            ..
        } => {
            used::<USES, _>(*vec_ref, &mut f);
            used::<USES, _>(*idx_a, &mut f);
            used::<USES, _>(*idx_b, &mut f);
        },

        Instr::Branch { .. } => {},
        Instr::BrTrue { cond, .. } | Instr::BrFalse { cond, .. } => used::<USES, _>(*cond, &mut f),
        Instr::BrCmp { lhs, rhs, .. } => {
            used::<USES, _>(*lhs, &mut f);
            used::<USES, _>(*rhs, &mut f);
        },
        Instr::BrCmpImm { lhs, .. } => used::<USES, _>(*lhs, &mut f),
        Instr::Ret { srcs } => uses::<USES, _>(srcs, &mut f),
        Instr::Abort { code } => used::<USES, _>(*code, &mut f),
        Instr::AbortMsg { code, msg } => {
            used::<USES, _>(*code, &mut f);
            used::<USES, _>(*msg, &mut f);
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
fn rewrite_slot<SlotForm: Copy>(slot: &mut SlotForm, f: &mut impl FnMut(SlotForm) -> SlotForm) {
    *slot = f(*slot);
}

/// Rewrite each slot in a slice by applying `f`.
#[inline]
fn rewrite_slots<SlotForm: Copy>(slots: &mut [SlotForm], f: &mut impl FnMut(SlotForm) -> SlotForm) {
    for slot in slots.iter_mut() {
        rewrite_slot(slot, f);
    }
}

/// Rewrite slot operands of an instruction in-place.
///
/// - `DEFS` / `USES`: select which slots to rewrite (compile-time).
/// - `SKIP_PLACE_USE`: when true, place uses are not
///   rewritten.
fn rewrite_instr_slots<
    const DEFS: bool,
    const USES: bool,
    const SKIP_PLACE_USE: bool,
    SlotForm: Copy,
>(
    instr: &mut Instr<SlotForm>,
    mut f: impl FnMut(SlotForm) -> SlotForm,
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

// =============================================================================
// Consuming slot-form conversion
// =============================================================================

/// Map a boxed slot slice through `f`, preserving order.
fn try_map_slot_box<FromForm, ToForm, E>(
    slots: Box<[FromForm]>,
    f: &mut impl FnMut(FromForm) -> Result<ToForm, E>,
) -> Result<Box<[ToForm]>, E> {
    slots.into_vec().into_iter().map(f).collect()
}

/// Consume `instr`, converting every slot operand from form `FromForm` to
/// form `ToForm` through `f`. The sole conversion path between slot forms;
/// the output type guarantees no operand escapes unconverted.
pub(crate) fn try_map_slots<FromForm, ToForm, E>(
    instr: Instr<FromForm>,
    mut f: impl FnMut(FromForm) -> Result<ToForm, E>,
) -> Result<Instr<ToForm>, E> {
    Ok(match instr {
        Instr::LdConst { dst, const_idx } => Instr::LdConst {
            dst: f(dst)?,
            const_idx,
        },
        Instr::LdImm { dst, imm } => Instr::LdImm { dst: f(dst)?, imm },

        Instr::Copy { dst, src } => Instr::Copy {
            dst: f(dst)?,
            src: f(src)?,
        },
        Instr::Move { dst, src } => Instr::Move {
            dst: f(dst)?,
            src: f(src)?,
        },

        Instr::UnaryOp { dst, op, src } => Instr::UnaryOp {
            dst: f(dst)?,
            op,
            src: f(src)?,
        },
        Instr::BinaryOp { dst, op, lhs, rhs } => Instr::BinaryOp {
            dst: f(dst)?,
            op,
            lhs: f(lhs)?,
            rhs: f(rhs)?,
        },
        Instr::BinaryOpImm { dst, op, lhs, imm } => Instr::BinaryOpImm {
            dst: f(dst)?,
            op,
            lhs: f(lhs)?,
            imm,
        },

        Instr::Pack {
            dst,
            struct_ty,
            srcs,
        } => Instr::Pack {
            dst: f(dst)?,
            struct_ty,
            srcs: try_map_slot_box(srcs, &mut f)?,
        },
        Instr::Unpack {
            dsts,
            struct_ty,
            src,
        } => Instr::Unpack {
            dsts: try_map_slot_box(dsts, &mut f)?,
            struct_ty,
            src: f(src)?,
        },

        Instr::PackVariant {
            dst,
            enum_ty,
            variant,
            srcs,
        } => Instr::PackVariant {
            dst: f(dst)?,
            enum_ty,
            variant,
            srcs: try_map_slot_box(srcs, &mut f)?,
        },
        Instr::UnpackVariant {
            dsts,
            enum_ty,
            variant,
            src,
        } => Instr::UnpackVariant {
            dsts: try_map_slot_box(dsts, &mut f)?,
            enum_ty,
            variant,
            src: f(src)?,
        },
        Instr::TestVariant {
            dst,
            enum_ty,
            variant,
            src,
        } => Instr::TestVariant {
            dst: f(dst)?,
            enum_ty,
            variant,
            src: f(src)?,
        },

        Instr::ImmBorrowLoc { dst, local } => Instr::ImmBorrowLoc {
            dst: f(dst)?,
            local: f(local)?,
        },
        Instr::MutBorrowLoc { dst, local } => Instr::MutBorrowLoc {
            dst: f(dst)?,
            local: f(local)?,
        },
        Instr::ImmBorrowField {
            dst,
            owner_ty,
            field,
            src,
        } => Instr::ImmBorrowField {
            dst: f(dst)?,
            owner_ty,
            field,
            src: f(src)?,
        },
        Instr::MutBorrowField {
            dst,
            owner_ty,
            field,
            src,
        } => Instr::MutBorrowField {
            dst: f(dst)?,
            owner_ty,
            field,
            src: f(src)?,
        },
        Instr::ImmBorrowVariantField {
            dst,
            owner_ty,
            field,
            src,
        } => Instr::ImmBorrowVariantField {
            dst: f(dst)?,
            owner_ty,
            field,
            src: f(src)?,
        },
        Instr::MutBorrowVariantField {
            dst,
            owner_ty,
            field,
            src,
        } => Instr::MutBorrowVariantField {
            dst: f(dst)?,
            owner_ty,
            field,
            src: f(src)?,
        },
        Instr::ReadRef { dst, src } => Instr::ReadRef {
            dst: f(dst)?,
            src: f(src)?,
        },
        Instr::WriteRef { dst_ref, val } => Instr::WriteRef {
            dst_ref: f(dst_ref)?,
            val: f(val)?,
        },

        Instr::ReadField {
            dst,
            owner_ty,
            field,
            src,
        } => Instr::ReadField {
            dst: f(dst)?,
            owner_ty,
            field,
            src: f(src)?,
        },
        Instr::WriteField {
            dst_ref,
            owner_ty,
            field,
            val,
        } => Instr::WriteField {
            dst_ref: f(dst_ref)?,
            owner_ty,
            field,
            val: f(val)?,
        },
        Instr::ReadVariantField {
            dst,
            owner_ty,
            field,
            src,
        } => Instr::ReadVariantField {
            dst: f(dst)?,
            owner_ty,
            field,
            src: f(src)?,
        },
        Instr::WriteVariantField {
            dst_ref,
            owner_ty,
            field,
            val,
        } => Instr::WriteVariantField {
            dst_ref: f(dst_ref)?,
            owner_ty,
            field,
            val: f(val)?,
        },

        Instr::ImmBorrowLocField {
            dst,
            owner_ty,
            field,
            local,
        } => Instr::ImmBorrowLocField {
            dst: f(dst)?,
            owner_ty,
            field,
            local: f(local)?,
        },
        Instr::MutBorrowLocField {
            dst,
            owner_ty,
            field,
            local,
        } => Instr::MutBorrowLocField {
            dst: f(dst)?,
            owner_ty,
            field,
            local: f(local)?,
        },
        Instr::ReadLocField {
            dst,
            owner_ty,
            field,
            local,
        } => Instr::ReadLocField {
            dst: f(dst)?,
            owner_ty,
            field,
            local: f(local)?,
        },
        Instr::WriteLocField {
            local,
            owner_ty,
            field,
            val,
        } => Instr::WriteLocField {
            local: f(local)?,
            owner_ty,
            field,
            val: f(val)?,
        },

        Instr::ReadFieldChain { dst, path, src } => Instr::ReadFieldChain {
            dst: f(dst)?,
            path,
            src: f(src)?,
        },
        Instr::WriteFieldChain { dst_ref, path, val } => Instr::WriteFieldChain {
            dst_ref: f(dst_ref)?,
            path,
            val: f(val)?,
        },
        Instr::ImmBorrowFieldChain { dst, path, src } => Instr::ImmBorrowFieldChain {
            dst: f(dst)?,
            path,
            src: f(src)?,
        },
        Instr::MutBorrowFieldChain { dst, path, src } => Instr::MutBorrowFieldChain {
            dst: f(dst)?,
            path,
            src: f(src)?,
        },
        Instr::ReadLocFieldChain { dst, path, local } => Instr::ReadLocFieldChain {
            dst: f(dst)?,
            path,
            local: f(local)?,
        },
        Instr::WriteLocFieldChain { local, path, val } => Instr::WriteLocFieldChain {
            local: f(local)?,
            path,
            val: f(val)?,
        },
        Instr::ImmBorrowLocFieldChain { dst, path, local } => Instr::ImmBorrowLocFieldChain {
            dst: f(dst)?,
            path,
            local: f(local)?,
        },
        Instr::MutBorrowLocFieldChain { dst, path, local } => Instr::MutBorrowLocFieldChain {
            dst: f(dst)?,
            path,
            local: f(local)?,
        },

        Instr::Exists {
            dst,
            resource_ty,
            addr,
        } => Instr::Exists {
            dst: f(dst)?,
            resource_ty,
            addr: f(addr)?,
        },
        Instr::MoveFrom {
            dst,
            resource_ty,
            addr,
        } => Instr::MoveFrom {
            dst: f(dst)?,
            resource_ty,
            addr: f(addr)?,
        },
        Instr::MoveTo {
            resource_ty,
            signer,
            val,
        } => Instr::MoveTo {
            resource_ty,
            signer: f(signer)?,
            val: f(val)?,
        },
        Instr::ImmBorrowGlobal {
            dst,
            resource_ty,
            addr,
        } => Instr::ImmBorrowGlobal {
            dst: f(dst)?,
            resource_ty,
            addr: f(addr)?,
        },
        Instr::MutBorrowGlobal {
            dst,
            resource_ty,
            addr,
        } => Instr::MutBorrowGlobal {
            dst: f(dst)?,
            resource_ty,
            addr: f(addr)?,
        },

        Instr::Call { data } => {
            let CallData {
                rets,
                function_handle,
                ty_args,
                args,
            } = *data;
            Instr::Call {
                data: Box::new(CallData {
                    rets: try_map_slot_box(rets, &mut f)?,
                    function_handle,
                    ty_args,
                    args: try_map_slot_box(args, &mut f)?,
                }),
            }
        },
        Instr::PackClosure { data } => {
            let PackClosureData {
                dst,
                function_handle,
                ty_args,
                mask,
                captured,
            } = *data;
            Instr::PackClosure {
                data: Box::new(PackClosureData {
                    dst: f(dst)?,
                    function_handle,
                    ty_args,
                    mask,
                    captured: try_map_slot_box(captured, &mut f)?,
                }),
            }
        },
        Instr::CallClosure { data } => {
            let CallClosureData {
                rets,
                closure_ty,
                args,
            } = *data;
            Instr::CallClosure {
                data: Box::new(CallClosureData::new(
                    try_map_slot_box(rets, &mut f)?,
                    closure_ty,
                    try_map_slot_box(args, &mut f)?,
                )),
            }
        },

        Instr::VecPack { dst, elem_ty, srcs } => Instr::VecPack {
            dst: f(dst)?,
            elem_ty,
            srcs: try_map_slot_box(srcs, &mut f)?,
        },
        Instr::VecLen {
            dst,
            elem_ty,
            vec_ref,
        } => Instr::VecLen {
            dst: f(dst)?,
            elem_ty,
            vec_ref: f(vec_ref)?,
        },
        Instr::VecImmBorrow {
            dst,
            elem_ty,
            vec_ref,
            idx,
        } => Instr::VecImmBorrow {
            dst: f(dst)?,
            elem_ty,
            vec_ref: f(vec_ref)?,
            idx: f(idx)?,
        },
        Instr::VecMutBorrow {
            dst,
            elem_ty,
            vec_ref,
            idx,
        } => Instr::VecMutBorrow {
            dst: f(dst)?,
            elem_ty,
            vec_ref: f(vec_ref)?,
            idx: f(idx)?,
        },
        Instr::VecPushBack {
            vec_ref,
            elem_ty,
            val,
        } => Instr::VecPushBack {
            vec_ref: f(vec_ref)?,
            elem_ty,
            val: f(val)?,
        },
        Instr::VecPopBack {
            dst,
            elem_ty,
            vec_ref,
        } => Instr::VecPopBack {
            dst: f(dst)?,
            elem_ty,
            vec_ref: f(vec_ref)?,
        },
        Instr::VecUnpack { dsts, elem_ty, src } => Instr::VecUnpack {
            dsts: try_map_slot_box(dsts, &mut f)?,
            elem_ty,
            src: f(src)?,
        },
        Instr::VecSwap {
            vec_ref,
            elem_ty,
            idx_a,
            idx_b,
        } => Instr::VecSwap {
            vec_ref: f(vec_ref)?,
            elem_ty,
            idx_a: f(idx_a)?,
            idx_b: f(idx_b)?,
        },

        Instr::Branch { target } => Instr::Branch { target },
        Instr::BrTrue { target, cond } => Instr::BrTrue {
            target,
            cond: f(cond)?,
        },
        Instr::BrFalse { target, cond } => Instr::BrFalse {
            target,
            cond: f(cond)?,
        },
        Instr::BrCmp {
            target,
            op,
            lhs,
            rhs,
        } => Instr::BrCmp {
            target,
            op,
            lhs: f(lhs)?,
            rhs: f(rhs)?,
        },
        Instr::BrCmpImm {
            target,
            op,
            lhs,
            imm,
        } => Instr::BrCmpImm {
            target,
            op,
            lhs: f(lhs)?,
            imm,
        },
        Instr::Ret { srcs } => Instr::Ret {
            srcs: try_map_slot_box(srcs, &mut f)?,
        },
        Instr::Abort { code } => Instr::Abort { code: f(code)? },
        Instr::AbortMsg { code, msg } => Instr::AbortMsg {
            code: f(code)?,
            msg: f(msg)?,
        },

        Instr::ForceGC => Instr::ForceGC,
    })
}
