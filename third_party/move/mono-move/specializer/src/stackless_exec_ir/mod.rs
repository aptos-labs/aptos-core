// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Data types for the stackless execution IR.
//!
//! This IR converts Move's stack-based bytecode into explicit named-slot form,
//! eliminating the operand stack and allowing direct named-slot operands on each instruction.

mod display;

use move_binary_format::{
    file_format::{
        ConstantPoolIndex, FieldHandleIndex, FieldInstantiationIndex, FunctionHandleIndex,
        FunctionInstantiationIndex, IdentifierIndex, SignatureIndex, StructDefInstantiationIndex,
        StructDefinitionIndex, StructVariantHandleIndex, StructVariantInstantiationIndex,
        VariantFieldHandleIndex, VariantFieldInstantiationIndex,
    },
    CompiledModule,
};
use move_core_types::{
    function::ClosureMask,
    int256::{I256, U256},
};
use move_vm_types::loaded_data::runtime_types::Type;

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
#[derive(Clone, Debug, PartialEq, Eq)]
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

/// Binary operations.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Neq,
    Or,
    And,
}

/// Immediate values for `BinaryOpImm`. Restricted to small types.
#[derive(Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug)]
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

    // --- Struct ---
    Pack(Slot, StructDefinitionIndex, Vec<Slot>),
    PackGeneric(Slot, StructDefInstantiationIndex, Vec<Slot>),
    Unpack(Vec<Slot>, StructDefinitionIndex, Slot),
    UnpackGeneric(Vec<Slot>, StructDefInstantiationIndex, Slot),

    // --- Variant ---
    PackVariant(Slot, StructVariantHandleIndex, Vec<Slot>),
    PackVariantGeneric(Slot, StructVariantInstantiationIndex, Vec<Slot>),
    UnpackVariant(Vec<Slot>, StructVariantHandleIndex, Slot),
    UnpackVariantGeneric(Vec<Slot>, StructVariantInstantiationIndex, Slot),
    TestVariant(Slot, StructVariantHandleIndex, Slot),
    TestVariantGeneric(Slot, StructVariantInstantiationIndex, Slot),

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

    // --- Globals ---
    Exists(Slot, StructDefinitionIndex, Slot),
    ExistsGeneric(Slot, StructDefInstantiationIndex, Slot),
    MoveFrom(Slot, StructDefinitionIndex, Slot),
    MoveFromGeneric(Slot, StructDefInstantiationIndex, Slot),
    /// `(def, signer, val)`
    MoveTo(StructDefinitionIndex, Slot, Slot),
    MoveToGeneric(StructDefInstantiationIndex, Slot, Slot),
    ImmBorrowGlobal(Slot, StructDefinitionIndex, Slot),
    ImmBorrowGlobalGeneric(Slot, StructDefInstantiationIndex, Slot),
    MutBorrowGlobal(Slot, StructDefinitionIndex, Slot),
    MutBorrowGlobalGeneric(Slot, StructDefInstantiationIndex, Slot),

    // --- Calls ---
    Call(Vec<Slot>, FunctionHandleIndex, Vec<Slot>),
    CallGeneric(Vec<Slot>, FunctionInstantiationIndex, Vec<Slot>),

    // --- Closures ---
    PackClosure(Slot, FunctionHandleIndex, ClosureMask, Vec<Slot>),
    PackClosureGeneric(Slot, FunctionInstantiationIndex, ClosureMask, Vec<Slot>),
    CallClosure(Vec<Slot>, SignatureIndex, Vec<Slot>),

    // --- Vector ---
    VecPack(Slot, SignatureIndex, u16, Vec<Slot>),
    VecLen(Slot, SignatureIndex, Slot),
    VecImmBorrow(Slot, SignatureIndex, Slot, Slot),
    VecMutBorrow(Slot, SignatureIndex, Slot, Slot),
    VecPushBack(SignatureIndex, Slot, Slot),
    VecPopBack(Slot, SignatureIndex, Slot),
    VecUnpack(Vec<Slot>, SignatureIndex, u16, Slot),
    VecSwap(SignatureIndex, Slot, Slot, Slot),

    // --- Control flow ---
    Branch(Label),
    BrTrue(Label, Slot),
    BrFalse(Label, Slot),
    Ret(Vec<Slot>),
    Abort(Slot),
    AbortMsg(Slot, Slot),
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
    pub home_slot_types: Vec<Type>,
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

/// IR for a module (wraps the original CompiledModule for pool access).
pub struct ModuleIR {
    /// The original compiled module for resolving pool indices.
    pub module: CompiledModule,
    /// One per non-native FunctionDefinition.
    pub functions: Vec<FunctionIR>,
}
