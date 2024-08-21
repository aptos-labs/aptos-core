// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for unnecessary boolean
//! identity comparisons, e.g, `x == true`, `false != foo(x)`, etc.
//!
//! The recommendation is to instead use the boolean expression (or their negations)
//! directly, e.g.,
//!   `x == true` ==> `x`
//!   `false != foo(x)` ==> `!foo(x)`

use crate::env_pipeline::model_ast_lints::ExpressionLinter;
use move_model::{
    ast::{ExpData, Operation, Value},
    model::GlobalEnv,
};

#[derive(Default)]
pub struct UnnecessaryBooleanIdentityComparison;

impl ExpressionLinter for UnnecessaryBooleanIdentityComparison {
    fn get_name(&self) -> &'static str {
        "unnecessary_boolean_identity_comparison"
    }

    fn visit_expr_pre(&mut self, env: &GlobalEnv, expr: &ExpData) {
        use ExpData::{Call, Value as ExpValue};
        use Operation::*;
        use Value::Bool;
        if let Call(_, cmp @ (Eq | Neq), args) = expr {
            // Narrowed down to == or != comparisons.
            debug_assert!(
                args.len() == 2,
                "there should be exactly two arguments for == or !="
            );
            match (args[0].as_ref(), args[1].as_ref()) {
                (ExpValue(_, Bool(b)), e) | (e, ExpValue(_, Bool(b))) => {
                    // One of the arguments is a boolean literal.
                    let msg = format!(
                        "Directly use the {}boolean expression, instead of comparing it with `{}`.",
                        if (*b && cmp == &Eq) || (!*b && cmp == &Neq) {
                            ""
                        } else {
                            "negation of the "
                        },
                        if *b { "true" } else { "false" }
                    );
                    env.lint_diag(&env.get_node_loc(e.node_id()), self.get_name(), &msg);
                },
                _ => {},
            }
        }
    }
}
