// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::execution_format::instructions::Bytecode;
use move_binary_format::{
    errors::PartialVMResult,
    file_format::{
        Bytecode as FileFormatBytecode, CodeOffset, ConstantPoolIndex, FieldHandleIndex,
        FieldInstantiationIndex, FunctionHandleIndex, FunctionInstantiationIndex, LocalIndex,
        SignatureIndex, StructDefInstantiationIndex, StructDefinitionIndex,
        StructVariantHandleIndex, StructVariantInstantiationIndex, VariantFieldHandleIndex,
        VariantFieldInstantiationIndex,
    },
};
use move_core_types::function::ClosureMask;

pub trait ExecutionFormatConverter {
    fn convert_pop(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Pop)
    }

    fn convert_copy_loc(&self, idx: &LocalIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::CopyLoc(*idx))
    }

    fn convert_move_loc(&self, idx: &LocalIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::MoveLoc(*idx))
    }

    fn convert_st_loc(&self, idx: &LocalIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::StLoc(*idx))
    }

    fn convert_mut_borrow_loc(&self, idx: &LocalIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::MutBorrowLoc(*idx))
    }

    fn convert_imm_borrow_loc(&self, idx: &LocalIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::ImmBorrowLoc(*idx))
    }

    fn convert_ret(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Ret)
    }

    fn convert_abort(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Abort)
    }

    fn convert_br_true(&self, offset: &CodeOffset) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::BrTrue(*offset))
    }

    fn convert_br_false(&self, offset: &CodeOffset) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::BrFalse(*offset))
    }

    fn convert_branch(&self, offset: &CodeOffset) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Branch(*offset))
    }

    fn convert_nop(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Nop)
    }

    fn convert_call(&self, idx: &FunctionHandleIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Call(*idx))
    }

    fn convert_call_generic(&self, idx: &FunctionInstantiationIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::CallGeneric(*idx))
    }

    fn convert_ld_true(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::LdTrue)
    }

    fn convert_ld_false(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::LdFalse)
    }

    fn convert_ld_u8(&self, value: &u8) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::LdU8(*value))
    }

    fn convert_ld_u16(&self, value: &u16) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::LdU16(*value))
    }

    fn convert_ld_u32(&self, value: &u32) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::LdU32(*value))
    }

    fn convert_ld_u64(&self, value: &u64) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::LdU64(*value))
    }

    fn convert_ld_u128(&self, value: &u128) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::LdU128(*value))
    }

    fn convert_ld_u256(&self, value: &move_core_types::u256::U256) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::LdU256(*value))
    }

    fn convert_ld_const(&self, idx: &ConstantPoolIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::LdConst(*idx))
    }

    fn convert_cast_u8(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::CastU8)
    }

    fn convert_cast_u16(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::CastU16)
    }

    fn convert_cast_u32(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::CastU32)
    }

    fn convert_cast_u64(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::CastU64)
    }

    fn convert_cast_u128(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::CastU128)
    }

    fn convert_cast_u256(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::CastU256)
    }

    fn convert_pack(&self, idx: &StructDefinitionIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Pack(*idx))
    }

    fn convert_pack_generic(&self, idx: &StructDefInstantiationIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::PackGeneric(*idx))
    }

    fn convert_unpack(&self, idx: &StructDefinitionIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Unpack(*idx))
    }

    fn convert_unpack_generic(
        &self,
        idx: &StructDefInstantiationIndex,
    ) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::UnpackGeneric(*idx))
    }

    fn convert_pack_variant(&self, idx: &StructVariantHandleIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::PackVariant(*idx))
    }

    fn convert_pack_variant_generic(
        &self,
        idx: &StructVariantInstantiationIndex,
    ) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::PackVariantGeneric(*idx))
    }

    fn convert_unpack_variant(&self, idx: &StructVariantHandleIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::UnpackVariant(*idx))
    }

    fn convert_unpack_variant_generic(
        &self,
        idx: &StructVariantInstantiationIndex,
    ) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::UnpackVariantGeneric(*idx))
    }

    fn convert_test_variant(&self, idx: &StructVariantHandleIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::TestVariant(*idx))
    }

    fn convert_test_variant_generic(
        &self,
        idx: &StructVariantInstantiationIndex,
    ) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::TestVariantGeneric(*idx))
    }

    fn convert_imm_borrow_field(&self, idx: &FieldHandleIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::ImmBorrowField(*idx))
    }

    fn convert_imm_borrow_field_generic(
        &self,
        idx: &FieldInstantiationIndex,
    ) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::ImmBorrowFieldGeneric(*idx))
    }

    fn convert_mut_borrow_field(&self, idx: &FieldHandleIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::MutBorrowField(*idx))
    }

    fn convert_mut_borrow_field_generic(
        &self,
        idx: &FieldInstantiationIndex,
    ) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::MutBorrowFieldGeneric(*idx))
    }

    fn convert_imm_borrow_variant_field(
        &self,
        idx: &VariantFieldHandleIndex,
    ) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::ImmBorrowVariantField(*idx))
    }

    fn convert_imm_borrow_variant_field_generic(
        &self,
        idx: &VariantFieldInstantiationIndex,
    ) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::ImmBorrowVariantFieldGeneric(*idx))
    }

    fn convert_mut_borrow_variant_field(
        &self,
        idx: &VariantFieldHandleIndex,
    ) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::MutBorrowVariantField(*idx))
    }

    fn convert_mut_borrow_variant_field_generic(
        &self,
        idx: &VariantFieldInstantiationIndex,
    ) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::MutBorrowVariantFieldGeneric(*idx))
    }

    fn convert_read_ref(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::ReadRef)
    }

    fn convert_write_ref(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::WriteRef)
    }

    fn convert_freeze_ref(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::FreezeRef)
    }

    fn convert_move_to(&self, idx: &StructDefinitionIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::MoveTo(*idx))
    }

    fn convert_move_to_generic(
        &self,
        idx: &StructDefInstantiationIndex,
    ) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::MoveToGeneric(*idx))
    }

    fn convert_exists(&self, idx: &StructDefinitionIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Exists(*idx))
    }

    fn convert_exists_generic(
        &self,
        idx: &StructDefInstantiationIndex,
    ) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::ExistsGeneric(*idx))
    }

    fn convert_imm_borrow_global(&self, idx: &StructDefinitionIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::ImmBorrowGlobal(*idx))
    }

    fn convert_imm_borrow_global_generic(
        &self,
        idx: &StructDefInstantiationIndex,
    ) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::ImmBorrowGlobalGeneric(*idx))
    }

    fn convert_mut_borrow_global(&self, idx: &StructDefinitionIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::MutBorrowGlobal(*idx))
    }

    fn convert_mut_borrow_global_generic(
        &self,
        idx: &StructDefInstantiationIndex,
    ) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::MutBorrowGlobalGeneric(*idx))
    }

    fn convert_move_from(&self, idx: &StructDefinitionIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::MoveFrom(*idx))
    }

    fn convert_move_from_generic(
        &self,
        idx: &StructDefInstantiationIndex,
    ) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::MoveFromGeneric(*idx))
    }

    fn convert_add(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Add)
    }

    fn convert_sub(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Sub)
    }

    fn convert_mul(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Mul)
    }

    fn convert_mod(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Mod)
    }

    fn convert_div(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Div)
    }

    fn convert_bit_or(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::BitOr)
    }

    fn convert_bit_and(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::BitAnd)
    }

    fn convert_xor(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Xor)
    }

    fn convert_shl(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Shl)
    }

    fn convert_shr(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Shr)
    }

    fn convert_or(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Or)
    }

    fn convert_and(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::And)
    }

    fn convert_not(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Not)
    }

    fn convert_eq(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Eq)
    }

    fn convert_neq(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Neq)
    }

    fn convert_lt(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Lt)
    }

    fn convert_gt(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Gt)
    }

    fn convert_le(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Le)
    }

    fn convert_ge(&self) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::Ge)
    }

    fn convert_vec_pack(&self, idx: &SignatureIndex, num: &u64) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::VecPack(*idx, *num))
    }

    fn convert_vec_len(&self, idx: &SignatureIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::VecLen(*idx))
    }

    fn convert_vec_imm_borrow(&self, idx: &SignatureIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::VecImmBorrow(*idx))
    }

    fn convert_vec_mut_borrow(&self, idx: &SignatureIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::VecMutBorrow(*idx))
    }

    fn convert_vec_push_back(&self, idx: &SignatureIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::VecPushBack(*idx))
    }

    fn convert_vec_pop_back(&self, idx: &SignatureIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::VecPopBack(*idx))
    }

    fn convert_vec_unpack(&self, idx: &SignatureIndex, num: &u64) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::VecUnpack(*idx, *num))
    }

    fn convert_vec_swap(&self, idx: &SignatureIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::VecSwap(*idx))
    }

    fn convert_pack_closure(
        &self,
        idx: &FunctionHandleIndex,
        mask: &ClosureMask,
    ) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::PackClosure(*idx, *mask))
    }

    fn convert_pack_closure_generic(
        &self,
        idx: &FunctionInstantiationIndex,
        mask: &ClosureMask,
    ) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::PackClosureGeneric(*idx, *mask))
    }

    fn convert_call_closure(&self, idx: &SignatureIndex) -> PartialVMResult<Bytecode> {
        Ok(Bytecode::CallClosure(*idx))
    }

    fn convert_instruction(&self, instr: &FileFormatBytecode) -> PartialVMResult<Bytecode> {
        match instr {
            FileFormatBytecode::Pop => self.convert_pop(),
            FileFormatBytecode::Ret => self.convert_ret(),
            FileFormatBytecode::Abort => self.convert_abort(),
            FileFormatBytecode::BrTrue(offset) => self.convert_br_true(offset),
            FileFormatBytecode::BrFalse(offset) => self.convert_br_false(offset),
            FileFormatBytecode::Branch(offset) => self.convert_branch(offset),
            FileFormatBytecode::LdU8(value) => self.convert_ld_u8(value),
            FileFormatBytecode::LdU16(value) => self.convert_ld_u16(value),
            FileFormatBytecode::LdU32(value) => self.convert_ld_u32(value),
            FileFormatBytecode::LdU64(value) => self.convert_ld_u64(value),
            FileFormatBytecode::LdU128(value) => self.convert_ld_u128(value),
            FileFormatBytecode::LdU256(value) => self.convert_ld_u256(value),
            FileFormatBytecode::CastU8 => self.convert_cast_u8(),
            FileFormatBytecode::CastU16 => self.convert_cast_u16(),
            FileFormatBytecode::CastU32 => self.convert_cast_u32(),
            FileFormatBytecode::CastU64 => self.convert_cast_u64(),
            FileFormatBytecode::CastU128 => self.convert_cast_u128(),
            FileFormatBytecode::CastU256 => self.convert_cast_u256(),
            FileFormatBytecode::LdConst(idx) => self.convert_ld_const(idx),
            FileFormatBytecode::LdTrue => self.convert_ld_true(),
            FileFormatBytecode::LdFalse => self.convert_ld_false(),
            FileFormatBytecode::CopyLoc(idx) => self.convert_copy_loc(idx),
            FileFormatBytecode::MoveLoc(idx) => self.convert_move_loc(idx),
            FileFormatBytecode::StLoc(idx) => self.convert_st_loc(idx),
            FileFormatBytecode::Call(idx) => self.convert_call(idx),
            FileFormatBytecode::CallGeneric(idx) => self.convert_call_generic(idx),
            FileFormatBytecode::Pack(idx) => self.convert_pack(idx),
            FileFormatBytecode::PackGeneric(idx) => self.convert_pack_generic(idx),
            FileFormatBytecode::PackVariant(idx) => self.convert_pack_variant(idx),
            FileFormatBytecode::PackVariantGeneric(idx) => self.convert_pack_variant_generic(idx),
            FileFormatBytecode::Unpack(idx) => self.convert_unpack(idx),
            FileFormatBytecode::UnpackGeneric(idx) => self.convert_unpack_generic(idx),
            FileFormatBytecode::UnpackVariant(idx) => self.convert_unpack_variant(idx),
            FileFormatBytecode::UnpackVariantGeneric(idx) => {
                self.convert_unpack_variant_generic(idx)
            },
            FileFormatBytecode::TestVariant(idx) => self.convert_test_variant(idx),
            FileFormatBytecode::TestVariantGeneric(idx) => self.convert_test_variant_generic(idx),
            FileFormatBytecode::ReadRef => self.convert_read_ref(),
            FileFormatBytecode::WriteRef => self.convert_write_ref(),
            FileFormatBytecode::FreezeRef => self.convert_freeze_ref(),
            FileFormatBytecode::MutBorrowLoc(idx) => self.convert_mut_borrow_loc(idx),
            FileFormatBytecode::ImmBorrowLoc(idx) => self.convert_imm_borrow_loc(idx),
            FileFormatBytecode::MutBorrowField(idx) => self.convert_mut_borrow_field(idx),
            FileFormatBytecode::MutBorrowVariantField(idx) => {
                self.convert_mut_borrow_variant_field(idx)
            },
            FileFormatBytecode::MutBorrowFieldGeneric(idx) => {
                self.convert_mut_borrow_field_generic(idx)
            },
            FileFormatBytecode::MutBorrowVariantFieldGeneric(idx) => {
                self.convert_mut_borrow_variant_field_generic(idx)
            },
            FileFormatBytecode::ImmBorrowField(idx) => self.convert_imm_borrow_field(idx),
            FileFormatBytecode::ImmBorrowVariantField(idx) => {
                self.convert_imm_borrow_variant_field(idx)
            },
            FileFormatBytecode::ImmBorrowFieldGeneric(idx) => {
                self.convert_imm_borrow_field_generic(idx)
            },
            FileFormatBytecode::ImmBorrowVariantFieldGeneric(idx) => {
                self.convert_imm_borrow_variant_field_generic(idx)
            },
            FileFormatBytecode::MutBorrowGlobal(idx) => self.convert_mut_borrow_global(idx),
            FileFormatBytecode::MutBorrowGlobalGeneric(idx) => {
                self.convert_mut_borrow_global_generic(idx)
            },
            FileFormatBytecode::ImmBorrowGlobal(idx) => self.convert_imm_borrow_global(idx),
            FileFormatBytecode::ImmBorrowGlobalGeneric(idx) => {
                self.convert_imm_borrow_global_generic(idx)
            },
            FileFormatBytecode::Add => self.convert_add(),
            FileFormatBytecode::Sub => self.convert_sub(),
            FileFormatBytecode::Mul => self.convert_mul(),
            FileFormatBytecode::Mod => self.convert_mod(),
            FileFormatBytecode::Div => self.convert_div(),
            FileFormatBytecode::BitOr => self.convert_bit_or(),
            FileFormatBytecode::BitAnd => self.convert_bit_and(),
            FileFormatBytecode::Xor => self.convert_xor(),
            FileFormatBytecode::Or => self.convert_or(),
            FileFormatBytecode::And => self.convert_and(),
            FileFormatBytecode::Not => self.convert_not(),
            FileFormatBytecode::Eq => self.convert_eq(),
            FileFormatBytecode::Neq => self.convert_neq(),
            FileFormatBytecode::Lt => self.convert_lt(),
            FileFormatBytecode::Gt => self.convert_gt(),
            FileFormatBytecode::Le => self.convert_le(),
            FileFormatBytecode::Ge => self.convert_ge(),
            FileFormatBytecode::Nop => self.convert_nop(),
            FileFormatBytecode::Exists(idx) => self.convert_exists(idx),
            FileFormatBytecode::ExistsGeneric(idx) => self.convert_exists_generic(idx),
            FileFormatBytecode::MoveFrom(idx) => self.convert_move_from(idx),
            FileFormatBytecode::MoveFromGeneric(idx) => self.convert_move_from_generic(idx),
            FileFormatBytecode::MoveTo(idx) => self.convert_move_to(idx),
            FileFormatBytecode::MoveToGeneric(idx) => self.convert_move_to_generic(idx),
            FileFormatBytecode::Shl => self.convert_shl(),
            FileFormatBytecode::Shr => self.convert_shr(),
            FileFormatBytecode::VecPack(idx, num) => self.convert_vec_pack(idx, num),
            FileFormatBytecode::VecLen(idx) => self.convert_vec_len(idx),
            FileFormatBytecode::VecImmBorrow(idx) => self.convert_vec_imm_borrow(idx),
            FileFormatBytecode::VecMutBorrow(idx) => self.convert_vec_mut_borrow(idx),
            FileFormatBytecode::VecPushBack(idx) => self.convert_vec_push_back(idx),
            FileFormatBytecode::VecPopBack(idx) => self.convert_vec_pop_back(idx),
            FileFormatBytecode::VecUnpack(idx, num) => self.convert_vec_unpack(idx, num),
            FileFormatBytecode::VecSwap(idx) => self.convert_vec_swap(idx),
            FileFormatBytecode::PackClosure(idx, mask) => self.convert_pack_closure(idx, mask),
            FileFormatBytecode::PackClosureGeneric(idx, mask) => {
                self.convert_pack_closure_generic(idx, mask)
            },
            FileFormatBytecode::CallClosure(idx) => self.convert_call_closure(idx),
        }
    }

    fn convert_code(
        &self,
        file_format_code: &[FileFormatBytecode],
    ) -> PartialVMResult<Vec<Bytecode>> {
        let mut code = Vec::with_capacity(file_format_code.len());
        for instr in file_format_code {
            code.push(self.convert_instruction(instr)?);
        }
        Ok(code)
    }
}
