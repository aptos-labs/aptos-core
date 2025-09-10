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
        Exp,
        ExpData::{self, Call, Value},
        Operation,
        Value::Number,
    },
    model::{FunctionEnv, GlobalEnv},
    ty::Type,
};
use num::BigInt;
use Operation::*;

const DIVISION_BY_ZERO_MSG: &str = "Division by zero will abort";
const MODULO_BY_ZERO_MSG: &str = "Modulo by zero will abort";
const SHIFT_OVERFLOW_MSG: &str =
    "Shift by an amount greater than or equal to the type's bit width will abort";
const CAST_OVERFLOW_MSG: &str =
    "Cast operation will abort because the value is outside the target type's range";

#[derive(Default)]
pub struct KnownToAbort;

/// Attempts to compute a constant value from an expression tree involving
/// arithmetic operations and constant integer values.
///
/// Currently handles expressions involving addition (`+`), multiplication (`*`),
/// and constant integer values. This could be extended to include all numerical
/// and bitwise operations.
fn get_constant_value(args: &Vec<Exp>, op: &Operation) -> Option<BigInt> {
    let mut result = None;
    for arg in args {
        let value = match arg.as_ref() {
            ExpData::Value(_, Number(n)) => Some(n.clone()),
            ExpData::Call(_, inner_op @ (Add | Mul), inner_args) => {
                get_constant_value(inner_args, inner_op)
            },
            _ => None,
        };

        match (result, value) {
            (Some(current), Some(new_val)) => {
                result = Some(match op {
                    Add => current + new_val,
                    Mul => current * new_val,
                    _ => return None,
                });
            },
            (None, Some(new_val)) => {
                result = Some(new_val);
            },
            _ => return None,
        }
    }
    result
}

impl ExpChecker for KnownToAbort {
    fn get_name(&self) -> String {
        "known_to_abort".to_string()
    }

    fn visit_expr_pre(&mut self, function_env: &FunctionEnv<'_>, expr: &ExpData) {
        let env = function_env.env();
        match expr {
            Call(id, op, args) if args.len() == 2 => {
                let (lhs, rhs) = (args[0].as_ref(), args[1].as_ref());

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
                match args[0].as_ref() {
                    Value(_, Number(constant_value)) => {
                        let target_type = env.get_node_type(expr.node_id());
                        if let Some(msg) = self.check_cast_overflow(constant_value, &target_type) {
                            self.report(env, &env.get_node_loc(*id), msg);
                        }
                    },
                    Call(_, op, inner_args) => {
                        let constant_value = if *op == Add || *op == Mul {
                            get_constant_value(inner_args, op)
                        } else {
                            None
                        };

                        if let Some(constant_value) = constant_value {
                            let target_type = env.get_node_type(expr.node_id());
                            if let Some(msg) = self.check_cast_overflow(&constant_value, &target_type) {
                                self.report(env, &env.get_node_loc(*id), msg);
                            }
                        }
                    },
                    _ => {},
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

        let bit_width = prim_ty.get_num_bits()?;

        if shift_amount >= &BigInt::from(bit_width) {
            Some(SHIFT_OVERFLOW_MSG)
        } else {
            None
        }
    }

    fn check_cast_overflow(&self, value: &BigInt, target_type: &Type) -> Option<&'static str> {
        let Type::Primitive(prim_ty) = target_type else {
            return None;
        };

        let (Some(min_val), Some(max_val)) = (prim_ty.get_min_value(), prim_ty.get_max_value())
        else {
            return None;
        };

        if value < &min_val || value > &max_val {
            Some(CAST_OVERFLOW_MSG)
        } else {
            None
        }
    }
}
