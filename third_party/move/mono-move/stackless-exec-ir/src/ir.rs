// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Data types for the stackless execution IR.
//!
//! This IR converts Move's stack-based bytecode into explicit register-based form,
//! eliminating the operand stack and allowing direct register operands on each instruction.

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

/// Register index. Params+locals occupy `0..num_params+num_locals-1`.
/// Synthetic temporaries start at `num_params+num_locals`.
pub type Reg = u16;

/// Label for branch targets.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Label(pub u16);

/// Unary operations (pop 1, push 1).
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

/// Binary operations (pop 2, push 1).
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

/// A stackless IR instruction with explicit register operands.
#[derive(Clone, Debug)]
pub enum Instr {
    // --- Loads ---
    LdConst(Reg, ConstantPoolIndex),
    LdTrue(Reg),
    LdFalse(Reg),
    LdU8(Reg, u8),
    LdU16(Reg, u16),
    LdU32(Reg, u32),
    LdU64(Reg, u64),
    LdU128(Reg, u128),
    LdU256(Reg, U256),
    LdI8(Reg, i8),
    LdI16(Reg, i16),
    LdI32(Reg, i32),
    LdI64(Reg, i64),
    LdI128(Reg, i128),
    LdI256(Reg, I256),

    // --- Register ops ---
    /// `dst = copy(src)`, source remains valid.
    Copy(Reg, Reg),
    /// `dst = move(src)`, source invalidated.
    Move(Reg, Reg),

    // --- Unary / Binary ---
    /// `dst = op(src)`
    UnaryOp(Reg, UnaryOp, Reg),
    /// `dst = op(lhs, rhs)`
    BinaryOp(Reg, BinaryOp, Reg, Reg),
    /// `dst = op(lhs_reg, immediate)` — binary op with immediate right operand
    BinaryOpImm(Reg, BinaryOp, Reg, ImmValue),

    // --- Struct ---
    Pack(Reg, StructDefinitionIndex, Vec<Reg>),
    PackGeneric(Reg, StructDefInstantiationIndex, Vec<Reg>),
    Unpack(Vec<Reg>, StructDefinitionIndex, Reg),
    UnpackGeneric(Vec<Reg>, StructDefInstantiationIndex, Reg),

    // --- Variant ---
    PackVariant(Reg, StructVariantHandleIndex, Vec<Reg>),
    PackVariantGeneric(Reg, StructVariantInstantiationIndex, Vec<Reg>),
    UnpackVariant(Vec<Reg>, StructVariantHandleIndex, Reg),
    UnpackVariantGeneric(Vec<Reg>, StructVariantInstantiationIndex, Reg),
    TestVariant(Reg, StructVariantHandleIndex, Reg),
    TestVariantGeneric(Reg, StructVariantInstantiationIndex, Reg),

    // --- References ---
    ImmBorrowLoc(Reg, Reg),
    MutBorrowLoc(Reg, Reg),
    ImmBorrowField(Reg, FieldHandleIndex, Reg),
    MutBorrowField(Reg, FieldHandleIndex, Reg),
    ImmBorrowFieldGeneric(Reg, FieldInstantiationIndex, Reg),
    MutBorrowFieldGeneric(Reg, FieldInstantiationIndex, Reg),
    ImmBorrowVariantField(Reg, VariantFieldHandleIndex, Reg),
    MutBorrowVariantField(Reg, VariantFieldHandleIndex, Reg),
    ImmBorrowVariantFieldGeneric(Reg, VariantFieldInstantiationIndex, Reg),
    MutBorrowVariantFieldGeneric(Reg, VariantFieldInstantiationIndex, Reg),
    ReadRef(Reg, Reg),
    /// `*dst_ref = src_val`
    WriteRef(Reg, Reg),

    // --- Fused field access (borrow+read/write combined) ---
    /// `dst = src_ref.field` (imm_borrow_field + read_ref)
    ReadField(Reg, FieldHandleIndex, Reg),
    ReadFieldGeneric(Reg, FieldInstantiationIndex, Reg),
    /// `dst_ref.field = val` (mut_borrow_field + write_ref)
    WriteField(FieldHandleIndex, Reg, Reg),
    WriteFieldGeneric(FieldInstantiationIndex, Reg, Reg),
    ReadVariantField(Reg, VariantFieldHandleIndex, Reg),
    ReadVariantFieldGeneric(Reg, VariantFieldInstantiationIndex, Reg),
    WriteVariantField(VariantFieldHandleIndex, Reg, Reg),
    WriteVariantFieldGeneric(VariantFieldInstantiationIndex, Reg, Reg),

    // --- Globals ---
    Exists(Reg, StructDefinitionIndex, Reg),
    ExistsGeneric(Reg, StructDefInstantiationIndex, Reg),
    MoveFrom(Reg, StructDefinitionIndex, Reg),
    MoveFromGeneric(Reg, StructDefInstantiationIndex, Reg),
    /// `(def, signer, val)`
    MoveTo(StructDefinitionIndex, Reg, Reg),
    MoveToGeneric(StructDefInstantiationIndex, Reg, Reg),
    ImmBorrowGlobal(Reg, StructDefinitionIndex, Reg),
    ImmBorrowGlobalGeneric(Reg, StructDefInstantiationIndex, Reg),
    MutBorrowGlobal(Reg, StructDefinitionIndex, Reg),
    MutBorrowGlobalGeneric(Reg, StructDefInstantiationIndex, Reg),

    // --- Calls ---
    Call(Vec<Reg>, FunctionHandleIndex, Vec<Reg>),
    CallGeneric(Vec<Reg>, FunctionInstantiationIndex, Vec<Reg>),

    // --- Closures ---
    PackClosure(Reg, FunctionHandleIndex, ClosureMask, Vec<Reg>),
    PackClosureGeneric(Reg, FunctionInstantiationIndex, ClosureMask, Vec<Reg>),
    CallClosure(Vec<Reg>, SignatureIndex, Vec<Reg>),

    // --- Vector ---
    VecPack(Reg, SignatureIndex, u64, Vec<Reg>),
    VecLen(Reg, SignatureIndex, Reg),
    VecImmBorrow(Reg, SignatureIndex, Reg, Reg),
    VecMutBorrow(Reg, SignatureIndex, Reg, Reg),
    VecPushBack(SignatureIndex, Reg, Reg),
    VecPopBack(Reg, SignatureIndex, Reg),
    VecUnpack(Vec<Reg>, SignatureIndex, u64, Reg),
    VecSwap(SignatureIndex, Reg, Reg, Reg),

    // --- Control flow ---
    Label(Label),
    Branch(Label),
    BrTrue(Label, Reg),
    BrFalse(Label, Reg),
    Ret(Vec<Reg>),
    Abort(Reg),
    AbortMsg(Reg, Reg),

}

/// IR for a single function.
pub struct FunctionIR {
    /// Function name in identifier pool.
    pub name_idx: IdentifierIndex,
    /// Function handle index.
    pub handle_idx: FunctionHandleIndex,
    /// Number of parameters.
    pub num_params: Reg,
    /// Number of non-param locals.
    pub num_locals: Reg,
    /// Total registers used (params + locals + temps).
    pub num_regs: Reg,
    /// The instruction stream.
    pub instrs: Vec<Instr>,
    /// Type of each register (indexed by Reg).
    pub reg_types: Vec<Type>,
}

/// IR for a module (wraps the original CompiledModule for pool access).
pub struct ModuleIR {
    /// The original compiled module for resolving pool indices.
    pub module: CompiledModule,
    /// One per non-native FunctionDefinition.
    pub functions: Vec<FunctionIR>,
}
