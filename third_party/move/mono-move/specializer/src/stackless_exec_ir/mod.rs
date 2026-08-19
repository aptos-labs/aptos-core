// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Data types for the stackless execution IR.
//!
//! This IR converts Move's stack-based bytecode into explicit named-slot form,
//! eliminating the operand stack and allowing direct named-slot operands on each instruction.

mod display;
mod instr_seq;
pub(crate) mod instr_utils;

use crate::{gas::BlockCost, validate::TranslationWitness};
pub use instr_seq::InstrSeq;
pub use mono_move_core::CmpKind;
use mono_move_core::{
    types::{InternedType, InternedTypeList},
    IntTy, PreparedModule,
};
use move_binary_format::file_format::{
    ConstantPoolIndex, FieldHandleIndex, FunctionDefinitionIndex, FunctionHandleIndex,
    IdentifierIndex, VariantFieldHandleIndex,
};
use move_core_types::{
    function::ClosureMask,
    int256::{I256, U256},
};

/// Index of a `Home` slot within the frame's local area. Shared by
/// [`SsaSlot::Home`] and [`NamedSlot::Home`]: slot allocation maps pinned
/// locals across the two forms index-preservingly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HomeIndex(pub u16);

/// Position within the frame's transfer area.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransferPosition(pub u16);

/// Instruction operand used while a function is in SSA form, before slot
/// allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SsaSlot {
    /// Frame-local storage for parameters and declared locals. These map
    /// 1:1 to frame slots and stay mutable across blocks. Displayed as
    /// `r0, r1, ...`.
    Home(HomeIndex),
    /// SSA value ID, a temporary defined exactly once within its block.
    /// Displayed as `v0, v1, ...`.
    ValueId(u16),
}

impl SsaSlot {
    /// Returns `true` if this is a `ValueId` slot (SSA value ID).
    pub fn is_value_id(self) -> bool {
        matches!(self, SsaSlot::ValueId(_))
    }
}

/// Instruction operand after slot allocation: a concrete named position in
/// the runtime frame. Contrast with the lowering layer's `SizedSlot`, which
/// adds layout (offset and size) to the name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NamedSlot {
    /// Frame-local storage: parameters, declared locals, and temporaries
    /// due to destackification. Displayed as `r0, r1, ...`.
    Home(HomeIndex),
    /// Call-interface slots, used for both passing arguments to a callee
    /// (before the call) and receiving return values (after the call).
    /// `Transfer` overlaps with the callee's parameter/return area, so
    /// values produced directly into a `Transfer` slot avoid a copy at the
    /// call site. Displayed as `x0, x1, ...`.
    Transfer(TransferPosition),
}

/// Label for branch targets.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Label(pub u16);

/// Unary operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    /// Cast the operand to the given integer type, aborting if it doesn't fit.
    Cast(IntTy),
    Not,
    Negate,
    FreezeRef,
}

/// Binary operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitOr,
    BitAnd,
    BitXor,
    Shl,
    Shr,
    Cmp(CmpKind),
    Or,
    And,
}

/// Immediate values used by `Instr`. Wide widths (u128 / U256 / i128 / I256)
/// are boxed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImmValue {
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(Box<u128>),
    U256(Box<U256>),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(Box<i128>),
    I256(Box<I256>),
}

// Wide variants box their payload to keep the enum at 16 bytes regardless
// of the largest integer type.
const _: () = assert!(std::mem::size_of::<ImmValue>() == 16);

/// A chain of inline-struct field selections, each an `(instantiated owner
/// type, field handle)` pair.
pub type FieldPath = Box<[(InternedType, FieldHandleIndex)]>;

