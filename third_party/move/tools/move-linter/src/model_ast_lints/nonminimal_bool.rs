// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for boolean expressions:
//! 1. `x && true`, which can be replaced with just `x`.
//! 2. `x && false`, which can be replaced with just `false`.
//! 3. `x || true`, which can be replaced with just `true`.
//! 4. `x || false`, which can be replaced with just `x`.
//! 5. `x <==> true`, which can be replaced with just `x`.
//! 6. `x <==> false`, which can be replaced with just `!x`.
//! 7. `x ==> true`, which can be replaced with just `true`.
//! 8. `x ==> false`, which can be replaced with just `!x`.
//! 9. `true ==> x`, which can be replaced with just `x`.
//! 10. `false ==> x`, which can be replaced with just `true`.
//! 11. `!true`, which can be replaced with just `false`.
//! 12. `!false`, which can be replaced with just `true`.
//!
//! Note also that rules 1 through 6 have both LHS and RHS version

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{ExpData, Operation, Value},
    model::FunctionEnv,
};

#[derive(Default)]
pub struct NonminimalBool;

impl NonminimalBool {
    // Converts a binary boolean operator to its string representation
    fn name_of_op(&self, cmp: &Operation) -> &'static str {
        match cmp {
            Operation::And => "&&",
            Operation::Or => "||",
            Operation::Implies => "==>",
            Operation::Iff => "<==>",
            _ => unreachable!("Unexpected operation"),
        }
    }

    // Returns the message for a binary boolean operator with a literal
    // cmp is the boolean operator
    // b is the boolean literal, true or false
    // literal_is_lhs is true if literal is on the left and false if it is on the right
    // lhs and rhs are the literal and "bexpr" (based on the side of the literal)
    fn get_msg(
        &self,
        cmp: &Operation,
        b: bool,
        literal_is_lhs: bool,
        lhs: String,
        rhs: String,
    ) -> Option<String> {
        use Operation::*;
        let equiv = match (cmp, b, literal_is_lhs) {
            (Or, true, _) | (Implies, true, false) | (Implies, false, true) => Some("`true`"),
            (And, false, _) => Some("`false`"),
            (And, true, _) | (Or, false, _) | (Iff, true, _) | (Implies, true, true) => {
                Some("`bexpr`")
            },
            (Iff, false, _) | (Implies, false, false) => Some("the negation of `bexpr`"),
            _ => None,
        };
        equiv.map(|s| {
            format!(
                "The {}-hand side of `{}` evaluates to `{}`. Recall that the expression `{} {} {}` is logically equivalent to {}. Consider simplifying.",
                if literal_is_lhs { "left" } else { "right" },
                self.name_of_op(cmp),
                b,
                lhs,
                self.name_of_op(cmp),
                rhs,
                s
            )
        })
    }
}

impl ExpChecker for NonminimalBool {
    fn get_name(&self) -> String {
        "nonminimal_bool".to_string()
    }

    fn visit_expr_pre(&mut self, function: &FunctionEnv, expr: &ExpData) {
        use ExpData::{Call, Value as ExpValue};
        use Operation::*;
        use Value::Bool;

        let Call(_, cmp, args) = expr else {
            return;
        };
        let msg = match cmp {
            And | Or | Implies | Iff => {
                match (args[0].as_ref(), args[1].as_ref()) {
                    // When one of the arguments is a boolean literal (true or false)
                    (ExpValue(_, Bool(b)), _) => self.get_msg(cmp, *b, true, b.to_string(), "bexpr".to_string()),
                    (_, ExpValue(_, Bool(b))) => self.get_msg(cmp, *b, false, "bexpr".to_string(), b.to_string()),
                    _ => None,
                }
            },
            Not => match args[0].as_ref() {
                ExpValue(_, Bool(b)) => {
                    Some(format!("This expression evaluates to `{}`. Recall that the expression `!{}` is logically equivalent to `{}`. Consider simplifying.", !b, b, !b))
                },
                _ => None,
            },
            _ => None,
        };

        if let Some(msg) = msg {
            let env = function.env();
            self.report(env, &env.get_node_loc(expr.node_id()), &msg);
        }
    }
}
