// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Data types for the stackless execution IR.
//!
//! This IR converts Move's stack-based bytecode into explicit named-slot form,
//! eliminating the operand stack and allowing direct named-slot operands on each instruction.

mod display;
pub(crate) mod instr_utils;

use crate::gas::BlockCost;
pub use mono_move_core::CmpKind;
use mono_move_core::{
    types::{InternedType, InternedTypeList},
    IntTy, PreparedModule,
};
use move_binary_format::file_format::{
    ConstantPoolIndex, FieldHandleIndex, FunctionHandleIndex, IdentifierIndex,
    VariantFieldHandleIndex,
};
use move_core_types::{
    function::ClosureMask,
    int256::{I256, U256},
};

/// Named slot operand.
/// TODO(cleanup): consider renaming this enum to `NamedSlot`, to contrast with `SizedSlot`.
///
/// - `Home` — frame-local storage: parameters, declared locals, and temporaries
///   due to destackification. These map 1:1 to frame slots.
///
/// - `Xfer` — transfer slots used for both  passing arguments to a callee
///   (before the call) and receiving return values (after the call).
///   `Xfer` overlaps with the callee's  parameter/return area, so values
///   produced directly into a `Xfer` slot avoid a copy at the call site.
///
/// - `Vid` — SSA value ID, a 0-based temporary that exists only in the
///   pre-allocation IR. Slot allocation replaces every `Vid` with a real
///   `Home` or `Xfer` slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Slot {
    /// Params, locals, and temporaries — displayed as `r0, r1, ...`
    Home(u16),
    /// Call-interface slots — displayed as `x0, x1, ...`
    Xfer(u16),
    /// SSA value ID (pre-allocation only) — displayed as `v0, v1, ...`
    Vid(u16),
}

impl Slot {
    /// Returns `true` if this is a Vid slot (SSA value ID).
    pub fn is_vid(self) -> bool {
        matches!(self, Slot::Vid(_))
    }

    /// Returns `true` if this is a Home slot (pinned local).
    pub fn is_home(self) -> bool {
        matches!(self, Slot::Home(_))
    }
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

/// Immediate values for `BinaryOpImm`. Wide widths (u128 / U256 / i128 /
/// I256) are boxed.
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

/// A stackless IR instruction with explicit named-slot operands.
///
/// TODO(cleanup):
/// (1) convert variants to struct-style (named fields) so call sites read
/// `Instr::Move { dst, src }` rather than positional tuples.
/// (2) add description for each instruction variant.
/// (3) change uses of raw integers into newtypes/type-aliases.
#[derive(Clone)]
pub enum Instr {
    // --- Loads ---
    LdConst(Slot, ConstantPoolIndex),
    LdTrue(Slot),
    LdFalse(Slot),
    LdU8(Slot, u8),
    LdU16(Slot, u16),
    LdU32(Slot, u32),
    LdU64(Slot, u64),
    LdU128(Slot, u128),
    LdU256(Slot, U256),
    LdI8(Slot, i8),
    LdI16(Slot, i16),
    LdI32(Slot, i32),
    LdI64(Slot, i64),
    LdI128(Slot, i128),
    LdI256(Slot, I256),

    // --- Slot ops ---
    /// `dst = copy(src)`, source remains valid.
    Copy(Slot, Slot),
    /// `dst = move(src)`, source invalidated.
    Move(Slot, Slot),

    // --- Unary / Binary ---
    /// `dst = op(src)`
    UnaryOp(Slot, UnaryOp, Slot),
    /// `dst = op(lhs, rhs)`
    BinaryOp(Slot, BinaryOp, Slot, Slot),
    /// `dst = op(lhs_slot, immediate)` — binary op with immediate right operand
    BinaryOpImm(Slot, BinaryOp, Slot, ImmValue),

    // --- Struct (second field is the interned struct `Type`) ---
    //
    // Contract: the carried `InternedType` is the instantiated nominal, with
    // the instantiation's type arguments already applied. Inside a generic
    // function it may still contain the enclosing function's `TypeParam`s.
    Pack(Slot, InternedType, Vec<Slot>),
    Unpack(Vec<Slot>, InternedType, Slot),

    // --- Variant (enum type + variant ordinal; same type contract as
    // `Pack`/`Unpack`) ---
    PackVariant(Slot, InternedType, u16, Vec<Slot>),
    UnpackVariant(Vec<Slot>, InternedType, u16, Slot),
    TestVariant(Slot, InternedType, u16, Slot),

    // --- References ---
    //
    // Field ops carry `(instantiated owner type, non-generic field handle)`:
    // the handle gives the field position; the owner type has the same contract
    // as `Pack`/`Unpack`.
    ImmBorrowLoc(Slot, Slot),
    MutBorrowLoc(Slot, Slot),
    ImmBorrowField(Slot, InternedType, FieldHandleIndex, Slot),
    MutBorrowField(Slot, InternedType, FieldHandleIndex, Slot),
    ImmBorrowVariantField(Slot, InternedType, VariantFieldHandleIndex, Slot),
    MutBorrowVariantField(Slot, InternedType, VariantFieldHandleIndex, Slot),
    ReadRef(Slot, Slot),
    /// `*dst_ref = src_val`
    WriteRef(Slot, Slot),

