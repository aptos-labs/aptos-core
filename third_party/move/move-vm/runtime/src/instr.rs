// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::loader::{
    FieldHandle, FieldInstantiation, StructDef, StructInstantiation, StructVariantInfo,
};
use move_binary_format::{
    errors::{PartialVMError, PartialVMResult},
    file_format::{
        Bytecode, CodeOffset, ConstantPoolIndex, FieldHandleIndex, FieldInstantiationIndex,
        FunctionHandle, FunctionHandleIndex, FunctionInstantiationIndex, LocalIndex,
        SignatureIndex, StructDefInstantiationIndex, StructDefinitionIndex,
        StructVariantHandleIndex, StructVariantInstantiationIndex, VariantFieldHandleIndex,
        VariantFieldInstantiationIndex, VariantIndex,
    },
};
use move_core_types::{
    function::ClosureMask,
    int256::{I256, U256},
    vm_status::StatusCode,
};
use move_vm_types::loaded_data::{
    runtime_types::{AbilityInfo, Type, TypeBuilder},
    struct_name_indexing::StructNameIndex,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TestVariantV2 {
    pub variant_idx: VariantIndex,
    pub struct_name_idx: StructNameIndex,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BorrowFieldV2 {
    pub is_mut: bool,
    pub field_offset: usize,
    pub struct_name_idx: StructNameIndex,
    pub field_ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackV2 {
    pub is_generic: bool,
    pub field_count: u16,
    pub struct_ty: Type,
    pub field_tys: Vec<Type>,
}

/// The VM's internal representation of instructions.
///
/// Currently, it is an exact mirror of the Move bytecode, but can be extended with more
/// instructions in the future.
///
/// This provides path for incremental performance optimizations, while making it less painful to
/// maintain backward compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Instruction {
    Pop,
    Ret,
    BrTrue(CodeOffset),
    BrFalse(CodeOffset),
    Branch(CodeOffset),
    LdU8(u8),
    LdU64(u64),
    LdU128(Box<u128>),
    CastU8,
    CastU64,
    CastU128,
    LdConst(ConstantPoolIndex),
    LdTrue,
    LdFalse,
    CopyLoc(LocalIndex),
    MoveLoc(LocalIndex),
    StLoc(LocalIndex),
    Call(FunctionHandleIndex),
    CallGeneric(FunctionInstantiationIndex),
    Pack(StructDefinitionIndex),
    PackGeneric(StructDefInstantiationIndex),
    PackVariant(StructVariantHandleIndex),
    PackVariantGeneric(StructVariantInstantiationIndex),
    Unpack(StructDefinitionIndex),
    UnpackGeneric(StructDefInstantiationIndex),
    UnpackVariant(StructVariantHandleIndex),
    UnpackVariantGeneric(StructVariantInstantiationIndex),
    TestVariant(StructVariantHandleIndex),
    TestVariantGeneric(StructVariantInstantiationIndex),
    ReadRef,
    WriteRef,
    FreezeRef,
    MutBorrowLoc(LocalIndex),
    ImmBorrowLoc(LocalIndex),
    MutBorrowField(FieldHandleIndex),
    MutBorrowVariantField(VariantFieldHandleIndex),
    MutBorrowFieldGeneric(FieldInstantiationIndex),
    MutBorrowVariantFieldGeneric(VariantFieldInstantiationIndex),
    ImmBorrowField(FieldHandleIndex),
    ImmBorrowVariantField(VariantFieldHandleIndex),
    ImmBorrowFieldGeneric(FieldInstantiationIndex),
    ImmBorrowVariantFieldGeneric(VariantFieldInstantiationIndex),
    MutBorrowGlobal(StructDefinitionIndex),
    MutBorrowGlobalGeneric(StructDefInstantiationIndex),
    ImmBorrowGlobal(StructDefinitionIndex),
    ImmBorrowGlobalGeneric(StructDefInstantiationIndex),
    Add,
    Sub,
    Mul,
    Mod,
    Div,
    BitOr,
    BitAnd,
    Xor,
    Or,
    And,
    Not,
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
    Abort,
    Nop,
    Exists(StructDefinitionIndex),
    ExistsGeneric(StructDefInstantiationIndex),
    MoveFrom(StructDefinitionIndex),
    MoveFromGeneric(StructDefInstantiationIndex),
    MoveTo(StructDefinitionIndex),
    MoveToGeneric(StructDefInstantiationIndex),
    Shl,
    Shr,
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
    LdU16(u16),
    LdU32(u32),
    LdU256(Box<U256>),
    CastU16,
    CastU32,
    CastU256,
    LdI8(i8),
    LdI16(i16),
    LdI32(i32),
    LdI64(i64),
    LdI128(Box<i128>),
    LdI256(Box<I256>),
    CastI8,
    CastI16,
    CastI32,
    CastI64,
    CastI128,
    CastI256,
    Negate,

    VecLenV2,
    TestVariantV2(TestVariantV2),
    BorrowFieldV2(Box<BorrowFieldV2>),
    PackV2(Box<PackV2>),
}

pub(crate) struct BytecodeTransformer<'a> {
    pub(crate) use_fast_instructions: bool,

    pub(crate) ty_builder: TypeBuilder,

    pub(crate) structs: &'a [StructDef],
    pub(crate) struct_instantiations: &'a [StructInstantiation],

    pub(crate) struct_variant_infos: &'a [StructVariantInfo],
    pub(crate) struct_variant_instantiation_infos: &'a [StructVariantInfo],

    pub(crate) field_handles: &'a [FieldHandle],
    pub(crate) field_instantiations: &'a [FieldInstantiation],
}

impl<'a> BytecodeTransformer<'a> {
    pub fn new(
        structs: &'a [StructDef],
        struct_instantiations: &'a [StructInstantiation],
        struct_variant_infos: &'a [StructVariantInfo],
        struct_variant_instantiation_infos: &'a [StructVariantInfo],
        field_handles: &'a [FieldHandle],
        field_instantiations: &'a [FieldInstantiation],
    ) -> Self {
        Self {
            use_fast_instructions: true,
            ty_builder: TypeBuilder::with_limits(128, 20), // TODO: get this from config
            structs,
            struct_instantiations,
            struct_variant_infos,
            struct_variant_instantiation_infos,
            field_handles,
            field_instantiations,
        }
    }

    fn transform_vec_len(&self, idx: SignatureIndex) -> PartialVMResult<Instruction> {
        Ok(if self.use_fast_instructions {
            Instruction::VecLenV2
        } else {
            Instruction::VecLen(idx)
        })
    }

    fn transform_test_variant(
        &self,
        idx: StructVariantHandleIndex,
    ) -> PartialVMResult<Instruction> {
        Ok(if self.use_fast_instructions {
            let info = &self.struct_variant_infos[idx.0 as usize];
            Instruction::TestVariantV2(TestVariantV2 {
                variant_idx: info.variant,
                struct_name_idx: info.definition_struct_type.idx,
            })
        } else {
            Instruction::TestVariant(idx)
        })
    }

    fn transform_test_variant_generic(
        &self,
        idx: StructVariantInstantiationIndex,
    ) -> PartialVMResult<Instruction> {
        Ok(if self.use_fast_instructions {
            let info = &self.struct_variant_instantiation_infos[idx.0 as usize];
            Instruction::TestVariantV2(TestVariantV2 {
                variant_idx: info.variant,
                struct_name_idx: info.definition_struct_type.idx,
            })
        } else {
            Instruction::TestVariantGeneric(idx)
        })
    }

    #[allow(clippy::collapsible_else_if)]
    fn transform_borrow_field(
        &self,
        is_mut: bool,
        idx: FieldHandleIndex,
    ) -> PartialVMResult<Instruction> {
        Ok(if self.use_fast_instructions {
            let handle = &self.field_handles[idx.0 as usize];
            Instruction::BorrowFieldV2(Box::new(BorrowFieldV2 {
                is_mut,
                field_offset: handle.offset,
                struct_name_idx: handle.definition_struct_type.idx,
                field_ty: handle.field_ty.clone(),
            }))
        } else if is_mut {
            Instruction::MutBorrowField(idx)
        } else {
            Instruction::ImmBorrowField(idx)
        })
    }

    #[allow(clippy::collapsible_else_if)]
    fn transform_borrow_field_generic(
        &self,
        is_mut: bool,
        idx: FieldInstantiationIndex,
    ) -> PartialVMResult<Instruction> {
        if self.use_fast_instructions {
            let field_inst = &self.field_instantiations[idx.0 as usize];

            // TODO: used cached result -- we're already computing this during module loading
            let is_concrete = field_inst.instantiation.iter().all(|ty| ty.is_concrete());
            if !is_concrete {
                return Ok(if is_mut {
                    Instruction::MutBorrowFieldGeneric(idx)
                } else {
                    Instruction::ImmBorrowFieldGeneric(idx)
                });
            }

            let field_ty = self
                .ty_builder
                .create_ty_with_subst(
                    &field_inst.uninstantiated_field_ty,
                    &field_inst.instantiation,
                )
                .map_err(|e| {
                    PartialVMError::new(StatusCode::TYPE_RESOLUTION_FAILURE)
                        .with_message(format!("Failed to create field type: {}", e))
                })?;

            Ok(Instruction::BorrowFieldV2(Box::new(BorrowFieldV2 {
                is_mut,
                field_offset: field_inst.offset,
                struct_name_idx: field_inst.definition_struct_type.idx,
                field_ty,
            })))
        } else {
            Ok(if is_mut {
                Instruction::MutBorrowFieldGeneric(idx)
            } else {
                Instruction::ImmBorrowFieldGeneric(idx)
            })
        }
    }

    fn transform_pack(&self, idx: StructDefinitionIndex) -> PartialVMResult<Instruction> {
        Ok(if self.use_fast_instructions {
            let struct_def = &self.structs[idx.0 as usize];

            let field_tys = struct_def
                .definition_struct_type
                .fields(None)?
                .iter()
                .map(|(_, ty)| ty.clone())
                .collect();

            let struct_ty = self.ty_builder.create_struct_ty(
                struct_def.definition_struct_type.idx,
                AbilityInfo::struct_(struct_def.definition_struct_type.abilities),
            );

            // TODO: check depth

            Instruction::PackV2(Box::new(PackV2 {
                is_generic: false,
                field_count: struct_def.field_count,
                struct_ty,
                field_tys,
            }))
        } else {
            Instruction::Pack(idx)
        })
    }

    fn transform_pack_generic(
        &self,
        idx: StructDefInstantiationIndex,
    ) -> PartialVMResult<Instruction> {
        if self.use_fast_instructions {
            let struct_inst = &self.struct_instantiations[idx.0 as usize];

            let is_concrete = struct_inst.instantiation.iter().all(|ty| ty.is_concrete());
            if !is_concrete {
                return Ok(Instruction::PackGeneric(idx));
            }

            let mut field_tys = vec![];
            for (_, ty) in struct_inst.definition_struct_type.fields(None)? {
                field_tys.push(
                    self.ty_builder
                        .create_ty_with_subst(ty, &struct_inst.instantiation)
                        .map_err(|e| {
                            PartialVMError::new(StatusCode::TYPE_RESOLUTION_FAILURE)
                                .with_message(format!("Failed to create field type: {}", e))
                        })?,
                );
            }

            // TODO: check depth
            let struct_ty = Type::StructInstantiation {
                idx: struct_inst.definition_struct_type.idx,
                ty_args: triomphe::Arc::new(struct_inst.instantiation.clone()),
                ability: AbilityInfo::generic_struct(
                    struct_inst.definition_struct_type.abilities,
                    struct_inst
                        .definition_struct_type
                        .phantom_ty_params_mask
                        .clone(),
                ),
            };

            Ok(Instruction::PackV2(Box::new(PackV2 {
                is_generic: true,
                field_count: struct_inst.field_count,
                struct_ty,
                field_tys,
            })))
        } else {
            Ok(Instruction::PackGeneric(idx))
        }
    }

    pub fn transform(&self, bytecode: Bytecode) -> PartialVMResult<Instruction> {
        use Bytecode as B;
        use Instruction as I;

        Ok(match bytecode {
            B::Pop => I::Pop,
            B::Ret => I::Ret,
            B::BrTrue(offset) => I::BrTrue(offset),
            B::BrFalse(offset) => I::BrFalse(offset),
            B::Branch(offset) => I::Branch(offset),
            B::LdU8(val) => I::LdU8(val),
            B::LdU64(val) => I::LdU64(val),
            B::LdU128(val) => I::LdU128(Box::new(val)),
            B::CastU8 => I::CastU8,
            B::CastU64 => I::CastU64,
            B::CastU128 => I::CastU128,
            B::LdConst(idx) => I::LdConst(idx),
            B::LdTrue => I::LdTrue,
            B::LdFalse => I::LdFalse,
            B::CopyLoc(idx) => I::CopyLoc(idx),
            B::MoveLoc(idx) => I::MoveLoc(idx),
            B::StLoc(idx) => I::StLoc(idx),
            B::Call(idx) => I::Call(idx),
            B::CallGeneric(idx) => I::CallGeneric(idx),
            B::Pack(idx) => self.transform_pack(idx)?,
            B::PackGeneric(idx) => self.transform_pack_generic(idx)?,
            B::PackVariant(idx) => I::PackVariant(idx),
            B::PackVariantGeneric(idx) => I::PackVariantGeneric(idx),
            B::Unpack(idx) => I::Unpack(idx),
            B::UnpackGeneric(idx) => I::UnpackGeneric(idx),
            B::UnpackVariant(idx) => I::UnpackVariant(idx),
            B::UnpackVariantGeneric(idx) => I::UnpackVariantGeneric(idx),
            B::TestVariant(idx) => self.transform_test_variant(idx)?,
            B::TestVariantGeneric(idx) => self.transform_test_variant_generic(idx)?,
            B::ReadRef => I::ReadRef,
            B::WriteRef => I::WriteRef,
            B::FreezeRef => I::FreezeRef,
            B::MutBorrowLoc(idx) => I::MutBorrowLoc(idx),
            B::ImmBorrowLoc(idx) => I::ImmBorrowLoc(idx),
            B::MutBorrowField(idx) => self.transform_borrow_field(true, idx)?,
            B::MutBorrowVariantField(idx) => I::MutBorrowVariantField(idx),
            B::MutBorrowFieldGeneric(idx) => self.transform_borrow_field_generic(true, idx)?,
            B::MutBorrowVariantFieldGeneric(idx) => I::MutBorrowVariantFieldGeneric(idx),
            B::ImmBorrowField(idx) => self.transform_borrow_field(false, idx)?,
            B::ImmBorrowVariantField(idx) => I::ImmBorrowVariantField(idx),
            B::ImmBorrowFieldGeneric(idx) => self.transform_borrow_field_generic(false, idx)?,
            B::ImmBorrowVariantFieldGeneric(idx) => I::ImmBorrowVariantFieldGeneric(idx),
            B::MutBorrowGlobal(idx) => I::MutBorrowGlobal(idx),
            B::MutBorrowGlobalGeneric(idx) => I::MutBorrowGlobalGeneric(idx),
            B::ImmBorrowGlobal(idx) => I::ImmBorrowGlobal(idx),
            B::ImmBorrowGlobalGeneric(idx) => I::ImmBorrowGlobalGeneric(idx),
            B::Add => I::Add,
            B::Sub => I::Sub,
            B::Mul => I::Mul,
            B::Mod => I::Mod,
            B::Div => I::Div,
            B::BitOr => I::BitOr,
            B::BitAnd => I::BitAnd,
            B::Xor => I::Xor,
            B::Or => I::Or,
            B::And => I::And,
            B::Not => I::Not,
            B::Eq => I::Eq,
            B::Neq => I::Neq,
            B::Lt => I::Lt,
            B::Gt => I::Gt,
            B::Le => I::Le,
            B::Ge => I::Ge,
            B::Abort => I::Abort,
            B::Nop => I::Nop,
            B::Exists(idx) => I::Exists(idx),
            B::ExistsGeneric(idx) => I::ExistsGeneric(idx),
            B::MoveFrom(idx) => I::MoveFrom(idx),
            B::MoveFromGeneric(idx) => I::MoveFromGeneric(idx),
            B::MoveTo(idx) => I::MoveTo(idx),
            B::MoveToGeneric(idx) => I::MoveToGeneric(idx),
            B::Shl => I::Shl,
            B::Shr => I::Shr,
            B::VecPack(idx, n) => I::VecPack(idx, n),
            B::VecLen(idx) => self.transform_vec_len(idx)?,
            B::VecImmBorrow(idx) => I::VecImmBorrow(idx),
            B::VecMutBorrow(idx) => I::VecMutBorrow(idx),
            B::VecPushBack(idx) => I::VecPushBack(idx),
            B::VecPopBack(idx) => I::VecPopBack(idx),
            B::VecUnpack(idx, n) => I::VecUnpack(idx, n),
            B::VecSwap(idx) => I::VecSwap(idx),
            B::PackClosure(idx, mask) => I::PackClosure(idx, mask),
            B::PackClosureGeneric(idx, mask) => I::PackClosureGeneric(idx, mask),
            B::CallClosure(idx) => I::CallClosure(idx),
            B::LdU16(val) => I::LdU16(val),
            B::LdU32(val) => I::LdU32(val),
            B::LdU256(val) => I::LdU256(Box::new(val)),
            B::CastU16 => I::CastU16,
            B::CastU32 => I::CastU32,
            B::CastU256 => I::CastU256,
            B::LdI8(val) => I::LdI8(val),
            B::LdI16(val) => I::LdI16(val),
            B::LdI32(val) => I::LdI32(val),
            B::LdI64(val) => I::LdI64(val),
            B::LdI128(val) => I::LdI128(Box::new(val)),
            B::LdI256(val) => I::LdI256(Box::new(val)),
            B::CastI8 => I::CastI8,
            B::CastI16 => I::CastI16,
            B::CastI32 => I::CastI32,
            B::CastI64 => I::CastI64,
            B::CastI128 => I::CastI128,
            B::CastI256 => I::CastI256,
            B::Negate => I::Negate,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn test_vm_operation_size() {
        let size = size_of::<Instruction>();

        assert_eq!(
            size, 16,
            "VMOperation size should be exactly 16 bytes, but got {} bytes",
            size
        );
    }
}
