// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module defines all the gas parameters and formulae for instructions, along with their
//! initial values in the genesis and a mapping between the Rust representation and the on-chain
//! gas schedule.

use crate::algebra::InternalGasPerAbstractValueUnit;
use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::{InternalGas, InternalGasPerArg, InternalGasPerByte};
use move_vm_types::gas::SimpleInstruction;

crate::params::define_gas_parameters!(
    InstructionGasParameters,
    "instr",
    [
        // nop
        [nop: InternalGas, "nop", 1],
        // control flow
        [ret: InternalGas, "ret", 1],
        [abort: InternalGas, "abort", 1],
        [br_true: InternalGas, "br_true", 1],
        [br_false: InternalGas, "br_false", 1],
        [branch: InternalGas, "branch", 1],
        // stack
        [pop: InternalGas, "pop", 1],
        [ld_u8: InternalGas, "ld_u8", 1],
        [ld_u64: InternalGas, "ld_u64", 1],
        [ld_u128: InternalGas, "ld_u128", 1],
        [ld_true: InternalGas, "ld_true", 1],
        [ld_false: InternalGas, "ld_false", 1],
        [ld_const_base: InternalGas, "ld_const.base", 1],
        [
            ld_const_per_byte: InternalGasPerByte,
            "ld_const.per_byte",
            1
        ],
        // borrow
        [imm_borrow_loc: InternalGas, "imm_borrow_loc", 1],
        [mut_borrow_loc: InternalGas, "mut_borrow_loc", 1],
        [imm_borrow_field: InternalGas, "imm_borrow_field", 1],
        [mut_borrow_field: InternalGas, "mut_borrow_field", 1],
        [
            imm_borrow_field_generic: InternalGas,
            "imm_borrow_field_generic",
            1
        ],
        [
            mut_borrow_field_generic: InternalGas,
            "mut_borrow_field_generic",
            1
        ],
        // locals
        [copy_loc_base: InternalGas, "copy_loc.base", 1],
        [
            copy_loc_per_abs_val_unit: InternalGasPerAbstractValueUnit,
            "copy_loc.per_abs_val_unit",
            1
        ],
        [move_loc_base: InternalGas, "move_loc.base", 1],
        [st_loc_base: InternalGas, "st_loc.base", 1],
        // call
        [call_base: InternalGas, "call.base", 1],
        [call_per_arg: InternalGasPerArg, "call.per_arg", 1],
        [call_generic_base: InternalGas, "call_generic.base", 1],
        [
            call_generic_per_ty_arg: InternalGasPerArg,
            "call_generic.per_ty_arg",
            1
        ],
        [
            call_generic_per_arg: InternalGasPerArg,
            "call_generic.per_arg",
            1
        ],
        // struct
        [pack_base: InternalGas, "pack.base", 1],
        [pack_per_field: InternalGasPerArg, "pack.per_field", 1],
        [pack_generic_base: InternalGas, "pack_generic.base", 1],
        [
            pack_generic_per_field: InternalGasPerArg,
            "pack_generic.per_field",
            1
        ],
        [unpack_base: InternalGas, "unpack.base", 1],
        [unpack_per_field: InternalGasPerArg, "unpack.per_field", 1],
        [unpack_generic_base: InternalGas, "unpack_generic.base", 1],
        [
            unpack_generic_per_field: InternalGasPerArg,
            "unpack_generic.per_field",
            1
        ],
        // ref
        [read_ref_base: InternalGas, "read_ref.base", 1],
        [
            read_ref_per_abs_val_unit: InternalGasPerAbstractValueUnit,
            "read_ref.per_abs_val_unit",
            1
        ],
        [write_ref_base: InternalGas, "write_ref.base", 1],
        [freeze_ref: InternalGas, "freeze_ref", 1],
        // casting
        [cast_u8: InternalGas, "cast_u8", 1],
        [cast_u64: InternalGas, "cast_u64", 1],
        [cast_u128: InternalGas, "cast_u128", 1],
        // arithmetic
        [add: InternalGas, "add", 1],
        [sub: InternalGas, "sub", 1],
        [mul: InternalGas, "mul", 1],
        [mod_: InternalGas, "mod", 1],
        [div: InternalGas, "div", 1],
        // bitwise
        [bit_or: InternalGas, "bit_or", 1],
        [bit_and: InternalGas, "bit_and", 1],
        [xor: InternalGas, "bit_xor", 1],
        [shl: InternalGas, "bit_shl", 1],
        [shr: InternalGas, "bit_shr", 1],
        // boolean
        [or: InternalGas, "or", 1],
        [and: InternalGas, "and", 1],
        [not: InternalGas, "not", 1],
        // comparison
        [lt: InternalGas, "lt", 1],
        [gt: InternalGas, "gt", 1],
        [le: InternalGas, "le", 1],
        [ge: InternalGas, "ge", 1],
        [eq_base: InternalGas, "eq.base", 1],
        [
            eq_per_abs_val_unit: InternalGasPerAbstractValueUnit,
            "eq.per_abs_val_unit",
            1
        ],
        [neq_base: InternalGas, "neq.base", 1],
        [
            neq_per_abs_val_unit: InternalGasPerAbstractValueUnit,
            "neq.per_abs_val_unit",
            1
        ],
        // global
        [
            imm_borrow_global_base: InternalGas,
            "imm_borrow_global.base",
            10
        ],
        [
            imm_borrow_global_generic_base: InternalGas,
            "imm_borrow_global_generic.base",
            10
        ],
        [
            mut_borrow_global_base: InternalGas,
            "mut_borrow_global.base",
            10
        ],
        [
            mut_borrow_global_generic_base: InternalGas,
            "mut_borrow_global_generic.base",
            10
        ],
        [exists_base: InternalGas, "exists.base", 10],
        [exists_generic_base: InternalGas, "exists_generic.base", 10],
        [move_from_base: InternalGas, "move_from.base", 10],
        [
            move_from_generic_base: InternalGas,
            "move_from_generic.base",
            10
        ],
        [move_to_base: InternalGas, "move_to.base", 10],
        [
            move_to_generic_base: InternalGas,
            "move_to_generic.base",
            10
        ],
        // vec
        [vec_len_base: InternalGas, "vec_len.base", 1],
        [vec_imm_borrow_base: InternalGas, "vec_imm_borrow.base", 1],
        [vec_mut_borrow_base: InternalGas, "vec_mut_borrow.base", 1],
        [vec_push_back_base: InternalGas, "vec_push_back.base", 1],
        [vec_pop_back_base: InternalGas, "vec_pop_back.base", 1],
        [vec_swap_base: InternalGas, "vec_swap.base", 1],
        [vec_pack_base: InternalGas, "vec_pack.base", 1],
        [vec_pack_per_elem: InternalGasPerArg, "vec_pack.per_elem", 1],
        [vec_unpack_base: InternalGas, "vec_unpack.base", 1],
        [
            vec_unpack_per_expected_elem: InternalGasPerArg,
            "vec_unpack.per_expected_elem",
            1
        ],
    ]
);

