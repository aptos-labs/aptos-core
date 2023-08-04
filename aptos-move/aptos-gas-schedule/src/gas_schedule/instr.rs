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
        [nop: InternalGas, "nop", 200],
        // control flow
        [ret: InternalGas, "ret", 1044], // 1044
        [abort: InternalGas, "abort", 1200],

        // Note(Gas): The costs of the branch instructions have been jacked up a bit intentionally
        //            to prevent any single transaction from running for too long.
        [br_true: InternalGas, "br_true", 581], // 2400
        [br_false: InternalGas, "br_false", 558], // 2400
        [branch: InternalGas, "branch", 537], // 1600

        // stack
        [pop: InternalGas, "pop", 856], // 800
        [ld_u8: InternalGas, "ld_u8", 1550], // 1200
        [ld_u16: InternalGas, { 5.. => "ld_u16" }, 1524], // 1200
        [ld_u32: InternalGas, { 5.. => "ld_u32" }, 1492], // 1200
        [ld_u64: InternalGas, "ld_u64", 1701], // 1200
        [ld_u128: InternalGas, "ld_u128", 1507], // 1600
        [ld_u256: InternalGas, { 5.. => "ld_u256" }, 1633], // 1600
        [ld_true: InternalGas, "ld_true", 1550], // 1200
        [ld_false: InternalGas, "ld_false", 1200], // 1200
        [ld_const_base: InternalGas, "ld_const.base", 13000], // 13000
        [
            ld_const_per_byte: InternalGasPerByte,
            "ld_const.per_byte",
            700, // 700
            LD_CONST_PER_BYTE
        ],
        // borrow
        [imm_borrow_loc: InternalGas, "imm_borrow_loc", 6509], // 1200
        [mut_borrow_loc: InternalGas, "mut_borrow_loc", 5997], // 1200
        [imm_borrow_field: InternalGas, "imm_borrow_field", 8635], // 4000
        [mut_borrow_field: InternalGas, "mut_borrow_field", 5997], // 4000
        [
            imm_borrow_field_generic: InternalGas,
            "imm_borrow_field_generic",
            4000 // 4000
        ],
        [
            mut_borrow_field_generic: InternalGas,
            "mut_borrow_field_generic",
            4000 // 4000
        ],
        // locals
        [copy_loc_base: InternalGas, "copy_loc.base", 1600], // 1600
        [
            copy_loc_per_abs_val_unit: InternalGasPerAbstractValueUnit,
            "copy_loc.per_abs_val_unit",
            80 // 80
        ],
        [move_loc_base: InternalGas, "move_loc.base", 2400], // 2400
        [st_loc_base: InternalGas, "st_loc.base", 2400], // 2400
        // call
        [call_base: InternalGas, "call.base", 20301], // 20000
        [call_per_arg: InternalGasPerArg, "call.per_arg", 2000], // 2000
        [call_per_local: InternalGasPerArg, { 1.. => "call.per_local" }, 2000], // 2000
        [call_generic_base: InternalGas, "call_generic.base", 38989], // 20000
        [
            call_generic_per_ty_arg: InternalGasPerArg,
            "call_generic.per_ty_arg",
            2251 // 2000
        ],
        [
            call_generic_per_arg: InternalGasPerArg,
            "call_generic.per_arg",
            2000 // 2000
        ],
        [call_generic_per_local: InternalGasPerArg, { 1.. => "call_generic.per_local" }, 1483], // 2000
        // struct
        [pack_base: InternalGas, "pack.base", 4400], // 4400
        [pack_per_field: InternalGasPerArg, "pack.per_field", 800], // 800
        [pack_generic_base: InternalGas, "pack_generic.base", 4400], // 4400
        [
            pack_generic_per_field: InternalGasPerArg,
            "pack_generic.per_field",
            800 // 800
        ],
        [unpack_base: InternalGas, "unpack.base", 4400], // 4400
        [unpack_per_field: InternalGasPerArg, "unpack.per_field", 800], // 800
        [unpack_generic_base: InternalGas, "unpack_generic.base", 4400], // 4400
        [
            unpack_generic_per_field: InternalGasPerArg,
            "unpack_generic.per_field",
            800 // 800
        ],
        // ref
        [read_ref_base: InternalGas, "read_ref.base", 4000], // 4000
        [
            read_ref_per_abs_val_unit: InternalGasPerAbstractValueUnit,
            "read_ref.per_abs_val_unit",
            80 // 80
        ],
        [write_ref_base: InternalGas, "write_ref.base", 4000], // 4000
        [freeze_ref: InternalGas, "freeze_ref", 200], // 200
        // casting
        [cast_u8: InternalGas, "cast_u8", 2733], // 2400
        [cast_u16: InternalGas, { 5.. => "cast_u16" }, 2724], // 2400
        [cast_u32: InternalGas, { 5.. => "cast_u32" }, 2759], // 2400
        [cast_u64: InternalGas, "cast_u64", 2304], // 2400
        [cast_u128: InternalGas, "cast_u128", 2453], // 2400
        [cast_u256: InternalGas, { 5.. => "cast_u256" }, 3645], // 2400
        // arithmetic
        [add: InternalGas, "add", 3157], // 3200
        [sub: InternalGas, "sub", 3096], // 3200
        [mul: InternalGas, "mul", 3234], // 3200
        [mod_: InternalGas, "mod", 3125], // 3200
        [div: InternalGas, "div", 3231], // 3200
        // bitwise
        [bit_or: InternalGas, "bit_or", 2728], // 3200
        [bit_and: InternalGas, "bit_and", 2669], // 3200
        [xor: InternalGas, "bit_xor", 2728], // 3200
        [shl: InternalGas, "bit_shl", 4537], // 3200
        [shr: InternalGas, "bit_shr", 4241], // 3200
        // boolean
        [or: InternalGas, "or", 1103], // 3200
        [and: InternalGas, "and", 2138], // 3200
        [not: InternalGas, "not", 1679], // 3200
        // comparison
        [lt: InternalGas, "lt", 4220], // 3200
        [gt: InternalGas, "gt", 2946], // 3200
        [le: InternalGas, "le", 2946], // 3200
        [ge: InternalGas, "ge", 2972], // 3200
        [eq_base: InternalGas, "eq.base", 2000], // 2000
        [
            eq_per_abs_val_unit: InternalGasPerAbstractValueUnit,
            "eq.per_abs_val_unit",
            80 // 80
        ],
        [neq_base: InternalGas, "neq.base", 2000], // 2000
        [
            neq_per_abs_val_unit: InternalGasPerAbstractValueUnit,
            "neq.per_abs_val_unit",
            80 // 80
        ],
        // global
        [
            imm_borrow_global_base: InternalGas,
            "imm_borrow_global.base",
            10000 // 10000
        ],
        [
            imm_borrow_global_generic_base: InternalGas,
            "imm_borrow_global_generic.base",
            10000 // 10000
        ],
        [
            mut_borrow_global_base: InternalGas,
            "mut_borrow_global.base",
            10000 // 10000
        ],
        [
            mut_borrow_global_generic_base: InternalGas,
            "mut_borrow_global_generic.base",
            10000 // 10000
        ],
        [exists_base: InternalGas, "exists.base", 5000], // 5000
        [exists_generic_base: InternalGas, "exists_generic.base", 5000], // 5000
        [move_from_base: InternalGas, "move_from.base", 7000], // 7000
        [
            move_from_generic_base: InternalGas,
            "move_from_generic.base",
            7000 // 7000
        ],
        [move_to_base: InternalGas, "move_to.base", 10000], // 10000
        [
            move_to_generic_base: InternalGas,
            "move_to_generic.base",
            10000 // 10000
        ],
        // vec
        [vec_len_base: InternalGas, "vec_len.base", 4803], // 4400
        [vec_imm_borrow_base: InternalGas, "vec_imm_borrow.base", 9381], // 6600
        [vec_mut_borrow_base: InternalGas, "vec_mut_borrow.base", 8330], // 6600
        [vec_push_back_base: InternalGas, "vec_push_back.base", 4807], // 7600
        [vec_pop_back_base: InternalGas, "vec_pop_back.base", 6481], // 5200
        [vec_swap_base: InternalGas, "vec_swap.base", 4749], // 6000
        [vec_pack_base: InternalGas, "vec_pack.base", 12000], // 12000
        [
            vec_pack_per_elem: InternalGasPerArg,
            "vec_pack.per_elem",
            800 // 800
        ],
        [vec_unpack_base: InternalGas, "vec_unpack.base", 10000], // 10000
        [
            vec_unpack_per_expected_elem: InternalGasPerArg,
            "vec_unpack.per_expected_elem",
            938 // 800
        ],
    ]
);
