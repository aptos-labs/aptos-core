// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for expressions
//! that are known to abort at compile time.
//!
//! The following cases are checked:
//! * `x << n`, `x >> n` where `n` is a constant >= the bit width of `x`'s type
//! * `x / 0`, `x % 0` (divide or modulo by zero)
//! * `constant as type` where `constant` is outside the range of `type`

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{
        ExpData::{self, Call, Value},
        Operation,
        Value::Number,
    },
    model::GlobalEnv,
    ty::{PrimitiveType, Type},
};
use num::BigInt;
use Operation::*;

const DIVISION_BY_ZERO_MSG: &str = "Division by zero will cause the program to abort";
const MODULO_BY_ZERO_MSG: &str = "Modulo by zero will cause the program to abort";
const SHIFT_OVERFLOW_MSG: &str =
    "Shift by amount greater than or equal to the type's bit width will cause the program to abort";
const CAST_OVERFLOW_MSG: &str = "Cast operation will cause the program to abort because the value is outside the target type's range";

#[derive(Default)]
pub struct KnownToAbort;

impl ExpChecker for KnownToAbort {
    fn get_name(&self) -> String {
        "known_to_abort".to_string()
    }

    fn visit_expr_pre(&mut self, env: &GlobalEnv, expr: &ExpData) {
        match expr {
            Call(id, op, args) if args.len() == 2 => {
                let (lhs, rhs): (&ExpData, &ExpData) = (&args[0], &args[1]);

                if let Some(msg) = match (lhs, op, rhs) {
                    (_, Div, Value(_, Number(n))) if n == &BigInt::from(0) => {
                        Some(DIVISION_BY_ZERO_MSG)
                    },
                    (_, Mod, Value(_, Number(n))) if n == &BigInt::from(0) => {
                        Some(MODULO_BY_ZERO_MSG)
                    },
                    (lhs_expr, Shl | Shr, Value(_, Number(n))) => {
                        self.check_shift_overflow(env, lhs_expr, n)
                    },
                    _ => None,
                } {
                    self.report(env, &env.get_node_loc(*id), msg);
                }
            },
            Call(id, Cast, args) if args.len() == 1 => {
                // Check for cast operations: constant as type
                if let Value(_, Number(constant_value)) = args[0].as_ref() {
                    let target_type = env.get_node_type(expr.node_id());
                    if self.check_cast_overflow(constant_value, &target_type) {
                        self.report(env, &env.get_node_loc(*id), CAST_OVERFLOW_MSG);
                    }
                }
            },
            _ => {},
        }
    }
}

impl KnownToAbort {
    fn check_shift_overflow(
        &self,
        env: &GlobalEnv,
        lhs_expr: &ExpData,
        shift_amount: &BigInt,
    ) -> Option<&'static str> {
        let ty = env.get_node_type(lhs_expr.node_id());
        let Type::Primitive(prim_ty) = &ty else {
            return None;
        };

        let bit_width = get_bit_width(prim_ty)?;

        if shift_amount >= &BigInt::from(bit_width) {
            Some(SHIFT_OVERFLOW_MSG)
        } else {
            None
        }
    }

    fn check_cast_overflow(&self, value: &BigInt, target_type: &Type) -> bool {
        let Type::Primitive(prim_ty) = target_type else {
            return false;
        };

        let (Some(min_val), Some(max_val)) = (prim_ty.get_min_value(), prim_ty.get_max_value())
        else {
            return false;
        };

        value < &min_val || value > &max_val
    }
}

fn get_bit_width(prim_ty: &PrimitiveType) -> Option<u32> {
    match prim_ty {
        PrimitiveType::U8 => Some(8),
        PrimitiveType::U16 => Some(16),
        PrimitiveType::U32 => Some(32),
        PrimitiveType::U64 => Some(64),
        PrimitiveType::U128 => Some(128),
        PrimitiveType::U256 => Some(256),
        _ => None,
    }
}
