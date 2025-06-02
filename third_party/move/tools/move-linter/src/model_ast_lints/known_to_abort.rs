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
    ast::{ExpData, Operation, Value},
    model::GlobalEnv,
    ty::Type,
};
use num::BigInt;

#[derive(Default)]
pub struct KnownToAbort;

impl ExpChecker for KnownToAbort {
    fn get_name(&self) -> String {
        "known_to_abort".to_string()
    }

    fn visit_expr_pre(&mut self, env: &GlobalEnv, expr: &ExpData) {
        use ExpData::{Call, Value as ExpVal};
        use Operation::*;
        use Value::Number as N;

        match expr {
            Call(id, op, args) if args.len() == 2 => {
                let (lhs, rhs): (&ExpData, &ExpData) = (&args[0], &args[1]);

                if let Some(msg) = match (lhs, op, rhs) {
                    (_, Div, ExpVal(_, N(n))) if n == &BigInt::from(0) => {
                        Some("Division by zero will cause the program to abort")
                    },
                    (_, Mod, ExpVal(_, N(n))) if n == &BigInt::from(0) => {
                        Some("Modulo by zero will cause the program to abort")
                    },
                    (lhs_expr, Shl, ExpVal(_, N(n))) => {
                        self.check_shift_overflow(env, lhs_expr, n, "left")
                    },
                    (lhs_expr, Shr, ExpVal(_, N(n))) => {
                        self.check_shift_overflow(env, lhs_expr, n, "right")
                    },
                    _ => None,
                } {
                    self.report(env, &env.get_node_loc(*id), msg);
                }
            },
            Call(id, Cast, args) if args.len() == 1 => {
                // Check for cast operations: constant as type
                if let ExpVal(_, N(constant_value)) = args[0].as_ref() {
                    let target_type = env.get_node_type(expr.node_id());
                    if let Some(msg) = self.check_cast_overflow(constant_value, &target_type) {
                        self.report(env, &env.get_node_loc(*id), msg);
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
        direction: &str,
    ) -> Option<&'static str> {
        let ty = env.get_node_type(lhs_expr.node_id());
        if !ty.is_number() {
            return None;
        }

        let Type::Primitive(prim_ty) = &ty else {
            return None;
        };

        let bit_width = match prim_ty {
            move_model::ty::PrimitiveType::U8 => 8,
            move_model::ty::PrimitiveType::U16 => 16,
            move_model::ty::PrimitiveType::U32 => 32,
            move_model::ty::PrimitiveType::U64 => 64,
            move_model::ty::PrimitiveType::U128 => 128,
            move_model::ty::PrimitiveType::U256 => 256,
            _ => return None,
        };

        if shift_amount >= &BigInt::from(bit_width) {
            match direction {
                "left" => Some("Left shift by amount greater than or equal to the type's bit width will cause the program to abort"),
                "right" => Some("Right shift by amount greater than or equal to the type's bit width will cause the program to abort"),
                _ => Some("Shift by amount greater than or equal to the type's bit width will cause the program to abort"),
            }
        } else {
            None
        }
    }

    fn check_cast_overflow(&self, value: &BigInt, target_type: &Type) -> Option<&'static str> {
        if !target_type.is_number() {
            return None;
        }

        let Type::Primitive(prim_ty) = target_type else {
            return None;
        };

        let min_value = prim_ty.get_min_value();
        let max_value = prim_ty.get_max_value();

        let (min_val, max_val) = match (min_value, max_value) {
            (Some(min), Some(max)) => (min, max),
            _ => return None,
        };

        if value < &min_val || value > &max_val {
            Some("Cast operation will cause the program to abort because the value is outside the target type's range")
        } else {
            None
        }
    }
}
