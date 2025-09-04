// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for needless bool
//! of the forms:
//! 1. `if (x) true else false`, which can be replaced with just `x`.
//! 2. `if (x) false else true`, which can be replaced with just `!x`.
//! 3. `if (x) true else true`, which should be rewritten to remove the redundant branch.
//!
//! In addition, it also handles similar cases where both branches explicitly return
//! boolean values.

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{ExpData, Value},
    model::FunctionEnv,
};

#[derive(Default)]
pub struct NeedlessBool;

impl ExpChecker for NeedlessBool {
    fn get_name(&self) -> String {
        "needless_bool".to_string()
    }

    fn visit_expr_pre(&mut self, function: &FunctionEnv, expr: &ExpData) {
        use ExpData::IfElse;
        let env = function.env();
        if let IfElse(id, _, then, else_) = expr {
            match Self::fixed_bool_values(then, else_) {
                None => {},
                Some(ThenElseFixedValues { then, else_, .. }) if then == else_ => {
                    self.report(
                        env,
                        &env.get_node_loc(*id),
                        "This if-else has the same bool expression in both branches, consider rewriting the code to remove this redundancy",
                    );
                },
                Some(ThenElseFixedValues {
                    then,
                    both_returned,
                    ..
                }) => {
                    let negation = if then { "" } else { "the negation of " };
                    let returned = if both_returned { " returning" } else { "" };
                    self.report(
                        env,
                        &env.get_node_loc(*id),
                        &format!(
                            "This if-else can be replaced with just{} {}the condition",
                            returned, negation
                        ),
                    );
                },
            }
        }
    }
}

/// Fixed boolean values of the `then` and `else` branches of an if-else expression.
struct ThenElseFixedValues {
    then: bool,
    else_: bool,
    // true if both branches have explicit returns
    both_returned: bool,
}

impl NeedlessBool {
    /// Determine the fixed boolean values of both the `then` and `else_` branches
    /// of an if-else expression, if they exist.
    fn fixed_bool_values(then: &ExpData, else_: &ExpData) -> Option<ThenElseFixedValues> {
        use ExpData::{Return, Value as ExpValue};
        use Value::Bool;
        match (then, else_) {
            (ExpValue(_, Bool(then)), ExpValue(_, Bool(else_))) => Some(ThenElseFixedValues {
                then: *then,
                else_: *else_,
                both_returned: false,
            }),
            (Return(_, then), Return(_, else_)) => {
                Self::fixed_bool_values(then.as_ref(), else_.as_ref()).map(|mut v| {
                    v.both_returned = true;
                    v
                })
            },
            _ => None,
        }
    }
}
