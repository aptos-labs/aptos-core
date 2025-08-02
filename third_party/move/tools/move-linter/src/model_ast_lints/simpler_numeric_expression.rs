// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for numerical expressions
//! that can be simplified and warns about them.
//!
//! The following cases are checked and simplifications suggested (here, `x` stands
//! for an arbitrary expression, `=>` stands for "can be simplified to"):
//! * `x & 0`, `x * 0`, `0 & x`, `0 * x`, `0 << x`, `0 >> x`, `x % 1` => `0`
//! * `x | 0`, `x ^ 0`, `x >> 0`, `x << 0`, `x + 0`, `x - 0`, `x / 1`, `x * 1`,
//!   `0 | x`, `0 ^ x`, `0 + x`, `1 * x` => `x`

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{ExpData, Operation, Value},
    model::FunctionEnv,
};
use num::BigInt;

#[derive(Default)]
pub struct SimplerNumericExpression;

impl ExpChecker for SimplerNumericExpression {
    fn get_name(&self) -> String {
        "simpler_numeric_expression".to_string()
    }

    fn visit_expr_pre(&mut self, function: &FunctionEnv, expr: &ExpData) {
        use ExpData::{Call, Value as ExpVal};
        use Operation::*;
        use Value::Number as N;
        let Call(id, op, args) = expr else { return };
        if args.len() != 2 {
            return;
        }
        let (lhs, rhs): (&ExpData, &ExpData) = (&args[0], &args[1]);
        let zero = BigInt::from(0);
        let one = BigInt::from(1);
        if let Some(msg) = match (lhs, op, rhs) {
            (_, BitAnd | Mul, ExpVal(_, N(n))) | (ExpVal(_, N(n)), BitAnd | Mul | Shl | Shr, _)
                if n == &zero =>
            {
                // `x & 0`, `x * 0`, `0 & x`, `0 * x`, `0 << x`, `0 >> x`
                // Note that in that last two cases, `x` must be u8 because of type checking.
                Some("This expression can be simplified to just `0`")
            },
            (_, Mod, ExpVal(_, N(n))) if n == &one => {
                // `x % 1`
                Some("This expression can be simplified to just `0`")
            },
            (_, BitOr | Xor | Shr | Shl | Add | Sub, ExpVal(_, N(n))) if n == &zero => {
                // `x | 0`, `x ^ 0`, `x >> 0`, `x << 0`, `x + 0`, `x - 0`
                Some("This binary operation can be simplified to just the left-hand side")
            },
            (_, Div | Mul, ExpVal(_, N(n))) if n == &one => {
                // `x / 1`, `x * 1`
                Some("This binary operation can be simplified to just the left-hand side")
            },
            (ExpVal(_, N(n)), BitOr | Xor | Add, _) if n == &zero => {
                // `0 | x`, `0 ^ x`, `0 + x`
                Some("This binary operation can be simplified to just the right-hand side")
            },
            (ExpVal(_, N(n)), Mul, _) if n == &one => {
                // `1 * x`
                Some("This binary operation can be simplified to just the right-hand side")
            },
            _ => None,
        } {
            let env = function.env();
            self.report(env, &env.get_node_loc(*id), msg);
        }
    }
}
