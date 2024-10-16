// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks code of the form:
//! `while (true) { ... }` and suggests to use `loop { ... }` instead.

use crate::{env_pipeline::model_ast_lints::ExpressionLinter, lint_common::LintChecker};
use move_compiler::parser::syntax::FOR_LOOP_UPDATE_ITER_FLAG;
use move_model::{
    ast::{Exp, ExpData, Value},
    model::GlobalEnv,
};

#[derive(Default)]
pub struct WhileTrue;

impl ExpressionLinter for WhileTrue {
    fn get_lint_checker(&self) -> LintChecker {
        LintChecker::WhileTrue
    }

    fn visit_expr_pre(&mut self, env: &GlobalEnv, expr: &ExpData) {
        use ExpData::{IfElse, Loop};
        // Check if `expr` is of the form: Loop(IfElse(true, then, _)).
        let Loop(id, body) = expr else { return };
        let IfElse(_, cond, then, _) = body.as_ref() else {
            return;
        };
        let ExpData::Value(_, Value::Bool(b)) = cond.as_ref() else {
            return;
        };
        if !*b {
            return;
        }
        // Check if it is the `for` loop.
        if detect_for_loop(then, env) {
            return;
        }
        // If we are here, it is `while (true) {...}`.
        self.warning(
            env,
            &env.get_node_loc(*id),
            "Use the more explicit `loop` instead.",
        );
    }
}

fn detect_for_loop(then: &Exp, env: &GlobalEnv) -> bool {
    use ExpData::{IfElse, LocalVar, Sequence};
    // Check if `then` is of the form:
    //   Sequence([IfElse(LocalVar(FOR_LOOP_UPDATE_ITER_FLAG), ...)])
    // If so, it is the `for` loop.
    let Sequence(_, stmts) = then.as_ref() else {
        return false;
    };
    let Some(stmt) = stmts.first() else {
        return false;
    };
    let IfElse(_, cond, _, _) = stmt.as_ref() else {
        return false;
    };
    let LocalVar(_, name) = cond.as_ref() else {
        return false;
    };
    return name.display(env.symbol_pool()).to_string() == FOR_LOOP_UPDATE_ITER_FLAG;
}
