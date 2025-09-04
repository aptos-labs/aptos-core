// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines the gas parameters for all Move instructions.

use crate::{
    gas_feature_versions::{RELEASE_V1_18, RELEASE_V1_33},
    gas_schedule::VMGasParameters,
};
use velor_gas_algebra::{
    InternalGas, InternalGasPerAbstractValueUnit, InternalGasPerArg, InternalGasPerByte,
    InternalGasPerTypeNode,
};

crate::gas_schedule::macros::define_gas_parameters!(
    InstructionGasParameters,
    "instr",
    VMGasParameters => .instr,
    [
        // nop
        [nop: InternalGas, "nop", 36],
        // control flow
        [ret: InternalGas, "ret", 220],
        [abort: InternalGas, "abort", 220],

        // Note(Gas): The costs of the branch instructions have been jacked up a bit intentionally
        //            to prevent any single transaction from running for too long.
        [br_true: InternalGas, "br_true", 441],
        [br_false: InternalGas, "br_false", 441],
        [branch: InternalGas, "branch", 294],

        // stack
        [pop: InternalGas, "pop", 147],
        [ld_u8: InternalGas, "ld_u8", 220],
        [ld_u16: InternalGas, { 5.. => "ld_u16" }, 220],
        [ld_u32: InternalGas, { 5.. => "ld_u32" }, 220],
        [ld_u64: InternalGas, "ld_u64", 220],
        [ld_u128: InternalGas, "ld_u128", 294],
        [ld_u256: InternalGas, { 5.. => "ld_u256" }, 294],
        [ld_true: InternalGas, "ld_true", 220],
        [ld_false: InternalGas, "ld_false", 220],
        [ld_const_base: InternalGas, "ld_const.base", 2389],
        [ld_const_per_byte: InternalGasPerByte, "ld_const.per_byte", 128],
        // borrow
        [imm_borrow_loc: InternalGas, "imm_borrow_loc", 220],
        [mut_borrow_loc: InternalGas, "mut_borrow_loc", 220],
        [imm_borrow_field: InternalGas, "imm_borrow_field", 735],
        [mut_borrow_field: InternalGas, "mut_borrow_field", 735],
        [imm_borrow_field_generic: InternalGas, "imm_borrow_field_generic" , 735],
        [mut_borrow_field_generic: InternalGas, "mut_borrow_field_generic", 735],
        [imm_borrow_variant_field: InternalGas,
            { RELEASE_V1_18.. => "imm_borrow_variant_field" }, 835],
        [mut_borrow_variant_field: InternalGas,
            { RELEASE_V1_18.. => "mut_borrow_variant_field" }, 835],
        [imm_borrow_variant_field_generic: InternalGas,
            { RELEASE_V1_18.. => "imm_borrow_variant_field_generic" }, 835],
        [mut_borrow_variant_field_generic: InternalGas,
            { RELEASE_V1_18.. => "mut_borrow_variant_field_generic" }, 835],

        // variant testing
        [test_variant: InternalGas,
            { RELEASE_V1_18.. => "test_variant" }, 535],
        [test_variant_generic: InternalGas,
            { RELEASE_V1_18.. => "test_variant_generic" }, 535],

        // locals
        [copy_loc_base: InternalGas, "copy_loc.base", 294],
        [copy_loc_per_abs_val_unit: InternalGasPerAbstractValueUnit, "copy_loc.per_abs_val_unit", 14],
        [move_loc_base: InternalGas, "move_loc.base", 441],
        [st_loc_base: InternalGas, "st_loc.base", 441],
        // call
        [call_base: InternalGas, "call.base", 3676],
        [call_per_arg: InternalGasPerArg, "call.per_arg", 367],
        [call_per_local: InternalGasPerArg, { 1.. => "call.per_local" }, 367],
        [call_generic_base: InternalGas, "call_generic.base", 3676],
        [call_generic_per_ty_arg: InternalGasPerArg, "call_generic.per_ty_arg", 367],
        [call_generic_per_arg: InternalGasPerArg, "call_generic.per_arg", 367],
        [call_generic_per_local: InternalGasPerArg, { 1.. => "call_generic.per_local" }, 367],
        // struct
        [pack_base: InternalGas, "pack.base", 808],
        [pack_per_field: InternalGasPerArg, "pack.per_field", 147],
        [pack_generic_base: InternalGas, "pack_generic.base", 808],
        [pack_generic_per_field: InternalGasPerArg, "pack_generic.per_field", 147],
        [unpack_base: InternalGas, "unpack.base", 808],
        [unpack_per_field: InternalGasPerArg, "unpack.per_field", 147],
        [unpack_generic_base: InternalGas, "unpack_generic.base", 808],
        [unpack_generic_per_field: InternalGasPerArg, "unpack_generic.per_field", 147],
        [pack_closure_base: InternalGas, { RELEASE_V1_33.. => "pack_closure.base" }, 908],
        [pack_closure_per_arg: InternalGasPerArg,  { RELEASE_V1_33.. => "pack.closure.per_arg" }, 147],
        [pack_closure_generic_base: InternalGas,  { RELEASE_V1_33.. => "pack_closure_generic.base" }, 908],
        [pack_closure_generic_per_arg: InternalGasPerArg,  { RELEASE_V1_33.. => "pack_closure_generic.per_arg" }, 147],
        // ref
        [read_ref_base: InternalGas, "read_ref.base", 735],
        [read_ref_per_abs_val_unit: InternalGasPerAbstractValueUnit, "read_ref.per_abs_val_unit", 14],
        [write_ref_base: InternalGas, "write_ref.base", 735],
        [freeze_ref: InternalGas, "freeze_ref", 36],
        // casting
        [cast_u8: InternalGas, "cast_u8", 441],
        [cast_u16: InternalGas, { 5.. => "cast_u16" }, 441],
        [cast_u32: InternalGas, { 5.. => "cast_u32" }, 441],
        [cast_u64: InternalGas, "cast_u64", 441],
        [cast_u128: InternalGas, "cast_u128", 441],
        [cast_u256: InternalGas, { 5.. => "cast_u256" }, 441],
        // arithmetic
        [add: InternalGas, "add", 588],
        [sub: InternalGas, "sub", 588],
        [mul: InternalGas, "mul", 588],
        [mod_: InternalGas, "mod", 588],
        [div: InternalGas, "div", 588],
        // bitwise
        [bit_or: InternalGas, "bit_or", 588],
        [bit_and: InternalGas, "bit_and", 588],
        [xor: InternalGas, "bit_xor", 588],
        [shl: InternalGas, "bit_shl", 588],
        [shr: InternalGas, "bit_shr", 588],
        // boolean
        [or: InternalGas, "or", 588],
        [and: InternalGas, "and", 588],
        [not: InternalGas, "not", 588],
        // comparison
        [lt: InternalGas, "lt", 588],
        [gt: InternalGas, "gt", 588],
        [le: InternalGas, "le", 588],
        [ge: InternalGas, "ge", 588],
        [eq_base: InternalGas, "eq.base", 367],
        [eq_per_abs_val_unit: InternalGasPerAbstractValueUnit, "eq.per_abs_val_unit", 14],
        [neq_base: InternalGas, "neq.base", 367],
        [neq_per_abs_val_unit: InternalGasPerAbstractValueUnit, "neq.per_abs_val_unit", 14],
        // global
        [imm_borrow_global_base: InternalGas, "imm_borrow_global.base", 1838],
        [imm_borrow_global_generic_base: InternalGas, "imm_borrow_global_generic.base", 1838],
        [mut_borrow_global_base: InternalGas, "mut_borrow_global.base", 1838],
        [mut_borrow_global_generic_base: InternalGas, "mut_borrow_global_generic.base", 1838],
        [exists_base: InternalGas, "exists.base", 919],
        [exists_generic_base: InternalGas, "exists_generic.base", 919],
        [move_from_base: InternalGas, "move_from.base", 1286],
        [move_from_generic_base: InternalGas, "move_from_generic.base", 1286],
        [move_to_base: InternalGas, "move_to.base", 1838],
        [move_to_generic_base: InternalGas, "move_to_generic.base", 1838],
        // vec
        [vec_len_base: InternalGas, "vec_len.base", 808],
        [vec_imm_borrow_base: InternalGas, "vec_imm_borrow.base", 1213],
        [vec_mut_borrow_base: InternalGas, "vec_mut_borrow.base", 1213],
        [vec_push_back_base: InternalGas, "vec_push_back.base", 1396],
        [vec_pop_back_base: InternalGas, "vec_pop_back.base", 955],
        [vec_swap_base: InternalGas, "vec_swap.base", 1102],
        [vec_pack_base: InternalGas, "vec_pack.base", 2205],
        [vec_pack_per_elem: InternalGasPerArg, "vec_pack.per_elem", 147],
        [vec_unpack_base: InternalGas, "vec_unpack.base", 1838],
        [vec_unpack_per_expected_elem: InternalGasPerArg, "vec_unpack.per_expected_elem", 147],
        [subst_ty_per_node: InternalGasPerTypeNode, { 14.. => "subst_ty_per_node" }, 400],
    ]
);