/// Payload of [`Instr::Call`].
///
/// `function_handle` gives the callee identity; `ty_args` is the
/// instantiation's type arguments, and is `EMPTY_TYPE_LIST` for a
/// non-generic call. Inside a generic function `ty_args` may still
/// contain the enclosing function's `TypeParam`s.
#[derive(Clone)]
pub struct CallData<SlotForm> {
    pub rets: Box<[SlotForm]>,
    pub function_handle: FunctionHandleIndex,
    pub ty_args: InternedTypeList,
    pub args: Box<[SlotForm]>,
}

/// Payload of [`Instr::PackClosure`] (same `(function_handle, ty_args)`
/// contract as [`CallData`]).
#[derive(Clone)]
pub struct PackClosureData<SlotForm> {
    pub dst: SlotForm,
    pub function_handle: FunctionHandleIndex,
    pub ty_args: InternedTypeList,
    pub mask: ClosureMask,
    pub captured: Box<[SlotForm]>,
}

/// Payload of [`Instr::CallClosure`].
#[derive(Clone)]
pub struct CallClosureData<SlotForm> {
    pub rets: Box<[SlotForm]>,
    /// The closure's type; always a `Type::Function`.
    pub closure_ty: InternedType,
    /// The arguments provided to the closure, followed by the closure
    /// itself as the last element. Never empty; enforced by [`Self::new`].
    args: Box<[SlotForm]>,
}

impl<SlotForm> CallClosureData<SlotForm> {
    /// `args` must end with the closure operand (hence be non-empty).
    pub(crate) fn new(
        rets: Box<[SlotForm]>,
        closure_ty: InternedType,
        args: Box<[SlotForm]>,
    ) -> Self {
        debug_assert!(
            !args.is_empty(),
            "CallClosure args must include the closure"
        );
        Self {
            rets,
            closure_ty,
            args,
        }
    }

    /// Splits `args` into `(closure, provided arguments)`.
    pub fn closure_and_provided(&self) -> (SlotForm, &[SlotForm])
    where
        SlotForm: Copy,
    {
        let (closure, provided) = self
            .args
            .split_last()
            .expect("CallClosure args are non-empty by construction");
        (*closure, provided)
    }
}

/// A stackless IR instruction with explicit slot operands.
///
/// Generic over the slot form `SlotForm`: [`SsaSlot`] before slot allocation,
/// [`NamedSlot`] after. Slot allocation is the only conversion between the
/// two (see `destack::slot_alloc`).
///
/// # Field order
///
/// Each variant declares its fields in three tiers:
/// 1. Destinations: result slots (`dst`/`dsts`), the written-through reference
///    (`dst_ref`; `vec_ref`), the in-place-written `local`, or the branch
///    `target`.
/// 2. Operation parameters: `op`, carried types (`struct_ty`, `enum_ty`,
///    `owner_ty`, `resource_ty`, `elem_ty`), `field`, `variant`, `path`,
///    `const_idx`.
/// 3. Sources, in evaluation order: `src`/`srcs`, `lhs`/`rhs`, `local`, `addr`,
///    `signer`, `val`, `vec_ref`, `idx`/`idx_a`/`idx_b`, `cond`, `code`, `msg`,
///    and a trailing `imm`.
///
/// A role keeps one field name across all variants, so same-typed roles can
/// be bound by or-patterns: `Instr::A { dst, src, .. } | Instr::B { dst, src }`
///
/// # Operand roles
///
/// Every slot operand is written (a def), read (a value use), or referenced
/// by location (a place use):
/// - `dst`/`dsts` are defs.
/// - Sources are value uses: a `ValueId` is consumed (single-use SSA before
///   slot allocation); a `Home` slot stays valid unless moved out. A
///   written-through reference is also a value use — its pointee is
///   mutated, but the slot is not a def.
/// - `local` is a place use: only the frame slot's location is taken; its
///   bytes are unread and it stays live with the same type. In `WriteLoc*`,
///   `local` is both a def and a place use (one field written, the rest
///   persist).
///
/// # Type contract
///
/// `struct_ty`, `enum_ty`, `owner_ty`, `resource_ty`, and the owners in a
/// `path` are instantiated nominals (type arguments already applied);
/// `elem_ty` and `closure_ty` are arbitrary element/function types. Inside
/// a generic function, carried types may still contain the enclosing
/// function's `TypeParam`s.
///
/// # Control flow
///
/// `Branch`, `Ret`, `Abort`, and `AbortMsg` terminate a block; `BrTrue`,
/// `BrFalse`, `BrCmp`, and `BrCmpImm` fall through when not taken. `Call`
/// and `CallClosure` clobber all `Transfer` slots.
///
/// TODO(cleanup): change uses of raw integers into newtypes/type-aliases.
#[derive(Clone)]
pub enum Instr<SlotForm> {
    // --- Loads ---
    /// `dst = const_pool[const_idx]` — load a constant, deserialized from
    /// its BCS byte blob in the constant pool.
    LdConst {
        dst: SlotForm,
        const_idx: ConstantPoolIndex,
    },
    /// `dst = imm` — load a bool or integer literal.
    LdImm { dst: SlotForm, imm: ImmValue },

