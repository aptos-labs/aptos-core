// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines all the gas parameters and formulae for instructions, along with their
//! initial values in the genesis and a mapping between the Rust representation and the on-chain
//! gas schedule.

use crate::{algebra::InternalGasPerAbstractValueUnit, gas_meter::EXECUTION_GAS_MULTIPLIER as MUL};
use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::{InternalGas, InternalGasPerArg, InternalGasPerByte};
use move_vm_types::gas::SimpleInstruction;

crate::params::define_gas_parameters!(
    InstructionGasParameters,
    "instr",
    [
        // nop
        [nop: InternalGas, "nop", 10 * MUL],
        // control flow
        [ret: InternalGas, "ret", 60 * MUL],
        [abort: InternalGas, "abort", 60 * MUL],

        // Note(Gas): The costs of the branch instructions have been jacked up a bit intentionally
        //            to prevent any single transaction from running for too long.
        [br_true: InternalGas, "br_true", 120 * MUL],
        [br_false: InternalGas, "br_false", 120 * MUL],
        [branch: InternalGas, "branch", 80 * MUL],

        // stack
        [pop: InternalGas, "pop", 40 * MUL],
        [ld_u8: InternalGas, "ld_u8", 60 * MUL],
        [ld_u16: InternalGas, { 5.. => "ld_u16" }, 60 * MUL],
        [ld_u32: InternalGas, { 5.. => "ld_u32" }, 60 * MUL],
        [ld_u64: InternalGas, "ld_u64", 60 * MUL],
        [ld_u128: InternalGas, "ld_u128", 80 * MUL],
        [ld_u256: InternalGas, { 5.. => "ld_u256" }, 80 * MUL],
        [ld_true: InternalGas, "ld_true", 60 * MUL],
        [ld_false: InternalGas, "ld_false", 60 * MUL],
        [ld_const_base: InternalGas, "ld_const.base", 650 * MUL],
        [
            ld_const_per_byte: InternalGasPerByte,
            "ld_const.per_byte",
            35 * MUL
        ],
        // borrow
        [imm_borrow_loc: InternalGas, "imm_borrow_loc", 60 * MUL],
        [mut_borrow_loc: InternalGas, "mut_borrow_loc", 60 * MUL],
        [imm_borrow_field: InternalGas, "imm_borrow_field", 200 * MUL],
        [mut_borrow_field: InternalGas, "mut_borrow_field", 200 * MUL],
        [
            imm_borrow_field_generic: InternalGas,
            "imm_borrow_field_generic",
            200 * MUL
        ],
        [
            mut_borrow_field_generic: InternalGas,
            "mut_borrow_field_generic",
            200 * MUL
        ],
        // locals
        [copy_loc_base: InternalGas, "copy_loc.base", 80 * MUL],
        [
            copy_loc_per_abs_val_unit: InternalGasPerAbstractValueUnit,
            "copy_loc.per_abs_val_unit",
            4 * MUL
        ],
        [move_loc_base: InternalGas, "move_loc.base", 120 * MUL],
        [st_loc_base: InternalGas, "st_loc.base", 120 * MUL],
        // call
        [call_base: InternalGas, "call.base", 1000 * MUL],
        [call_per_arg: InternalGasPerArg, "call.per_arg", 100 * MUL],
        [call_per_local: InternalGasPerArg, { 1.. => "call.per_local" }, 100 * MUL],
        [call_generic_base: InternalGas, "call_generic.base", 1000 * MUL],
        [
            call_generic_per_ty_arg: InternalGasPerArg,
            "call_generic.per_ty_arg",
            100 * MUL
        ],
        [
            call_generic_per_arg: InternalGasPerArg,
            "call_generic.per_arg",
            100 * MUL
        ],
        [call_generic_per_local: InternalGasPerArg, { 1.. => "call_generic.per_local" }, 100 * MUL],
        // struct
        [pack_base: InternalGas, "pack.base", 220 * MUL],
        [pack_per_field: InternalGasPerArg, "pack.per_field", 40 * MUL],
        [pack_generic_base: InternalGas, "pack_generic.base", 220 * MUL],
        [
            pack_generic_per_field: InternalGasPerArg,
            "pack_generic.per_field",
            40 * MUL
        ],
        [unpack_base: InternalGas, "unpack.base", 220 * MUL],
        [unpack_per_field: InternalGasPerArg, "unpack.per_field", 40 * MUL],
        [unpack_generic_base: InternalGas, "unpack_generic.base", 220 * MUL],
        [
            unpack_generic_per_field: InternalGasPerArg,
            "unpack_generic.per_field",
            40 * MUL
        ],
        // ref
        [read_ref_base: InternalGas, "read_ref.base", 200 * MUL],
        [
            read_ref_per_abs_val_unit: InternalGasPerAbstractValueUnit,
            "read_ref.per_abs_val_unit",
            4 * MUL
        ],
        [write_ref_base: InternalGas, "write_ref.base", 200 * MUL],
        [freeze_ref: InternalGas, "freeze_ref", 10 * MUL],
        // casting
        [cast_u8: InternalGas, "cast_u8", 120 * MUL],
        [cast_u16: InternalGas, { 5.. => "cast_u16" }, 120 * MUL],
        [cast_u32: InternalGas, { 5.. => "cast_u32" }, 120 * MUL],
        [cast_u64: InternalGas, "cast_u64", 120 * MUL],
        [cast_u128: InternalGas, "cast_u128", 120 * MUL],
        [cast_u256: InternalGas, { 5.. => "cast_u256" }, 120 * MUL],
        // arithmetic
        [add: InternalGas, "add", 160 * MUL],
        [sub: InternalGas, "sub", 160 * MUL],
        [mul: InternalGas, "mul", 160 * MUL],
        [mod_: InternalGas, "mod", 160 * MUL],
        [div: InternalGas, "div", 160 * MUL],
        // bitwise
        [bit_or: InternalGas, "bit_or", 160 * MUL],
        [bit_and: InternalGas, "bit_and", 160 * MUL],
        [xor: InternalGas, "bit_xor", 160 * MUL],
        [shl: InternalGas, "bit_shl", 160 * MUL],
        [shr: InternalGas, "bit_shr", 160 * MUL],
        // boolean
        [or: InternalGas, "or", 160 * MUL],
        [and: InternalGas, "and", 160 * MUL],
        [not: InternalGas, "not", 160 * MUL],
        // comparison
        [lt: InternalGas, "lt", 160 * MUL],
        [gt: InternalGas, "gt", 160 * MUL],
        [le: InternalGas, "le", 160 * MUL],
        [ge: InternalGas, "ge", 160 * MUL],
        [eq_base: InternalGas, "eq.base", 100 * MUL],
        [
            eq_per_abs_val_unit: InternalGasPerAbstractValueUnit,
            "eq.per_abs_val_unit",
            4 * MUL
        ],
        [neq_base: InternalGas, "neq.base", 100 * MUL],
        [
            neq_per_abs_val_unit: InternalGasPerAbstractValueUnit,
            "neq.per_abs_val_unit",
            4 * MUL
        ],
        // global
        [
            imm_borrow_global_base: InternalGas,
            "imm_borrow_global.base",
            500 * MUL
        ],
        [
            imm_borrow_global_generic_base: InternalGas,
            "imm_borrow_global_generic.base",
            500 * MUL
        ],
        [
            mut_borrow_global_base: InternalGas,
            "mut_borrow_global.base",
            500 * MUL
        ],
        [
            mut_borrow_global_generic_base: InternalGas,
            "mut_borrow_global_generic.base",
            500 * MUL
        ],
        [exists_base: InternalGas, "exists.base", 250 * MUL],
        [exists_generic_base: InternalGas, "exists_generic.base", 250 * MUL],
        [move_from_base: InternalGas, "move_from.base", 350 * MUL],
        [
            move_from_generic_base: InternalGas,
            "move_from_generic.base",
            350 * MUL
        ],
        [move_to_base: InternalGas, "move_to.base", 500 * MUL],
        [
            move_to_generic_base: InternalGas,
            "move_to_generic.base",
            500 * MUL
        ],
        // vec
        [vec_len_base: InternalGas, "vec_len.base", 220 * MUL],
        [vec_imm_borrow_base: InternalGas, "vec_imm_borrow.base", 330 * MUL],
        [vec_mut_borrow_base: InternalGas, "vec_mut_borrow.base", 330 * MUL],
        [vec_push_back_base: InternalGas, "vec_push_back.base", 380 * MUL],
        [vec_pop_back_base: InternalGas, "vec_pop_back.base", 260 * MUL],
        [vec_swap_base: InternalGas, "vec_swap.base", 300 * MUL],
        [vec_pack_base: InternalGas, "vec_pack.base", 600 * MUL],
        [
            vec_pack_per_elem: InternalGasPerArg,
            "vec_pack.per_elem",
            40 * MUL
        ],
        [vec_unpack_base: InternalGas, "vec_unpack.base", 500 * MUL],
        [
            vec_unpack_per_expected_elem: InternalGasPerArg,
            "vec_unpack.per_expected_elem",
            40 * MUL
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

            LdU8 => self.ld_u8,
            LdU16 => self.ld_u16,
            LdU32 => self.ld_u32,
            LdU64 => self.ld_u64,
            LdU128 => self.ld_u128,
            LdU256 => self.ld_u256,
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
            CastU16 => self.cast_u16,
            CastU32 => self.cast_u32,
            CastU64 => self.cast_u64,
            CastU128 => self.cast_u128,
            CastU256 => self.cast_u256,

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