    // --- Fused field access (borrow+read/write combined) ---
    /// `dst = src_ref.field` (imm_borrow_field + read_ref)
    ReadField(Slot, InternedType, FieldHandleIndex, Slot),
    /// `dst_ref.field = val` (mut_borrow_field + write_ref)
    WriteField(InternedType, FieldHandleIndex, Slot, Slot),
    ReadVariantField(Slot, InternedType, VariantFieldHandleIndex, Slot),
    WriteVariantField(InternedType, VariantFieldHandleIndex, Slot, Slot),

    // --- Fused inline-struct field access (borrow_loc + field op combined) ---
    /// `dst = &local.field` (imm_borrow_loc + imm_borrow_field on an inline struct local)
    ImmBorrowLocField(Slot, InternedType, FieldHandleIndex, Slot),
    /// `dst = &mut local.field`
    MutBorrowLocField(Slot, InternedType, FieldHandleIndex, Slot),
    /// `dst = local.field` (imm_borrow_loc + read_field on an inline struct local)
    ReadLocalField(Slot, InternedType, FieldHandleIndex, Slot),
    /// `local.field = src` (mut_borrow_loc + write_field on an inline struct local)
    WriteLocalField(InternedType, FieldHandleIndex, Slot, Slot),

    // --- Globals (struct type is the interned `Type` for the named
    // resource; same type contract as `Pack`/`Unpack`) ---
    Exists(Slot, InternedType, Slot),
    MoveFrom(Slot, InternedType, Slot),
    /// `(struct_ty, signer, val)`
    MoveTo(InternedType, Slot, Slot),
    ImmBorrowGlobal(Slot, InternedType, Slot),
    MutBorrowGlobal(Slot, InternedType, Slot),

    // --- Calls ---
    //
    // Carries `(inner FunctionHandleIndex, target ty_args)`: the handle gives the
    // callee identity; `ty_args` is the instantiation's type arguments, and is
    // `EMPTY_TYPE_LIST` for a non-generic call. Same type contract as
    // `Pack`/`Unpack` — inside a generic function the args may still contain the
    // enclosing function's `TypeParam`s.
    Call(Vec<Slot>, FunctionHandleIndex, InternedTypeList, Vec<Slot>),

    // --- Closures (same `(inner handle, target ty_args)` contract as `Call`) ---
    PackClosure(
        Slot,
        FunctionHandleIndex,
        InternedTypeList,
        ClosureMask,
        Vec<Slot>,
    ),
    /// `CallClosure(rets, signature_types, args)` — `signature_types` is the
    /// interned list of types from the closure's signature (arg types followed
    /// by result types, matching the source `SignatureIndex`).
    CallClosure(Vec<Slot>, InternedTypeList, Vec<Slot>),

    // --- Vector (second field is the vector's element type) ---
    VecPack(Slot, InternedType, Vec<Slot>),
    VecLen(Slot, InternedType, Slot),
    VecImmBorrow(Slot, InternedType, Slot, Slot),
    VecMutBorrow(Slot, InternedType, Slot, Slot),
    VecPushBack(InternedType, Slot, Slot),
    VecPopBack(Slot, InternedType, Slot),
    VecUnpack(Vec<Slot>, InternedType, Slot),
    VecSwap(InternedType, Slot, Slot, Slot),

    // --- Control flow ---
    Branch(Label),
    BrTrue(Label, Slot),
    BrFalse(Label, Slot),
    /// `BrCmp(target, op, lhs, rhs)` — branch to `target` if `op(lhs, rhs)` is true.
    BrCmp(Label, CmpKind, Slot, Slot),
    /// `BrCmpImm(target, op, src, imm)` — branch to `target` if `op(src, imm)` is true.
    BrCmpImm(Label, CmpKind, Slot, ImmValue),
    Ret(Vec<Slot>),
    Abort(Slot),
    AbortMsg(Slot, Slot),