    // --- Slot ops ---
    //
    // For transformation passes: when may `dst` and `src` be treated as
    // interchangeable names for one value?
    //
    // - `Move`: always — both name the same object, and `src` is unusable
    //   until redefined (bytecode verifier).
    // - `Copy`: only for bitwise-copy types
    //   (`PreparedModule::is_bitwise_copy_type`). Otherwise the lowering
    //   deep-copies, so `dst` is a different object: the `Copy` must not be
    //   substituted through, coalesced, or elided unless its result is
    //   provably unobserved.
    //
    // Either claim holds only until `dst` or `src` is redefined (frame slots
    // are recycled); invalidate before rewriting the redefining instruction
    // itself.
    /// `dst = copy(src)`, source remains valid.
    Copy { dst: SlotForm, src: SlotForm },
    /// `dst = move(src)`, source invalidated.
    Move { dst: SlotForm, src: SlotForm },

    // --- Unary / Binary ---
    /// `dst = op(src)`. `Cast` aborts when the value does not fit the
    /// target integer type.
    UnaryOp {
        dst: SlotForm,
        op: UnaryOp,
        src: SlotForm,
    },
    /// `dst = op(lhs, rhs)`. Arithmetic aborts on overflow and on
    /// division/modulo by zero; shifts abort when the amount is at least
    /// the operand width.
    BinaryOp {
        dst: SlotForm,
        op: BinaryOp,
        lhs: SlotForm,
        rhs: SlotForm,
    },
    /// `dst = op(lhs, imm)` — [`Instr::BinaryOp`] with an immediate right
    /// operand.
    BinaryOpImm {
        dst: SlotForm,
        op: BinaryOp,
        lhs: SlotForm,
        imm: ImmValue,
    },

    // --- Struct ---
    /// `dst = struct_ty { srcs }` — construct a struct from the field
    /// values, in field declaration order.
    Pack {
        dst: SlotForm,
        struct_ty: InternedType,
        srcs: Box<[SlotForm]>,
    },
    /// `dsts = fields of src` — destructure a `struct_ty` value into its
    /// field values, in field declaration order.
    Unpack {
        dsts: Box<[SlotForm]>,
        struct_ty: InternedType,
        src: SlotForm,
    },

    // --- Variant (enum type + variant ordinal) ---
    /// `dst = enum_ty::variant { srcs }` — construct an enum value with tag
    /// `variant` from the field values, in field declaration order.
    PackVariant {
        dst: SlotForm,
        enum_ty: InternedType,
        variant: u16,
        srcs: Box<[SlotForm]>,
    },
    /// `dsts = fields of src` — destructure an `enum_ty` value; aborts when
    /// the value's runtime tag is not `variant`.
    UnpackVariant {
        dsts: Box<[SlotForm]>,
        enum_ty: InternedType,
        variant: u16,
        src: SlotForm,
    },
    /// `dst = (tag(*src) == variant)` — `src` holds a reference to an
    /// `enum_ty` value.
    TestVariant {
        dst: SlotForm,
        enum_ty: InternedType,
        variant: u16,
        src: SlotForm,
    },