impl InstructionGasParameters {
    pub(crate) fn simple_instr_cost(
        &self,
        instr: SimpleInstruction,
    ) -> PartialVMResult<InternalGas> {
        use SimpleInstruction::*;

        Ok(match instr {
            Nop => self.nop,

            Abort => self.abort,
            Ret => self.ret,

            BrTrue => self.br_true,
            BrFalse => self.br_false,
            Branch => self.branch,

            Pop => self.pop,
            LdU8 => self.ld_u8,
            LdU64 => self.ld_u64,
            LdU128 => self.ld_u128,
            LdTrue => self.ld_true,
            LdFalse => self.ld_false,

            ImmBorrowLoc => self.imm_borrow_loc,
            MutBorrowLoc => self.mut_borrow_loc,
            ImmBorrowField => self.imm_borrow_field,
            MutBorrowField => self.mut_borrow_field,
            ImmBorrowFieldGeneric => self.imm_borrow_field_generic,
            MutBorrowFieldGeneric => self.mut_borrow_field_generic,
            FreezeRef => self.freeze_ref,

            CastU8 => self.cast_u8,
            CastU64 => self.cast_u64,
            CastU128 => self.cast_u128,

            Add => self.add,
            Sub => self.sub,
            Mul => self.mul,
            Mod => self.mod_,
            Div => self.div,

            BitOr => self.bit_or,
            BitAnd => self.bit_and,
            Xor => self.xor,
            Shl => self.shl,
            Shr => self.shr,

            Or => self.or,
            And => self.and,
            Not => self.not,

            Lt => self.lt,
            Gt => self.gt,
            Le => self.le,
            Ge => self.ge,
        })
    }
}
