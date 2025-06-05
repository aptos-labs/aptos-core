// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for assert!()s
//! where the condition is either `true` or `false`.
//! Note: As a side-effect, the linter also checks if blocks that are
//! equivalent to asserts. See the corresponding test file for an example.

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{ExpData, Operation, Value},
    model::FunctionEnv,
};

#[derive(Default)]
pub struct AssertConst;

impl ExpChecker for AssertConst {
    fn get_name(&self) -> String {
        "assert_const".to_string()
    }

    fn visit_expr_pre(&mut self, function: &FunctionEnv, expr: &ExpData) {
        let env = function.env();
        if let ExpData::IfElse(id, condition, then, else_) = expr {
            if !Self::is_assert(then, else_) {
                return;
            }
            let condition = Self::get_constant_bool_expression_value(condition);
            if condition.is_none() {
                return;
            }
            let condition = condition.unwrap();
            let string = if condition {
                "This assert can be removed"
            } else {
                "This assert can replaced with abort()"
            };
            self.report(env, &env.get_node_loc(*id), string);
        }
    }
}

impl AssertConst {
    fn empty_block(block: &ExpData) -> bool {
        if let ExpData::Call(_, op, exprs) = block {
            if *op != Operation::Tuple || exprs.len() != 0 {
                return false;
            }
            true
        } else {
            false
        }
    }
    fn abort_block(block: &ExpData) -> bool {
        if let ExpData::Call(_, op, _) = block {
            *op == Operation::Abort
        } else {
            false
        }
    }

    fn is_assert(then: &ExpData, else_: &ExpData) -> bool {
        Self::empty_block(then) && Self::abort_block(else_)
    }

    fn get_constant_bool_expression_value(expr: &ExpData) -> Option<bool> {
        if let ExpData::Value(_, val) = expr {
            match val {
                Value::Bool(x) => Some(*x),
                _ => None,
            }
        } else {
            None
        }
    }
}