    // --- References ---
    //
    /// `dst = &local`.
    ImmBorrowLoc { dst: SlotForm, local: SlotForm },
    /// `dst = &mut local`. A later write through `dst` mutates `local`
    /// without a def at the write site (a hidden write).
    MutBorrowLoc { dst: SlotForm, local: SlotForm },
    // Field ops carry the instantiated owner type (`owner_ty`) and a
    // non-generic field handle (`field`) giving the field position.
    /// `dst = &(*src).field` — pure address computation, no abort.
    ImmBorrowField {
        dst: SlotForm,
        owner_ty: InternedType,
        field: FieldHandleIndex,
        src: SlotForm,
    },
    /// `dst = &mut (*src).field` — pure address computation, no abort.
    MutBorrowField {
        dst: SlotForm,
        owner_ty: InternedType,
        field: FieldHandleIndex,
        src: SlotForm,
    },
    /// `dst = &(*src).field` on an enum; aborts when the value's runtime
    /// variant does not declare `field`.
    ImmBorrowVariantField {
        dst: SlotForm,
        owner_ty: InternedType,
        field: VariantFieldHandleIndex,
        src: SlotForm,
    },
    /// `dst = &mut (*src).field` on an enum; aborts when the value's
    /// runtime variant does not declare `field`.
    MutBorrowVariantField {
        dst: SlotForm,
        owner_ty: InternedType,
        field: VariantFieldHandleIndex,
        src: SlotForm,
    },
    /// `dst = *src`.
    ReadRef { dst: SlotForm, src: SlotForm },
    /// `*dst_ref = val`
    WriteRef { dst_ref: SlotForm, val: SlotForm },

    // --- Fused field access (borrow+read/write combined) ---
    /// `dst = (*src).field` (imm_borrow_field + read_ref)
    ReadField {
        dst: SlotForm,
        owner_ty: InternedType,
        field: FieldHandleIndex,
        src: SlotForm,
    },
    /// `(*dst_ref).field = val` (mut_borrow_field + write_ref)
    WriteField {
        dst_ref: SlotForm,
        owner_ty: InternedType,
        field: FieldHandleIndex,
        val: SlotForm,
    },
    /// `dst = (*src).field` on an enum (imm_borrow_variant_field +
    /// read_ref); aborts when the value's runtime variant does not declare
    /// `field`.
    ReadVariantField {
        dst: SlotForm,
        owner_ty: InternedType,
        field: VariantFieldHandleIndex,
        src: SlotForm,
    },
    /// `(*dst_ref).field = val` on an enum (mut_borrow_variant_field +
    /// write_ref); aborts when the value's runtime variant does not declare
    /// `field`.
    WriteVariantField {
        dst_ref: SlotForm,
        owner_ty: InternedType,
        field: VariantFieldHandleIndex,
        val: SlotForm,
    },

    // --- Fused inline-struct field access (borrow_loc + field op combined) ---
    /// `dst = &local.field` (imm_borrow_loc + imm_borrow_field on an inline struct local)
    ImmBorrowLocField {
        dst: SlotForm,
        owner_ty: InternedType,
        field: FieldHandleIndex,
        local: SlotForm,
    },
    /// `dst = &mut local.field`. Hidden-write hazard as in
    /// [`Instr::MutBorrowLoc`].
    MutBorrowLocField {
        dst: SlotForm,
        owner_ty: InternedType,
        field: FieldHandleIndex,
        local: SlotForm,
    },
    /// `dst = local.field` (imm_borrow_loc + read_field on an inline struct local)
    ReadLocField {
        dst: SlotForm,
        owner_ty: InternedType,
        field: FieldHandleIndex,
        local: SlotForm,
    },
    /// `local.field = val` (mut_borrow_loc + write_field on an inline
    /// struct local).
    WriteLocField {
        local: SlotForm,
        owner_ty: InternedType,
        field: FieldHandleIndex,
        val: SlotForm,
    },

