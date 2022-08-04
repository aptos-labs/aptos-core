// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::gas_meter::{FromOnChainGasSchedule, InitialGasSchedule, ToOnChainGasSchedule};
use move_binary_format::{
    errors::{PartialVMError, PartialVMResult},
    file_format_common::Opcodes,
};
use move_core_types::vm_status::StatusCode;
use std::collections::BTreeMap;

macro_rules! define_gas_parameters_for_instructions {
    ($([$name: ident, $key: literal $(,)?, $initial: expr $(,)?]),* $(,)?) => {
        /// Gas parameters for all bytecode instructions.
        ///
        /// Note: due to performance considerations, this is represented as a fixed struct instead of
        /// some other data structures that require complex lookups.
        #[derive(Debug, Clone)]
        pub struct InstructionGasParameters {
            $(pub $name : u64),*
        }

        impl FromOnChainGasSchedule for InstructionGasParameters {
            fn from_on_chain_gas_schedule(gas_schedule: &BTreeMap<String, u64>) -> Option<Self> {
                Some(InstructionGasParameters { $($name: gas_schedule.get(&format!("instr.{}", $key)).cloned()?),* })
            }
        }

        impl ToOnChainGasSchedule for InstructionGasParameters {
            fn to_on_chain_gas_schedule(&self) -> Vec<(String, u64)> {
                vec![$((format!("instr.{}", $key), self.$name)),*]
            }
        }

        impl InstructionGasParameters {
            pub fn zeros() -> Self {
                Self {
                    $($name: 0),*
                }
            }
        }

        impl InitialGasSchedule for InstructionGasParameters {
            fn initial() -> Self {
                Self {
                    $($name: $initial),*
                }
            }
        }

        #[test]
        fn keys_should_be_unique() {
            let mut map: BTreeMap<&str, ()> = BTreeMap::new();

            for key in [$($key),*] {
                assert!(map.insert(key, ()).is_none());
            }
        }
    };
}

