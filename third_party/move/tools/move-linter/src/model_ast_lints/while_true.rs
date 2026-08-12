// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module implements an expression linter that checks code of the form:
//! `while (true) { ... }` and suggests to use `loop { ... }` instead.

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
        // Note that `for` loops cannot match this shape: they are lowered with a
        // bound comparison as the loop condition.
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
        let env = function.env();
        // If we are here, it is `while (true) {...}`.
        self.report(
            env,
            &env.get_node_loc(*id),
            "Use the more explicit `loop` instead.",
        );
    }
}