    // --- Fused inline-struct field CHAINS ---
    //
    // A chain applies the field selections in `path` — `(owner, field)`
    // steps in selection order, always depth >= 2 — to a root operand, and
    // reads, writes, or borrows the field reached last. The root is a
    // reference in the `*FieldChain` forms (`src` when read or borrowed,
    // `dst_ref` when written through) and a by-value inline-struct `local`
    // in the `*LocFieldChain` forms. Every step selects an inline-struct
    // field, so the target's address is `base(root) + Σ offsets` and no
    // step can abort.
    //
    /// `dst = (*src).path`.
    ReadFieldChain {
        dst: SlotForm,
        path: FieldPath,
        src: SlotForm,
    },
    /// `(*dst_ref).path = val`.
    WriteFieldChain {
        dst_ref: SlotForm,
        path: FieldPath,
        val: SlotForm,
    },
    /// `dst = &(*src).path`.
    ImmBorrowFieldChain {
        dst: SlotForm,
        path: FieldPath,
        src: SlotForm,
    },
    /// `dst = &mut (*src).path`.
    MutBorrowFieldChain {
        dst: SlotForm,
        path: FieldPath,
        src: SlotForm,
    },
    /// `dst = local.path`.
    ReadLocFieldChain {
        dst: SlotForm,
        path: FieldPath,
        local: SlotForm,
    },
    /// `local.path = val`.
    WriteLocFieldChain {
        local: SlotForm,
        path: FieldPath,
        val: SlotForm,
    },
    /// `dst = &local.path`.
    ImmBorrowLocFieldChain {
        dst: SlotForm,
        path: FieldPath,
        local: SlotForm,
    },
    /// `dst = &mut local.path`. Hidden-write hazard as in
    /// [`Instr::MutBorrowLoc`].
    MutBorrowLocFieldChain {
        dst: SlotForm,
        path: FieldPath,
        local: SlotForm,
    },

    // --- Globals (`resource_ty` names the resource in global storage) ---
    /// `dst = exists<resource_ty>(addr)` — true iff global storage holds a
    /// `resource_ty` at `addr`.
    Exists {
        dst: SlotForm,
        resource_ty: InternedType,
        addr: SlotForm,
    },
    /// `dst = move_from<resource_ty>(addr)` — remove the resource from
    /// global storage; aborts when none exists at `addr`.
    MoveFrom {
        dst: SlotForm,
        resource_ty: InternedType,
        addr: SlotForm,
    },
    /// `move_to<resource_ty>(signer, val)` — publish `val` under the
    /// signer's address; aborts when a resource of this type already exists
    /// there.
    MoveTo {
        resource_ty: InternedType,
        signer: SlotForm,
        val: SlotForm,
    },
    /// `dst = &global<resource_ty>(addr)`; aborts when no resource exists
    /// at `addr`.
    ImmBorrowGlobal {
        dst: SlotForm,
        resource_ty: InternedType,
        addr: SlotForm,
    },
    /// `dst = &mut global<resource_ty>(addr)`; aborts when no resource
    /// exists at `addr`.
    MutBorrowGlobal {
        dst: SlotForm,
        resource_ty: InternedType,
        addr: SlotForm,
    },

    // --- Calls (payload boxed to keep `Instr` small; see `CallData`) ---
    /// `data.rets = data.function_handle<data.ty_args>(data.args)`.
    Call { data: Box<CallData<SlotForm>> },

