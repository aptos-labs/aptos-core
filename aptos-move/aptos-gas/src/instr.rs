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
        [nop: InternalGas, "nop", 10],
        // control flow
        [ret: InternalGas, "ret", 60],
        [abort: InternalGas, "abort", 60],
        [br_true: InternalGas, "br_true", 60],
        [br_false: InternalGas, "br_false", 60],
        [branch: InternalGas, "branch", 20],
        // stack
        [pop: InternalGas, "pop", 40],
        [ld_u8: InternalGas, "ld_u8", 60],
        [ld_u64: InternalGas, "ld_u64", 60],
        [ld_u128: InternalGas, "ld_u128", 80],
        [ld_true: InternalGas, "ld_true", 60],
        [ld_false: InternalGas, "ld_false", 60],
        [ld_const_base: InternalGas, "ld_const.base", 650],
        [
            ld_const_per_byte: InternalGasPerByte,
            "ld_const.per_byte",
            35
        ],
        // borrow
        [imm_borrow_loc: InternalGas, "imm_borrow_loc", 60],
        [mut_borrow_loc: InternalGas, "mut_borrow_loc", 60],
        [imm_borrow_field: InternalGas, "imm_borrow_field", 200],
        [mut_borrow_field: InternalGas, "mut_borrow_field", 200],
        [
            imm_borrow_field_generic: InternalGas,
            "imm_borrow_field_generic",
            200
        ],
        [
            mut_borrow_field_generic: InternalGas,
            "mut_borrow_field_generic",
            200
        ],
        // locals
        [copy_loc_base: InternalGas, "copy_loc.base", 80],
        [
            copy_loc_per_abs_val_unit: InternalGasPerAbstractValueUnit,
            "copy_loc.per_abs_val_unit",
            4
        ],
        [move_loc_base: InternalGas, "move_loc.base", 120],
        [st_loc_base: InternalGas, "st_loc.base", 120],
        // call
        [call_base: InternalGas, "call.base", 1500],
        [call_per_arg: InternalGasPerArg, "call.per_arg", 100],
        [call_generic_base: InternalGas, "call_generic.base", 1500],
        [
            call_generic_per_ty_arg: InternalGasPerArg,
            "call_generic.per_ty_arg",
            100
        ],
        [
            call_generic_per_arg: InternalGasPerArg,
            "call_generic.per_arg",
            100
        ],
        // struct
        [pack_base: InternalGas, "pack.base", 220],
        [pack_per_field: InternalGasPerArg, "pack.per_field", 40],
        [pack_generic_base: InternalGas, "pack_generic.base", 220],
        [
            pack_generic_per_field: InternalGasPerArg,
            "pack_generic.per_field",
            40
        ],
        [unpack_base: InternalGas, "unpack.base", 220],
        [unpack_per_field: InternalGasPerArg, "unpack.per_field", 40],
        [unpack_generic_base: InternalGas, "unpack_generic.base", 220],
        [
            unpack_generic_per_field: InternalGasPerArg,
            "unpack_generic.per_field",
            40
        ],
        // ref
        [read_ref_base: InternalGas, "read_ref.base", 200],
        [
            read_ref_per_abs_val_unit: InternalGasPerAbstractValueUnit,
            "read_ref.per_abs_val_unit",
            4
        ],
        [write_ref_base: InternalGas, "write_ref.base", 200],
        [freeze_ref: InternalGas, "freeze_ref", 10],
        // casting
        [cast_u8: InternalGas, "cast_u8", 120],
        [cast_u64: InternalGas, "cast_u64", 120],
        [cast_u128: InternalGas, "cast_u128", 120],
        // arithmetic
        [add: InternalGas, "add", 160],
        [sub: InternalGas, "sub", 160],
        [mul: InternalGas, "mul", 160],
        [mod_: InternalGas, "mod", 160],
        [div: InternalGas, "div", 160],
        // bitwise
        [bit_or: InternalGas, "bit_or", 160],
        [bit_and: InternalGas, "bit_and", 160],
        [xor: InternalGas, "bit_xor", 160],
        [shl: InternalGas, "bit_shl", 160],
        [shr: InternalGas, "bit_shr", 160],
        // boolean
        [or: InternalGas, "or", 160],
        [and: InternalGas, "and", 160],
        [not: InternalGas, "not", 160],
        // comparison
        [lt: InternalGas, "lt", 160],
        [gt: InternalGas, "gt", 160],
        [le: InternalGas, "le", 160],
        [ge: InternalGas, "ge", 160],
        [eq_base: InternalGas, "eq.base", 100],
        [
            eq_per_abs_val_unit: InternalGasPerAbstractValueUnit,
            "eq.per_abs_val_unit",
            4
        ],
        [neq_base: InternalGas, "neq.base", 100],
        [
            neq_per_abs_val_unit: InternalGasPerAbstractValueUnit,
            "neq.per_abs_val_unit",
            4
        ],
        // global
        [
            imm_borrow_global_base: InternalGas,
            "imm_borrow_global.base",
            500
        ],
        [
            imm_borrow_global_generic_base: InternalGas,
            "imm_borrow_global_generic.base",
            500
        ],
        [
            mut_borrow_global_base: InternalGas,
            "mut_borrow_global.base",
            500
        ],
        [
            mut_borrow_global_generic_base: InternalGas,
            "mut_borrow_global_generic.base",
            500
        ],
        [exists_base: InternalGas, "exists.base", 250],
        [exists_generic_base: InternalGas, "exists_generic.base", 250],
        [move_from_base: InternalGas, "move_from.base", 350],
        [
            move_from_generic_base: InternalGas,
            "move_from_generic.base",
            350
        ],
        [move_to_base: InternalGas, "move_to.base", 500],
        [
            move_to_generic_base: InternalGas,
            "move_to_generic.base",
            500
        ],
        // vec
        [vec_len_base: InternalGas, "vec_len.base", 220],
        [vec_imm_borrow_base: InternalGas, "vec_imm_borrow.base", 330],
        [vec_mut_borrow_base: InternalGas, "vec_mut_borrow.base", 330],
        [vec_push_back_base: InternalGas, "vec_push_back.base", 380],
        [vec_pop_back_base: InternalGas, "vec_pop_back.base", 260],
        [vec_swap_base: InternalGas, "vec_swap.base", 300],
        [vec_pack_base: InternalGas, "vec_pack.base", 600],
        [
            vec_pack_per_elem: InternalGasPerArg,
            "vec_pack.per_elem",
            40
        ],
        [vec_unpack_base: InternalGas, "vec_unpack.base", 500],
        [
            vec_unpack_per_expected_elem: InternalGasPerArg,
            "vec_unpack.per_expected_elem",
            40
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
