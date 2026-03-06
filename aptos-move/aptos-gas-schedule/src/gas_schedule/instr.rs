// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module defines the gas parameters for all Move instructions.

use crate::{
    gas_feature_versions::{RELEASE_V1_18, RELEASE_V1_33, RELEASE_V1_38, RELEASE_V1_40},
    gas_schedule::VMGasParameters,
};
use aptos_gas_algebra::{
    InternalGas, InternalGasPerAbstractValueUnit, InternalGasPerArg, InternalGasPerByte,
    InternalGasPerTypeNode,
};

crate::gas_schedule::macros::define_gas_parameters!(
    InstructionGasParameters,
    "instr",
    VMGasParameters => .instr,
    [
        // nop
        [nop: InternalGas, "nop", 360],
        // control flow
        [ret: InternalGas, "ret", 2200],
        [abort: InternalGas, "abort", 2200],
        [abort_msg_base: InternalGas, { RELEASE_V1_40.. => "abort_msg.base" }, 4400],
        [abort_msg_per_byte: InternalGasPerByte, { RELEASE_V1_40.. => "abort_msg.per_byte" }, 450],

        // Note(Gas): The costs of the branch instructions have been jacked up a bit intentionally
        //            to prevent any single transaction from running for too long.
        [br_true: InternalGas, "br_true", 4410],
        [br_false: InternalGas, "br_false", 4410],
        [branch: InternalGas, "branch", 2940],

        // stack
        [pop: InternalGas, "pop", 1470],
        [ld_u8: InternalGas, "ld_u8", 2200],
        [ld_u16: InternalGas, { 5.. => "ld_u16" }, 2200],
        [ld_u32: InternalGas, { 5.. => "ld_u32" }, 2200],
        [ld_u64: InternalGas, "ld_u64", 2200],
        [ld_u128: InternalGas, "ld_u128", 2940],
        [ld_u256: InternalGas, { 5.. => "ld_u256" }, 2940],
        [ld_i8: InternalGas, { RELEASE_V1_38.. => "ld_i8" }, 2200],
        [ld_i16: InternalGas, { RELEASE_V1_38.. => "ld_i16" }, 2200],
        [ld_i32: InternalGas, { RELEASE_V1_38.. => "ld_i32" }, 2200],
        [ld_i64: InternalGas, { RELEASE_V1_38.. => "ld_i64" }, 2200],
        [ld_i128: InternalGas, { RELEASE_V1_38.. => "ld_i128" }, 2940],
        [ld_i256: InternalGas, { RELEASE_V1_38.. => "ld_i256" }, 2940],
        [ld_true: InternalGas, "ld_true", 2200],
        [ld_false: InternalGas, "ld_false", 2200],
        [ld_const_base: InternalGas, "ld_const.base", 23890],
        [ld_const_per_byte: InternalGasPerByte, "ld_const.per_byte", 1280],
        // borrow
        [imm_borrow_loc: InternalGas, "imm_borrow_loc", 2200],
        [mut_borrow_loc: InternalGas, "mut_borrow_loc", 2200],
        [imm_borrow_field: InternalGas, "imm_borrow_field", 7350],
        [mut_borrow_field: InternalGas, "mut_borrow_field", 7350],
        [imm_borrow_field_generic: InternalGas, "imm_borrow_field_generic" , 7350],
        [mut_borrow_field_generic: InternalGas, "mut_borrow_field_generic", 7350],
        [imm_borrow_variant_field: InternalGas,
            { RELEASE_V1_18.. => "imm_borrow_variant_field" }, 8350],
        [mut_borrow_variant_field: InternalGas,
            { RELEASE_V1_18.. => "mut_borrow_variant_field" }, 8350],
        [imm_borrow_variant_field_generic: InternalGas,
            { RELEASE_V1_18.. => "imm_borrow_variant_field_generic" }, 8350],
        [mut_borrow_variant_field_generic: InternalGas,
            { RELEASE_V1_18.. => "mut_borrow_variant_field_generic" }, 8350],

        // variant testing
        [test_variant: InternalGas,
            { RELEASE_V1_18.. => "test_variant" }, 5350],
        [test_variant_generic: InternalGas,
            { RELEASE_V1_18.. => "test_variant_generic" }, 5350],

        // locals
        [copy_loc_base: InternalGas, "copy_loc.base", 2940],
        [copy_loc_per_abs_val_unit: InternalGasPerAbstractValueUnit, "copy_loc.per_abs_val_unit", 140],
        [move_loc_base: InternalGas, "move_loc.base", 4410],
        [st_loc_base: InternalGas, "st_loc.base", 4410],
        // call
        [call_base: InternalGas, "call.base", 36760],
        [call_per_arg: InternalGasPerArg, "call.per_arg", 3670],
        [call_per_local: InternalGasPerArg, { 1.. => "call.per_local" }, 3670],
        [call_generic_base: InternalGas, "call_generic.base", 36760],
        [call_generic_per_ty_arg: InternalGasPerArg, "call_generic.per_ty_arg", 3670],
        [call_generic_per_arg: InternalGasPerArg, "call_generic.per_arg", 3670],
        [call_generic_per_local: InternalGasPerArg, { 1.. => "call_generic.per_local" }, 3670],
        // struct
        [pack_base: InternalGas, "pack.base", 8080],
        [pack_per_field: InternalGasPerArg, "pack.per_field", 1470],
        [pack_generic_base: InternalGas, "pack_generic.base", 8080],
        [pack_generic_per_field: InternalGasPerArg, "pack_generic.per_field", 1470],
        [unpack_base: InternalGas, "unpack.base", 8080],
        [unpack_per_field: InternalGasPerArg, "unpack.per_field", 1470],
        [unpack_generic_base: InternalGas, "unpack_generic.base", 8080],
        [unpack_generic_per_field: InternalGasPerArg, "unpack_generic.per_field", 1470],
        [pack_closure_base: InternalGas, { RELEASE_V1_33.. => "pack_closure.base" }, 9080],
        [pack_closure_per_arg: InternalGasPerArg,  { RELEASE_V1_33.. => "pack.closure.per_arg" }, 1470],
        [pack_closure_generic_base: InternalGas,  { RELEASE_V1_33.. => "pack_closure_generic.base" }, 9080],
        [pack_closure_generic_per_arg: InternalGasPerArg,  { RELEASE_V1_33.. => "pack_closure_generic.per_arg" }, 1470],
        // ref
        [read_ref_base: InternalGas, "read_ref.base", 7350],
        [read_ref_per_abs_val_unit: InternalGasPerAbstractValueUnit, "read_ref.per_abs_val_unit", 140],
        [write_ref_base: InternalGas, "write_ref.base", 7350],
        [freeze_ref: InternalGas, "freeze_ref", 360],
        // casting
        [cast_u8: InternalGas, "cast_u8", 4410],
        [cast_u16: InternalGas, { 5.. => "cast_u16" }, 4410],
        [cast_u32: InternalGas, { 5.. => "cast_u32" }, 4410],
        [cast_u64: InternalGas, "cast_u64", 4410],
        [cast_u128: InternalGas, "cast_u128", 4410],
        [cast_u256: InternalGas, { 5.. => "cast_u256" }, 4410],
        [cast_i8: InternalGas, { RELEASE_V1_38.. => "cast_i8" }, 4410],
        [cast_i16: InternalGas, { RELEASE_V1_38.. => "cast_i16" }, 4410],
        [cast_i32: InternalGas, { RELEASE_V1_38.. => "cast_i32" }, 4410],
        [cast_i64: InternalGas, { RELEASE_V1_38.. => "cast_i64" }, 4410],
        [cast_i128: InternalGas, { RELEASE_V1_38.. => "cast_i128" }, 4410],
        [cast_i256: InternalGas, { RELEASE_V1_38.. => "cast_i256" }, 4410],
        // arithmetic
        [add: InternalGas, "add", 5880],
        [sub: InternalGas, "sub", 5880],
        [mul: InternalGas, "mul", 5880],
        [mod_: InternalGas, "mod", 5880],
        [div: InternalGas, "div", 5880],
        [negate: InternalGas, { RELEASE_V1_38.. =>  "negate" }, 5880],
        // bitwise
        [bit_or: InternalGas, "bit_or", 5880],
        [bit_and: InternalGas, "bit_and", 5880],
        [xor: InternalGas, "bit_xor", 5880],
        [shl: InternalGas, "bit_shl", 5880],
        [shr: InternalGas, "bit_shr", 5880],
        // boolean
        [or: InternalGas, "or", 5880],
        [and: InternalGas, "and", 5880],
        [not: InternalGas, "not", 5880],
        // comparison
        [lt: InternalGas, "lt", 5880],
        [gt: InternalGas, "gt", 5880],
        [le: InternalGas, "le", 5880],
        [ge: InternalGas, "ge", 5880],
        [eq_base: InternalGas, "eq.base", 3670],
        [eq_per_abs_val_unit: InternalGasPerAbstractValueUnit, "eq.per_abs_val_unit", 140],
        [neq_base: InternalGas, "neq.base", 3670],
        [neq_per_abs_val_unit: InternalGasPerAbstractValueUnit, "neq.per_abs_val_unit", 140],
        // global
        [imm_borrow_global_base: InternalGas, "imm_borrow_global.base", 18380],
        [imm_borrow_global_generic_base: InternalGas, "imm_borrow_global_generic.base", 18380],
        [mut_borrow_global_base: InternalGas, "mut_borrow_global.base", 18380],
        [mut_borrow_global_generic_base: InternalGas, "mut_borrow_global_generic.base", 18380],
        [exists_base: InternalGas, "exists.base", 9190],
        [exists_generic_base: InternalGas, "exists_generic.base", 9190],
        [move_from_base: InternalGas, "move_from.base", 12860],
        [move_from_generic_base: InternalGas, "move_from_generic.base", 12860],
        [move_to_base: InternalGas, "move_to.base", 18380],
        [move_to_generic_base: InternalGas, "move_to_generic.base", 18380],
        // vec
        [vec_len_base: InternalGas, "vec_len.base", 8080],
        [vec_imm_borrow_base: InternalGas, "vec_imm_borrow.base", 12130],
        [vec_mut_borrow_base: InternalGas, "vec_mut_borrow.base", 12130],
        [vec_push_back_base: InternalGas, "vec_push_back.base", 13960],
        [vec_pop_back_base: InternalGas, "vec_pop_back.base", 9550],
        [vec_swap_base: InternalGas, "vec_swap.base", 11020],
        [vec_pack_base: InternalGas, "vec_pack.base", 22050],
        [vec_pack_per_elem: InternalGasPerArg, "vec_pack.per_elem", 1470],
        [vec_unpack_base: InternalGas, "vec_unpack.base", 18380],
        [vec_unpack_per_expected_elem: InternalGasPerArg, "vec_unpack.per_expected_elem", 1470],
        [subst_ty_per_node: InternalGasPerTypeNode, { 14.. => "subst_ty_per_node" }, 4000],
    ]
);