    // --- Closures (payloads boxed; see `PackClosureData`/`CallClosureData`) ---
    /// `data.dst` receives a closure over `data.function_handle<data.ty_args>`
    /// in which the values of `data.captured` are pre-bound to the argument
    /// positions marked in `data.mask`; the remaining positions are filled by
    /// the arguments supplied when the closure is called.
    PackClosure {
        data: Box<PackClosureData<SlotForm>>,
    },
    /// `data.rets = closure(provided args)` — the closure operand is the
    /// last element of `data.args` (see
    /// [`CallClosureData::closure_and_provided`]).
    CallClosure {
        data: Box<CallClosureData<SlotForm>>,
    },

    // --- Vector (`elem_ty` is the vector's element type) ---
    /// `dst = vector[srcs]`.
    VecPack {
        dst: SlotForm,
        elem_ty: InternedType,
        srcs: Box<[SlotForm]>,
    },
    /// `dst = (*vec_ref).length()`.
    VecLen {
        dst: SlotForm,
        elem_ty: InternedType,
        vec_ref: SlotForm,
    },
    /// `dst = &(*vec_ref)[idx]`; aborts when `idx` is out of bounds.
    VecImmBorrow {
        dst: SlotForm,
        elem_ty: InternedType,
        vec_ref: SlotForm,
        idx: SlotForm,
    },
    /// `dst = &mut (*vec_ref)[idx]`; aborts when `idx` is out of bounds.
    VecMutBorrow {
        dst: SlotForm,
        elem_ty: InternedType,
        vec_ref: SlotForm,
        idx: SlotForm,
    },
    /// `(*vec_ref).push_back(val)`.
    VecPushBack {
        vec_ref: SlotForm,
        elem_ty: InternedType,
        val: SlotForm,
    },
    /// `dst = (*vec_ref).pop_back()`; aborts on an empty vector.
    VecPopBack {
        dst: SlotForm,
        elem_ty: InternedType,
        vec_ref: SlotForm,
    },
    /// `dsts = elements of src`; aborts unless the vector's length is
    /// exactly `dsts.len()`.
    VecUnpack {
        dsts: Box<[SlotForm]>,
        elem_ty: InternedType,
        src: SlotForm,
    },
    /// `(*vec_ref).swap(idx_a, idx_b)`; aborts when either index is out of
    /// bounds.
    VecSwap {
        vec_ref: SlotForm,
        elem_ty: InternedType,
        idx_a: SlotForm,
        idx_b: SlotForm,
    },

    // --- Control flow ---
    /// Unconditional jump to `target`.
    Branch { target: Label },
    /// Jump to `target` when `cond` is true; otherwise fall through.
    BrTrue { target: Label, cond: SlotForm },
    /// Jump to `target` when `cond` is false; otherwise fall through.
    BrFalse { target: Label, cond: SlotForm },
    /// Jump to `target` when `op(lhs, rhs)` is true; otherwise fall through
    /// (fused compare + conditional branch).
    BrCmp {
        target: Label,
        op: CmpKind,
        lhs: SlotForm,
        rhs: SlotForm,
    },
    /// Jump to `target` when `op(lhs, imm)` is true; otherwise fall
    /// through.
    BrCmpImm {
        target: Label,
        op: CmpKind,
        lhs: SlotForm,
        imm: ImmValue,
    },
    /// Return `srcs` to the caller.
    Ret { srcs: Box<[SlotForm]> },
    /// Abort execution with the error code held in `code`.
    Abort { code: SlotForm },
    /// Abort execution with the error code in `code` and the message
    /// payload in `msg`.
    AbortMsg { code: SlotForm, msg: SlotForm },

    // --- Test intrinsics ---
    /// Triggers a garbage collection.
    ForceGC,
}