define_gas_parameters_for_instructions!(
    // nop
    [nop, "nop", 1],
    // control flow
    [ret, "ret", 1],
    [abort, "abort", 1],
    [br_true, "br_true", 1],
    [br_false, "br_false", 1],
    [branch, "branch", 1],
    // stack
    [pop, "pop", 1],
    [ld_u8, "ld_u8", 1],
    [ld_u64, "ld_u64", 1],
    [ld_u128, "ld_u128", 1],
    [ld_true, "ld_true", 1],
    [ld_false, "ld_false", 1],
    [ld_const_base, "ld_const.base", 1],
    [ld_const_unit, "ld_const.unit", 1],
    // borrow
    [imm_borrow_loc, "imm_borrow_loc", 1],
    [mut_borrow_loc, "mut_borrow_loc", 1],
    [imm_borrow_field, "imm_borrow_field", 1],
    [mut_borrow_field, "mut_borrow_field", 1],
    [imm_borrow_field_generic, "imm_borrow_field_generic", 1],
    [mut_borrow_field_generic, "mut_borrow_field_generic", 1],
    // locals
    [copy_loc_base, "copy_loc.base", 1],
    [copy_loc_unit, "copy_loc.unit", 1],
    [move_loc_base, "move_loc.base", 1],
    [move_loc_unit, "move_loc.unit", 1],
    [st_loc_base, "st_loc.base", 1],
    [st_loc_unit, "st_loc.unit", 1],
    // call
    [call_base, "call.base", 1],
    [call_unit, "call.unit", 1],
    [call_generic_base, "call_generic.base", 1],
    [call_generic_unit, "call_generic.unit", 1],
    // struct
    [pack_base, "pack.base", 1],
    [pack_unit, "pack.unit", 1],
    [pack_generic_base, "pack_generic.base", 1],
    [pack_generic_unit, "pack_generic.unit", 1],
    [unpack_base, "unpack.base", 1],
    [unpack_unit, "unpack.unit", 1],
    [unpack_generic_base, "unpack_generic.base", 1],
    [unpack_generic_unit, "unpack_generic.unit", 1],
    // ref
    [read_ref_base, "read_ref.base", 1],
    [read_ref_unit, "read_ref.unit", 1],
    [write_ref_base, "write_ref.base", 1],
    [write_ref_unit, "write_ref.unit", 1],
    [freeze_ref, "freeze_ref", 1],
    // casting
    [cast_u8, "cast_u8", 1],
    [cast_u64, "cast_u64", 1],
    [cast_u128, "cast_u128", 1],
    // arithmetic
    [add, "add", 1],
    [sub, "sub", 1],
    [mul, "mul", 1],
    [mod_, "mod", 1],
    [div, "div", 1],
    // bitwise
    [bit_or, "bit_or", 1],
    [bit_and, "bit_and", 1],
    [xor, "bit_xor", 1],
    [shl, "bit_shl", 1],
    [shr, "bit_shr", 1],
    // boolean
    [or, "or", 1],
    [and, "and", 1],
    [not, "not", 1],
    // comparison
    [lt, "lt", 1],
    [gt, "gt", 1],
    [le, "le", 1],
    [ge, "ge", 1],
    [eq_base, "eq.base", 1],
    [eq_unit, "eq.unit", 1],
    [neq_base, "neq.base", 1],
    [neq_unit, "neq.unit", 1],
    // global
    [imm_borrow_global_base, "imm_borrow_global.base", 10],
    [imm_borrow_global_unit, "imm_borrow_global.unit", 10],
    [
        imm_borrow_global_generic_base,
        "imm_borrow_global_generic.base",
        10
    ],
    [
        imm_borrow_global_generic_unit,
        "imm_borrow_global_generic.unit",
        10
    ],
    [mut_borrow_global_base, "mut_borrow_global.base", 100],
    [mut_borrow_global_unit, "mut_borrow_global.unit", 100],
    [
        mut_borrow_global_generic_base,
        "mut_borrow_global_generic.base",
        100
    ],
    [
        mut_borrow_global_generic_unit,
        "mut_borrow_global_generic.unit",
        100
    ],
    [exists_base, "exists.base", 10],
    [exists_unit, "exists.unit", 10],
    [exists_generic_base, "exists_generic.base", 10],
    [exists_generic_unit, "exists_generic.unit", 10],
    [move_from_base, "move_from.base", 100],
    [move_from_unit, "move_from.unit", 100],
    [move_from_generic_base, "move_from_generic.base", 100],
    [move_from_generic_unit, "move_from_generic.unit", 100],
    [move_to_base, "move_to.base", 100],
    [move_to_unit, "move_to.unit", 100],
    [move_to_generic_base, "move_to_generic.base", 100],
    [move_to_generic_unit, "move_to_generic.unit", 100],
    // vec
    [vec_len, "vec_len", 1],
    [vec_imm_borrow, "vec_imm_borrow", 1],
    [vec_mut_borrow, "vec_mut_borrow", 1],
    [vec_push_back, "vec_push_back", 1],
    [vec_pop_back, "vec_pop_back", 1],
    [vec_swap, "vec_swap", 1],
    [vec_pack_base, "vec_pack.base", 1],
    [vec_pack_unit, "vec_pack.unit", 1],
    [vec_unpack_base, "vec_unpack.base", 1],
    [vec_unpack_unit, "vec_unpack.unit", 1],
);