    // --- Test intrinsics ---
    /// Triggers a garbage collection.
    ForceGC,
}

impl Instr {
    /// Returns the variant tag as a static string. Useful for terse error
    /// messages that don't need the full operand dump.
    pub fn opcode_name(&self) -> &'static str {
        match self {
            Instr::LdConst(..) => "LdConst",
            Instr::LdTrue(..) => "LdTrue",
            Instr::LdFalse(..) => "LdFalse",
            Instr::LdU8(..) => "LdU8",
            Instr::LdU16(..) => "LdU16",
            Instr::LdU32(..) => "LdU32",
            Instr::LdU64(..) => "LdU64",
            Instr::LdU128(..) => "LdU128",
            Instr::LdU256(..) => "LdU256",
            Instr::LdI8(..) => "LdI8",
            Instr::LdI16(..) => "LdI16",
            Instr::LdI32(..) => "LdI32",
            Instr::LdI64(..) => "LdI64",
            Instr::LdI128(..) => "LdI128",
            Instr::LdI256(..) => "LdI256",
            Instr::Copy(..) => "Copy",
            Instr::Move(..) => "Move",
            Instr::UnaryOp(..) => "UnaryOp",
            Instr::BinaryOp(..) => "BinaryOp",
            Instr::BinaryOpImm(..) => "BinaryOpImm",
            Instr::Pack(..) => "Pack",
            Instr::Unpack(..) => "Unpack",
            Instr::PackVariant(..) => "PackVariant",
            Instr::UnpackVariant(..) => "UnpackVariant",
            Instr::TestVariant(..) => "TestVariant",
            Instr::ImmBorrowLoc(..) => "ImmBorrowLoc",
            Instr::MutBorrowLoc(..) => "MutBorrowLoc",
            Instr::ImmBorrowField(..) => "ImmBorrowField",
            Instr::MutBorrowField(..) => "MutBorrowField",
            Instr::ImmBorrowVariantField(..) => "ImmBorrowVariantField",
            Instr::MutBorrowVariantField(..) => "MutBorrowVariantField",
            Instr::ReadRef(..) => "ReadRef",
            Instr::WriteRef(..) => "WriteRef",
            Instr::ReadField(..) => "ReadField",
            Instr::WriteField(..) => "WriteField",
            Instr::ReadVariantField(..) => "ReadVariantField",
            Instr::WriteVariantField(..) => "WriteVariantField",
            Instr::ImmBorrowLocField(..) => "ImmBorrowLocField",
            Instr::MutBorrowLocField(..) => "MutBorrowLocField",
            Instr::ReadLocalField(..) => "ReadLocalField",
            Instr::WriteLocalField(..) => "WriteLocalField",
            Instr::Exists(..) => "Exists",
            Instr::MoveFrom(..) => "MoveFrom",
            Instr::MoveTo(..) => "MoveTo",
            Instr::ImmBorrowGlobal(..) => "ImmBorrowGlobal",
            Instr::MutBorrowGlobal(..) => "MutBorrowGlobal",
            Instr::Call(..) => "Call",
            Instr::PackClosure(..) => "PackClosure",
            Instr::CallClosure(..) => "CallClosure",
            Instr::VecPack(..) => "VecPack",
            Instr::VecLen(..) => "VecLen",
            Instr::VecImmBorrow(..) => "VecImmBorrow",
            Instr::VecMutBorrow(..) => "VecMutBorrow",
            Instr::VecPushBack(..) => "VecPushBack",
            Instr::VecPopBack(..) => "VecPopBack",
            Instr::VecUnpack(..) => "VecUnpack",
            Instr::VecSwap(..) => "VecSwap",
            Instr::Branch(..) => "Branch",
            Instr::BrTrue(..) => "BrTrue",
            Instr::BrFalse(..) => "BrFalse",
            Instr::BrCmp(..) => "BrCmp",
            Instr::BrCmpImm(..) => "BrCmpImm",
            Instr::Ret(..) => "Ret",
            Instr::Abort(..) => "Abort",
            Instr::AbortMsg(..) => "AbortMsg",
            Instr::ForceGC => "ForceGC",
        }
    }
}

/// A basic block of instructions.
///
/// Every block has a label. The last instruction is a terminator.
/// (`Branch`, `BrTrue`, `BrFalse`, `Ret`, `Abort`, `AbortMsg`).
pub struct BasicBlock {
    /// Label identifying this block.
    pub label: Label,
    /// Instructions in this block.
    pub instrs: Vec<Instr>,
}

/// IR for a single function.
pub struct FunctionIR {
    /// Function name in identifier pool.
    pub name_idx: IdentifierIndex,
    /// Function handle index.
    pub handle_idx: FunctionHandleIndex,
    /// Number of parameters (count, not a slot).
    pub num_params: u16,
    /// Number of non-param locals (count, not a slot).
    pub num_locals: u16,
    /// Total Home slots used (params + locals + temps).
    pub num_home_slots: u16,
    /// Number of distinct `Xfer(j)` positions used across all calls in this
    /// function.
    pub num_xfer_positions: u16,
    /// Basic blocks of the function.
    pub blocks: Vec<BasicBlock>,
    /// Type of each Home slot (indexed by Home slot index, 0..num_home_slots-1).
    /// Xfer slots have no entry here — their types are inferred from call signatures.
    pub home_slot_types: Vec<InternedType>,
    /// Gas cost of each block as an unresolved formula, indexed by block label.
    pub(crate) block_costs: Vec<BlockCost>,
}

impl FunctionIR {
    /// Iterate over all instructions across all blocks.
    pub fn instrs(&self) -> impl Iterator<Item = &Instr> {
        self.blocks.iter().flat_map(|b| b.instrs.iter())
    }

    /// Iterate mutably over all instructions across all blocks.
    pub fn instrs_mut(&mut self) -> impl Iterator<Item = &mut Instr> {
        self.blocks.iter_mut().flat_map(|b| b.instrs.iter_mut())
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