// Both slot forms are 4 bytes (tag + u16), so both instantiations share
// the 32-byte pin.
const _: () = assert!(
    std::mem::size_of::<Instr<SsaSlot>>() == 32 && std::mem::size_of::<Instr<NamedSlot>>() == 32,
    "Instr is no longer 32 bytes; if it grew, box the offending variant's \
    payload — if the widest variant shrank, re-pin this constant"
);
// The align assert matters on its own: a raw `u128`-family payload could keep
// the size at 32 while bumping the alignment (and allocator padding) to 16.
const _: () = assert!(
    std::mem::align_of::<Instr<SsaSlot>>() == 8 && std::mem::align_of::<Instr<NamedSlot>>() == 8,
    "Instr alignment is no longer 8; if it grew, box the align-16 \
     (u128-family) payload — if it shrank, re-pin this constant"
);

impl<SlotForm> Instr<SlotForm> {
    /// Returns the variant tag as a static string. Useful for terse error
    /// messages that don't need the full operand dump.
    pub fn opcode_name(&self) -> &'static str {
        match self {
            Instr::LdConst { .. } => "LdConst",
            Instr::LdImm { .. } => "LdImm",
            Instr::Copy { .. } => "Copy",
            Instr::Move { .. } => "Move",
            Instr::UnaryOp { .. } => "UnaryOp",
            Instr::BinaryOp { .. } => "BinaryOp",
            Instr::BinaryOpImm { .. } => "BinaryOpImm",
            Instr::Pack { .. } => "Pack",
            Instr::Unpack { .. } => "Unpack",
            Instr::PackVariant { .. } => "PackVariant",
            Instr::UnpackVariant { .. } => "UnpackVariant",
            Instr::TestVariant { .. } => "TestVariant",
            Instr::ImmBorrowLoc { .. } => "ImmBorrowLoc",
            Instr::MutBorrowLoc { .. } => "MutBorrowLoc",
            Instr::ImmBorrowField { .. } => "ImmBorrowField",
            Instr::MutBorrowField { .. } => "MutBorrowField",
            Instr::ImmBorrowVariantField { .. } => "ImmBorrowVariantField",
            Instr::MutBorrowVariantField { .. } => "MutBorrowVariantField",
            Instr::ReadRef { .. } => "ReadRef",
            Instr::WriteRef { .. } => "WriteRef",
            Instr::ReadField { .. } => "ReadField",
            Instr::WriteField { .. } => "WriteField",
            Instr::ReadVariantField { .. } => "ReadVariantField",
            Instr::WriteVariantField { .. } => "WriteVariantField",
            Instr::ImmBorrowLocField { .. } => "ImmBorrowLocField",
            Instr::MutBorrowLocField { .. } => "MutBorrowLocField",
            Instr::ReadLocField { .. } => "ReadLocField",
            Instr::WriteLocField { .. } => "WriteLocField",
            Instr::ReadFieldChain { .. } => "ReadFieldChain",
            Instr::WriteFieldChain { .. } => "WriteFieldChain",
            Instr::ImmBorrowFieldChain { .. } => "ImmBorrowFieldChain",
            Instr::MutBorrowFieldChain { .. } => "MutBorrowFieldChain",
            Instr::ReadLocFieldChain { .. } => "ReadLocFieldChain",
            Instr::WriteLocFieldChain { .. } => "WriteLocFieldChain",
            Instr::ImmBorrowLocFieldChain { .. } => "ImmBorrowLocFieldChain",
            Instr::MutBorrowLocFieldChain { .. } => "MutBorrowLocFieldChain",
            Instr::Exists { .. } => "Exists",
            Instr::MoveFrom { .. } => "MoveFrom",
            Instr::MoveTo { .. } => "MoveTo",
            Instr::ImmBorrowGlobal { .. } => "ImmBorrowGlobal",
            Instr::MutBorrowGlobal { .. } => "MutBorrowGlobal",
            Instr::Call { .. } => "Call",
            Instr::PackClosure { .. } => "PackClosure",
            Instr::CallClosure { .. } => "CallClosure",
            Instr::VecPack { .. } => "VecPack",
            Instr::VecLen { .. } => "VecLen",
            Instr::VecImmBorrow { .. } => "VecImmBorrow",
            Instr::VecMutBorrow { .. } => "VecMutBorrow",
            Instr::VecPushBack { .. } => "VecPushBack",
            Instr::VecPopBack { .. } => "VecPopBack",
            Instr::VecUnpack { .. } => "VecUnpack",
            Instr::VecSwap { .. } => "VecSwap",
            Instr::Branch { .. } => "Branch",
            Instr::BrTrue { .. } => "BrTrue",
            Instr::BrFalse { .. } => "BrFalse",
            Instr::BrCmp { .. } => "BrCmp",
            Instr::BrCmpImm { .. } => "BrCmpImm",
            Instr::Ret { .. } => "Ret",
            Instr::Abort { .. } => "Abort",
            Instr::AbortMsg { .. } => "AbortMsg",
            Instr::ForceGC => "ForceGC",
        }
    }
}