impl InstructionGasParameters {
    pub(crate) fn instr_cost(&self, op: Opcodes) -> PartialVMResult<u64> {
        use Opcodes::*;

        Ok(match op {
            NOP => self.nop,

            ABORT => self.abort,
            RET => self.ret,

            BR_TRUE => self.br_true,
            BR_FALSE => self.br_false,
            BRANCH => self.branch,

            POP => self.pop,
            LD_U8 => self.ld_u8,
            LD_U64 => self.ld_u64,
            LD_U128 => self.ld_u128,
            LD_TRUE => self.ld_true,
            LD_FALSE => self.ld_false,

            IMM_BORROW_LOC => self.imm_borrow_loc,
            MUT_BORROW_LOC => self.mut_borrow_loc,
            IMM_BORROW_FIELD => self.imm_borrow_field,
            MUT_BORROW_FIELD => self.mut_borrow_field,
            IMM_BORROW_FIELD_GENERIC => self.imm_borrow_field_generic,
            MUT_BORROW_FIELD_GENERIC => self.mut_borrow_field_generic,
            FREEZE_REF => self.freeze_ref,

            CAST_U8 => self.cast_u8,
            CAST_U64 => self.cast_u64,
            CAST_U128 => self.cast_u128,

            ADD => self.add,
            SUB => self.sub,
            MUL => self.mul,
            MOD => self.mod_,
            DIV => self.div,

            BIT_OR => self.bit_or,
            BIT_AND => self.bit_and,
            XOR => self.xor,
            SHL => self.shl,
            SHR => self.shr,

            OR => self.or,
            AND => self.and,
            NOT => self.not,

            LT => self.lt,
            GT => self.gt,
            LE => self.le,
            GE => self.ge,

            VEC_LEN => self.vec_len,
            VEC_IMM_BORROW => self.vec_imm_borrow,
            VEC_MUT_BORROW => self.vec_mut_borrow,
            VEC_POP_BACK => self.vec_pop_back,
            VEC_SWAP => self.vec_swap,

            op => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!("cannot charge gas for unknown operation {:?}", op)),
                )
            }
        })
    }

    pub(crate) fn instr_cost_with_size(&self, op: Opcodes, size: u64) -> PartialVMResult<u64> {
        use Opcodes::*;

        Ok(match op {
            LD_CONST => self.ld_const_base + self.ld_const_unit * size,
            COPY_LOC => self.copy_loc_base + self.copy_loc_unit * size,

            // TODO(Gas): fix size calculation in the Move repo
            MOVE_LOC => self.move_loc_base + self.move_loc_unit * size,

            // TODO(Gas): fix size calculation in the Move repo
            ST_LOC => self.st_loc_base + self.st_loc_unit * size,

            // TODO(Gas): fix size calculation in the Move repo
            // size = num of args + 1
            CALL => self.call_base + self.call_unit * size,
            // TODO(Gas): fix size calculation in the Move repo
            // size = num of ty args + num of args + 1
            CALL_GENERIC => self.call_generic_base + self.call_generic_unit * size,

            // TODO(Gas): fix size calculation in the Move repo
            // size = num of fields + sum(field.size())
            PACK => self.pack_base + self.pack_unit * size,
            PACK_GENERIC => self.pack_generic_base + self.pack_generic_unit * size,

            // TODO(Gas): fix size calculation in the Move repo
            // size = num of fields + sum(field.size())
            UNPACK => self.unpack_base + self.unpack_unit * size,
            UNPACK_GENERIC => self.unpack_generic_base + self.unpack_generic_unit * size,

            READ_REF => self.read_ref_base + self.read_ref_unit * size,
            // TODO(Gas): fix size calculation in the Move repo
            // current size = full_size(value)
            WRITE_REF => self.write_ref_base + self.write_ref_unit * size,

            EQ => self.eq_base + self.eq_unit * size,
            NEQ => self.neq_base + self.neq_unit * size,

            IMM_BORROW_GLOBAL => self.imm_borrow_global_base + self.imm_borrow_global_unit * size,
            IMM_BORROW_GLOBAL_GENERIC => {
                self.imm_borrow_global_generic_base + self.imm_borrow_global_generic_unit * size
            }
            MUT_BORROW_GLOBAL => self.mut_borrow_global_base + self.mut_borrow_global_unit * size,
            MUT_BORROW_GLOBAL_GENERIC => {
                self.mut_borrow_global_generic_base + self.mut_borrow_global_generic_unit * size
            }
            EXISTS => self.exists_base + self.exists_unit * size,
            EXISTS_GENERIC => self.exists_generic_base + self.exists_generic_unit * size,
            MOVE_FROM => self.move_from_base + self.move_from_unit * size,
            MOVE_FROM_GENERIC => self.move_from_generic_base + self.move_from_generic_unit * size,
            MOVE_TO => self.move_to_base + self.move_to_unit * size,
            MOVE_TO_GENERIC => self.move_to_generic_base + self.move_to_generic_unit * size,

            // TODO(Gas): this should be an unsized operation
            VEC_PUSH_BACK => self.vec_push_back,
            // TODO(Gas): fix size calculation in the Move repo
            // current size = num of elements
            VEC_PACK => self.vec_pack_base + self.vec_pack_unit * size,
            VEC_UNPACK => self.vec_unpack_base + self.vec_unpack_unit * size,

            op => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!("cannot charge gas for unknown operation {:?}", op)),
                )
            }
        })
    }
}
