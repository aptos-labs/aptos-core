// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::file_format::{
    ConstantPoolIndex, FieldHandleIndex, FieldInstantiationIndex, FunctionHandleIndex,
    FunctionInstantiationIndex, LocalIndex, SignatureIndex, StructDefInstantiationIndex,
    StructDefinitionIndex, StructVariantHandleIndex, StructVariantInstantiationIndex,
    VariantFieldHandleIndex, VariantFieldInstantiationIndex,
};
use move_core_types::function::ClosureMask;

#[derive(Debug)]
pub enum Bytecode {
    CopyLoc(LocalIndex),
    MoveLoc(LocalIndex),
    StLoc(LocalIndex),
    MutBorrowLoc(LocalIndex),
    ImmBorrowLoc(LocalIndex),

    Call(FunctionHandleIndex),
    CallGeneric(FunctionInstantiationIndex),

    Pop,
    Ret,
    Abort,

    BrTrue(u16),
    BrFalse(u16),
    Branch(u16),

    LdTrue,
    LdFalse,
    LdU8(u8),
    LdU16(u16),
    LdU32(u32),
    LdU64(u64),
    LdU128(u128),
    LdU256(move_core_types::u256::U256),
    LdConst(ConstantPoolIndex),

    CastU8,
    CastU16,
    CastU32,
    CastU64,
    CastU128,
    CastU256,

    Pack(StructDefinitionIndex),
    PackGeneric(StructDefInstantiationIndex),
    ImmBorrowField(FieldHandleIndex),
    ImmBorrowFieldGeneric(FieldInstantiationIndex),
    MutBorrowField(FieldHandleIndex),
    MutBorrowFieldGeneric(FieldInstantiationIndex),
    Unpack(StructDefinitionIndex),
    UnpackGeneric(StructDefInstantiationIndex),

    PackVariant(StructVariantHandleIndex),
    PackVariantGeneric(StructVariantInstantiationIndex),
    TestVariant(StructVariantHandleIndex),
    TestVariantGeneric(StructVariantInstantiationIndex),
    ImmBorrowVariantField(VariantFieldHandleIndex),
    ImmBorrowVariantFieldGeneric(VariantFieldInstantiationIndex),
    MutBorrowVariantField(VariantFieldHandleIndex),
    MutBorrowVariantFieldGeneric(VariantFieldInstantiationIndex),
    UnpackVariant(StructVariantHandleIndex),
    UnpackVariantGeneric(StructVariantInstantiationIndex),

    ReadRef,
    WriteRef,
    FreezeRef,

    MoveTo(StructDefinitionIndex),
    MoveToGeneric(StructDefInstantiationIndex),
    Exists(StructDefinitionIndex),
    ExistsGeneric(StructDefInstantiationIndex),
    ImmBorrowGlobal(StructDefinitionIndex),
    ImmBorrowGlobalGeneric(StructDefInstantiationIndex),
    MutBorrowGlobal(StructDefinitionIndex),
    MutBorrowGlobalGeneric(StructDefInstantiationIndex),
    MoveFrom(StructDefinitionIndex),
    MoveFromGeneric(StructDefInstantiationIndex),

    Add,
    Sub,
    Mul,
    Mod,
    Div,
    BitOr,
    BitAnd,

    Xor,
    Shl,
    Shr,

    Or,
    And,
    Not,

    Eq,
    Neq,

    Lt,
    Gt,
    Le,
    Ge,

    VecPack(SignatureIndex, u64),
    VecLen(SignatureIndex),
    VecImmBorrow(SignatureIndex),
    VecMutBorrow(SignatureIndex),
    VecPushBack(SignatureIndex),
    VecPopBack(SignatureIndex),
    VecUnpack(SignatureIndex, u64),
    VecSwap(SignatureIndex),

    PackClosure(FunctionHandleIndex, ClosureMask),
    PackClosureGeneric(FunctionInstantiationIndex, ClosureMask),
    CallClosure(SignatureIndex),

    Nop,
}