/// A basic block of instructions, generic over the slot form like
/// [`Instr`].
///
/// Every block has a label. A block ends with a terminator (a branch,
/// return, or abort) OR falls through — possibly with no instructions at
/// all — to the next block in layout order:
///
/// ```text
/// LL1:  op
///       br_true LL3
///       // fallthrough to LL2
/// LL2:  op
///       ...
/// LL3:  op
///       ...
/// ```
///
/// See `instr_utils::classify_exit` for the derived exit classification.
pub struct BasicBlock<SlotForm> {
    /// Label identifying this block.
    pub label: Label,
    /// Instructions in this block, each carrying its originating bytecode
    /// offset.
    pub instrs: InstrSeq<SlotForm>,
}

/// IR for a single function, after slot allocation (named-slot form).
pub struct FunctionIR {
    /// Function name in identifier pool.
    pub name_idx: IdentifierIndex,
    /// Function handle index.
    pub handle_idx: FunctionHandleIndex,
    /// This function's definition index in its module.
    pub def_idx: FunctionDefinitionIndex,
    /// Number of parameters (count, not a slot).
    pub num_params: u16,
    /// Number of non-param locals (count, not a slot).
    pub num_locals: u16,
    /// Total Home slots used (params + locals + temps).
    pub num_home_slots: u16,
    /// Number of distinct `Transfer(j)` positions used across all calls in
    /// this function.
    pub num_transfer_positions: u16,
    /// Basic blocks of the function.
    pub blocks: Vec<BasicBlock<NamedSlot>>,
    /// Type of each Home slot (indexed by Home slot index, 0..num_home_slots-1).
    /// Transfer slots have no entry here — their types are inferred from call
    /// signatures.
    pub home_slot_types: Vec<InternedType>,
    /// Gas cost of each block as an unresolved formula, indexed by block label.
    pub(crate) block_costs: Vec<BlockCost>,
    /// Untrusted witness for translation validation — never read by
    /// execution or lowering. See [`TranslationWitness`].
    pub(crate) witness: TranslationWitness,
}

impl FunctionIR {
    /// Iterate over all instructions across all blocks.
    pub fn instrs(&self) -> impl Iterator<Item = &Instr<NamedSlot>> {
        self.blocks.iter().flat_map(|block| block.instrs.iter())
    }

    /// Iterate mutably over all instructions across all blocks.
    pub fn instrs_mut(&mut self) -> impl Iterator<Item = &mut Instr<NamedSlot>> {
        self.blocks
            .iter_mut()
            .flat_map(|block| block.instrs.iter_mut())
    }
}

/// IR for a module (wraps the original compiled and resolved module for pool
/// access).
pub struct ModuleIR {
    /// The original compiled module with resolved type pools.
    pub module: PreparedModule,
    /// Indexed by `FunctionDefinitionIndex`. `None` for native functions.
    pub functions: Vec<Option<FunctionIR>>,
}
