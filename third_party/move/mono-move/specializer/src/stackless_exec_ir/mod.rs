// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Data types for the stackless execution IR.
//!
//! This IR converts Move's stack-based bytecode into explicit named-slot form,
//! eliminating the operand stack and allowing direct named-slot operands on each instruction.

mod display;

use mono_move_core::{
    types::{InternedType, InternedTypeList},
    PreparedModule,
};
use move_binary_format::file_format::{
    ConstantPoolIndex, FieldHandleIndex, FieldInstantiationIndex, FunctionHandleIndex,
    FunctionInstantiationIndex, IdentifierIndex, VariantFieldHandleIndex,
    VariantFieldInstantiationIndex,
};
use move_core_types::{
    function::ClosureMask,
    int256::{I256, U256},
};

/// Named slot operand.
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
    CastU8,
    CastU16,
    CastU32,
    CastU64,
    CastU128,
    CastU256,
    CastI8,
    CastI16,
    CastI32,
    CastI64,
    CastI128,
    CastI256,
    Not,
    Negate,
    FreezeRef,
}

/// Comparison operations that produce a boolean result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmpOp {
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Neq,
}

impl CmpOp {
    /// Return the logically negated comparison.
    pub fn negate(self) -> Self {
        match self {
            CmpOp::Lt => CmpOp::Ge,
            CmpOp::Ge => CmpOp::Lt,
            CmpOp::Gt => CmpOp::Le,
            CmpOp::Le => CmpOp::Gt,
            CmpOp::Eq => CmpOp::Neq,
            CmpOp::Neq => CmpOp::Eq,
        }
    }
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
    Xor,
    Shl,
    Shr,
    Cmp(CmpOp),
    Or,
    And,
}

/// Immediate values for `BinaryOpImm`. Restricted to small types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImmValue {
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
}

