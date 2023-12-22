// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines the gas parameters for all Move instructions.

use crate::gas_schedule::VMGasParameters;
use aptos_gas_algebra::{
    InternalGas, InternalGasPerAbstractValueUnit, InternalGasPerArg, InternalGasPerByte,
};

crate::gas_schedule::macros::define_gas_parameters!(
    InstructionGasParameters,
    "instr",
    VMGasParameters => .instr,
    [
        // nop
        [nop: InternalGas, "nop", 41],
        // control flow
        [ret: InternalGas, "ret", 250],
        [abort: InternalGas, "abort", 250],

        // Note(Gas): The costs of the branch instructions have been jacked up a bit intentionally
        //            to prevent any single transaction from running for too long.
        [br_true: InternalGas, "br_true", 501],
        [br_false: InternalGas, "br_false", 501],
        [branch: InternalGas, "branch", 334],

        // stack
        [pop: InternalGas, "pop", 167],
        [ld_u8: InternalGas, "ld_u8", 250],
        [ld_u16: InternalGas, { 5.. => "ld_u16" }, 250],
        [ld_u32: InternalGas, { 5.. => "ld_u32" }, 250],
        [ld_u64: InternalGas, "ld_u64", 250],
        [ld_u128: InternalGas, "ld_u128", 334],
        [ld_u256: InternalGas, { 5.. => "ld_u256" }, 334],
        [ld_true: InternalGas, "ld_true", 250],
        [ld_false: InternalGas, "ld_false", 250],
        [ld_const_base: InternalGas, "ld_const.base", 2717],
        [ld_const_per_byte: InternalGasPerByte, "ld_const.per_byte", 146],
        // borrow
        [imm_borrow_loc: InternalGas, "imm_borrow_loc", 250],
        [mut_borrow_loc: InternalGas, "mut_borrow_loc", 250],
        [imm_borrow_field: InternalGas, "imm_borrow_field", 836],
        [mut_borrow_field: InternalGas, "mut_borrow_field", 836],
        [imm_borrow_field_generic: InternalGas, "imm_borrow_field_generic", 836],
        [mut_borrow_field_generic: InternalGas, "mut_borrow_field_generic", 836],
        // locals
        [copy_loc_base: InternalGas, "copy_loc.base", 334],
        [copy_loc_per_abs_val_unit: InternalGasPerAbstractValueUnit, "copy_loc.per_abs_val_unit", 16],
        [move_loc_base: InternalGas, "move_loc.base", 501],
        [st_loc_base: InternalGas, "st_loc.base", 501],
        // call
        [call_base: InternalGas, "call.base", 4180],
        [call_per_arg: InternalGasPerArg, "call.per_arg", 418],
        [call_per_local: InternalGasPerArg, { 1.. => "call.per_local" }, 418],
        [call_generic_base: InternalGas, "call_generic.base", 4180],
        [call_generic_per_ty_arg: InternalGasPerArg, "call_generic.per_ty_arg", 418],
        [call_generic_per_arg: InternalGasPerArg, "call_generic.per_arg", 418],
        [call_generic_per_local: InternalGasPerArg, { 1.. => "call_generic.per_local" }, 418],
        // struct
        [pack_base: InternalGas, "pack.base", 919],
        [pack_per_field: InternalGasPerArg, "pack.per_field", 167],
        [pack_generic_base: InternalGas, "pack_generic.base", 919],
        [pack_generic_per_field: InternalGasPerArg, "pack_generic.per_field", 167],
        [unpack_base: InternalGas, "unpack.base", 919],
        [unpack_per_field: InternalGasPerArg, "unpack.per_field", 167],
        [unpack_generic_base: InternalGas, "unpack_generic.base", 919],
        [unpack_generic_per_field: InternalGasPerArg, "unpack_generic.per_field", 167],
        // ref
        [read_ref_base: InternalGas, "read_ref.base", 836],
        [read_ref_per_abs_val_unit: InternalGasPerAbstractValueUnit, "read_ref.per_abs_val_unit", 16],
        [write_ref_base: InternalGas, "write_ref.base", 836],
        [freeze_ref: InternalGas, "freeze_ref", 41],
        // casting
        [cast_u8: InternalGas, "cast_u8", 501],
        [cast_u16: InternalGas, { 5.. => "cast_u16" }, 501],
        [cast_u32: InternalGas, { 5.. => "cast_u32" }, 501],
        [cast_u64: InternalGas, "cast_u64", 501],
        [cast_u128: InternalGas, "cast_u128", 501],
        [cast_u256: InternalGas, { 5.. => "cast_u256" }, 501],
        // arithmetic
        [add: InternalGas, "add", 668],
        [sub: InternalGas, "sub", 668],
        [mul: InternalGas, "mul", 668],
        [mod_: InternalGas, "mod", 668],
        [div: InternalGas, "div", 668],
        // bitwise
        [bit_or: InternalGas, "bit_or", 668],
        [bit_and: InternalGas, "bit_and", 668],
        [xor: InternalGas, "bit_xor", 668],
        [shl: InternalGas, "bit_shl", 668],
        [shr: InternalGas, "bit_shr", 668],
        // boolean
        [or: InternalGas, "or", 668],
        [and: InternalGas, "and", 668],
        [not: InternalGas, "not", 668],
        // comparison
        [lt: InternalGas, "lt", 668],
        [gt: InternalGas, "gt", 668],
        [le: InternalGas, "le", 668],
        [ge: InternalGas, "ge", 668],
        [eq_base: InternalGas, "eq.base", 418],
        [eq_per_abs_val_unit: InternalGasPerAbstractValueUnit, "eq.per_abs_val_unit", 16],
        [neq_base: InternalGas, "neq.base", 418],
        [neq_per_abs_val_unit: InternalGasPerAbstractValueUnit, "neq.per_abs_val_unit", 16],
        // global
        [imm_borrow_global_base: InternalGas, "imm_borrow_global.base", 2090],
        [imm_borrow_global_generic_base: InternalGas, "imm_borrow_global_generic.base", 2090],
        [mut_borrow_global_base: InternalGas, "mut_borrow_global.base", 2090],
        [mut_borrow_global_generic_base: InternalGas, "mut_borrow_global_generic.base", 2090],
        [exists_base: InternalGas, "exists.base", 1045],
        [exists_generic_base: InternalGas, "exists_generic.base", 1045],
        [move_from_base: InternalGas, "move_from.base", 1463],
        [move_from_generic_base: InternalGas, "move_from_generic.base", 1463],
        [move_to_base: InternalGas, "move_to.base", 2090],
        [move_to_generic_base: InternalGas, "move_to_generic.base", 2090],
        // vec
        [vec_len_base: InternalGas, "vec_len.base", 919],
        [vec_imm_borrow_base: InternalGas, "vec_imm_borrow.base", 1379],
        [vec_mut_borrow_base: InternalGas, "vec_mut_borrow.base", 1379],
        [vec_push_back_base: InternalGas, "vec_push_back.base", 1588],
        [vec_pop_back_base: InternalGas, "vec_pop_back.base", 1086],
        [vec_swap_base: InternalGas, "vec_swap.base", 1254],
        [vec_pack_base: InternalGas, "vec_pack.base", 2508],
        [vec_pack_per_elem: InternalGasPerArg, "vec_pack.per_elem", 167],
        [vec_unpack_base: InternalGas, "vec_unpack.base", 2090],
        [vec_unpack_per_expected_elem: InternalGasPerArg, "vec_unpack.per_expected_elem", 167],
    ]
);
