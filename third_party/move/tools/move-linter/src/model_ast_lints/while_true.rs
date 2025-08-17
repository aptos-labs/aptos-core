// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks code of the form:
//! `while (true) { ... }` and suggests to use `loop { ... }` instead.

use crate::utils::detect_for_loop;
use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{ExpData, Value},
    model::FunctionEnv,
};

#[derive(Default)]
pub struct WhileTrue;

impl ExpChecker for WhileTrue {
    fn get_name(&self) -> String {
        "while_true".to_string()
    }

    fn visit_expr_pre(&mut self, function: &FunctionEnv, expr: &ExpData) {
        use ExpData::{IfElse, Loop};
        // Check if `expr` is of the form: Loop(IfElse(true, then, _)).
        let Loop(id, body) = expr else { return };
        let IfElse(_, cond, _, _) = body.as_ref() else {
            return;
        };
        let ExpData::Value(_, Value::Bool(b)) = cond.as_ref() else {
            return;
        };
        if !*b {
            return;
        }
        // Check if it is the `for` loop.
        if detect_for_loop(expr, function) {
            return;
        }
        let env = function.env();
        // If we are here, it is `while (true) {...}`.
        self.report(
            env,
            &env.get_node_loc(*id),
            "Use the more explicit `loop` instead.",
        );
    }
}