/// A stackless IR instruction with explicit named-slot operands.
///
/// TODO: convert variants to struct-style (named fields) so call sites read
/// `Instr::Pack { dst, ty, args }` rather than positional tuples.
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

    // --- Struct (second field is the interned struct `Type`; generic
    // variants additionally carry an interned type-argument list) ---
    //
    // TODO: depending on how we pre-intern types, we may be able to unify
    // some of instructions here.
    Pack(Slot, InternedType, Vec<Slot>),
    PackGeneric(Slot, InternedType, InternedTypeList, Vec<Slot>),
    Unpack(Vec<Slot>, InternedType, Slot),
    UnpackGeneric(Vec<Slot>, InternedType, InternedTypeList, Slot),

    // --- Variant (enum type + variant ordinal; generic variants also
    // carry an interned type-argument list) ---
    PackVariant(Slot, InternedType, u16, Vec<Slot>),
    PackVariantGeneric(Slot, InternedType, u16, InternedTypeList, Vec<Slot>),
    UnpackVariant(Vec<Slot>, InternedType, u16, Slot),
    UnpackVariantGeneric(Vec<Slot>, InternedType, u16, InternedTypeList, Slot),
    TestVariant(Slot, InternedType, u16, Slot),
    TestVariantGeneric(Slot, InternedType, u16, InternedTypeList, Slot),

    // --- References ---
    ImmBorrowLoc(Slot, Slot),
    MutBorrowLoc(Slot, Slot),
    ImmBorrowField(Slot, FieldHandleIndex, Slot),
    MutBorrowField(Slot, FieldHandleIndex, Slot),
    ImmBorrowFieldGeneric(Slot, FieldInstantiationIndex, Slot),
    MutBorrowFieldGeneric(Slot, FieldInstantiationIndex, Slot),
    ImmBorrowVariantField(Slot, VariantFieldHandleIndex, Slot),
    MutBorrowVariantField(Slot, VariantFieldHandleIndex, Slot),
    ImmBorrowVariantFieldGeneric(Slot, VariantFieldInstantiationIndex, Slot),
    MutBorrowVariantFieldGeneric(Slot, VariantFieldInstantiationIndex, Slot),
    ReadRef(Slot, Slot),
    /// `*dst_ref = src_val`
    WriteRef(Slot, Slot),

    // --- Fused field access (borrow+read/write combined) ---
    /// `dst = src_ref.field` (imm_borrow_field + read_ref)
    ReadField(Slot, FieldHandleIndex, Slot),
    ReadFieldGeneric(Slot, FieldInstantiationIndex, Slot),
    /// `dst_ref.field = val` (mut_borrow_field + write_ref)
    WriteField(FieldHandleIndex, Slot, Slot),
    WriteFieldGeneric(FieldInstantiationIndex, Slot, Slot),
    ReadVariantField(Slot, VariantFieldHandleIndex, Slot),
    ReadVariantFieldGeneric(Slot, VariantFieldInstantiationIndex, Slot),
    WriteVariantField(VariantFieldHandleIndex, Slot, Slot),
    WriteVariantFieldGeneric(VariantFieldInstantiationIndex, Slot, Slot),

    // --- Globals (struct type is the interned `Type` for the named resource;
    // generic variants additionally carry an interned type-argument list) ---
    Exists(Slot, InternedType, Slot),
    ExistsGeneric(Slot, InternedType, InternedTypeList, Slot),
    MoveFrom(Slot, InternedType, Slot),
    MoveFromGeneric(Slot, InternedType, InternedTypeList, Slot),
    /// `(struct_ty, signer, val)`
    MoveTo(InternedType, Slot, Slot),
    MoveToGeneric(InternedType, InternedTypeList, Slot, Slot),
    ImmBorrowGlobal(Slot, InternedType, Slot),
    ImmBorrowGlobalGeneric(Slot, InternedType, InternedTypeList, Slot),
    MutBorrowGlobal(Slot, InternedType, Slot),
    MutBorrowGlobalGeneric(Slot, InternedType, InternedTypeList, Slot),

    // --- Calls ---
    Call(Vec<Slot>, FunctionHandleIndex, Vec<Slot>),
    CallGeneric(Vec<Slot>, FunctionInstantiationIndex, Vec<Slot>),

    // --- Closures ---
    PackClosure(Slot, FunctionHandleIndex, ClosureMask, Vec<Slot>),
    PackClosureGeneric(Slot, FunctionInstantiationIndex, ClosureMask, Vec<Slot>),
    /// `CallClosure(rets, signature_types, args)` — `signature_types` is the
    /// interned list of types from the closure's signature (arg types followed
    /// by result types, matching the source `SignatureIndex`).
    CallClosure(Vec<Slot>, InternedTypeList, Vec<Slot>),

    // --- Vector (second field is the vector's element type) ---
    VecPack(Slot, InternedType, u16, Vec<Slot>),
    VecLen(Slot, InternedType, Slot),
    VecImmBorrow(Slot, InternedType, Slot, Slot),
    VecMutBorrow(Slot, InternedType, Slot, Slot),
    VecPushBack(InternedType, Slot, Slot),
    VecPopBack(Slot, InternedType, Slot),
    VecUnpack(Vec<Slot>, InternedType, u16, Slot),
    VecSwap(InternedType, Slot, Slot, Slot),

    // --- Control flow ---
    Branch(Label),
    BrTrue(Label, Slot),
    BrFalse(Label, Slot),
    /// `BrCmp(target, op, lhs, rhs)` — branch to `target` if `op(lhs, rhs)` is true.
    BrCmp(Label, CmpOp, Slot, Slot),
    /// `BrCmpImm(target, op, src, imm)` — branch to `target` if `op(src, imm)` is true.
    BrCmpImm(Label, CmpOp, Slot, ImmValue),
    Ret(Vec<Slot>),
    Abort(Slot),
    AbortMsg(Slot, Slot),
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
            Instr::PackGeneric(..) => "PackGeneric",
            Instr::Unpack(..) => "Unpack",
            Instr::UnpackGeneric(..) => "UnpackGeneric",
            Instr::PackVariant(..) => "PackVariant",
            Instr::PackVariantGeneric(..) => "PackVariantGeneric",
            Instr::UnpackVariant(..) => "UnpackVariant",
            Instr::UnpackVariantGeneric(..) => "UnpackVariantGeneric",
            Instr::TestVariant(..) => "TestVariant",
            Instr::TestVariantGeneric(..) => "TestVariantGeneric",
            Instr::ImmBorrowLoc(..) => "ImmBorrowLoc",
            Instr::MutBorrowLoc(..) => "MutBorrowLoc",
            Instr::ImmBorrowField(..) => "ImmBorrowField",
            Instr::MutBorrowField(..) => "MutBorrowField",
            Instr::ImmBorrowFieldGeneric(..) => "ImmBorrowFieldGeneric",
            Instr::MutBorrowFieldGeneric(..) => "MutBorrowFieldGeneric",
            Instr::ImmBorrowVariantField(..) => "ImmBorrowVariantField",
            Instr::MutBorrowVariantField(..) => "MutBorrowVariantField",
            Instr::ImmBorrowVariantFieldGeneric(..) => "ImmBorrowVariantFieldGeneric",
            Instr::MutBorrowVariantFieldGeneric(..) => "MutBorrowVariantFieldGeneric",
            Instr::ReadRef(..) => "ReadRef",
            Instr::WriteRef(..) => "WriteRef",
            Instr::ReadField(..) => "ReadField",
            Instr::ReadFieldGeneric(..) => "ReadFieldGeneric",
            Instr::WriteField(..) => "WriteField",
            Instr::WriteFieldGeneric(..) => "WriteFieldGeneric",
            Instr::ReadVariantField(..) => "ReadVariantField",
            Instr::ReadVariantFieldGeneric(..) => "ReadVariantFieldGeneric",
            Instr::WriteVariantField(..) => "WriteVariantField",
            Instr::WriteVariantFieldGeneric(..) => "WriteVariantFieldGeneric",
            Instr::Exists(..) => "Exists",
            Instr::ExistsGeneric(..) => "ExistsGeneric",
            Instr::MoveFrom(..) => "MoveFrom",
            Instr::MoveFromGeneric(..) => "MoveFromGeneric",
            Instr::MoveTo(..) => "MoveTo",
            Instr::MoveToGeneric(..) => "MoveToGeneric",
            Instr::ImmBorrowGlobal(..) => "ImmBorrowGlobal",
            Instr::ImmBorrowGlobalGeneric(..) => "ImmBorrowGlobalGeneric",
            Instr::MutBorrowGlobal(..) => "MutBorrowGlobal",
            Instr::MutBorrowGlobalGeneric(..) => "MutBorrowGlobalGeneric",
            Instr::Call(..) => "Call",
            Instr::CallGeneric(..) => "CallGeneric",
            Instr::PackClosure(..) => "PackClosure",
            Instr::PackClosureGeneric(..) => "PackClosureGeneric",
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
    /// Maximum number of Xfer slots needed across all call sites in this function.
    pub num_xfer_slots: u16,
    /// Basic blocks of the function.
    pub blocks: Vec<BasicBlock>,
    /// Type of each Home slot (indexed by Home slot index, 0..num_home_slots-1).
    /// Xfer slots have no entry here — their types are inferred from call signatures.
    pub home_slot_types: Vec<InternedType>,
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
