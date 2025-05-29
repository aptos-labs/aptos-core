// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for boolean expressions
//! that can be simplified using boolean algebra laws.
//!
//! Currently detects:
//! 1. `a && b || a` which can be simplified to just `a` (absorption law)
//! 2. `a || a && b` which can be simplified to just `a` (absorption law)
//! 3. `a && a` which can be simplified to just `a` (idempotence)
//! 4. `a || a` which can be simplified to just `a` (idempotence)
//! 5. `a && !a` which can be simplified to just `false` (contradiction)
//! 6. `!a && a` which can be simplified to just `false` (contradiction)
//! 7. `a || !a` which can be simplified to just `true` (tautology)
//! 8. `!a || a` which can be simplified to just `true` (tautology)
//! 9. `(a && b) || (a && c)` which can be simplified to `a && (b || c)` (distributive law)
//! 10. `(a || b) && (a || c)` which can be simplified to `a || (b && c)` (distributive law)

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{Exp, ExpData, Operation, Value},
    model::GlobalEnv,
};
use ExpData::Call;
use Operation::{And, Not, Or};

fn get_text_from_expr(env: &GlobalEnv, exp_data: &ExpData) -> Option<String> {
    let loc = env.get_node_loc(exp_data.node_id());
    let file_id = loc.file_id();
    let source = env.get_file_source(file_id);
    let start = loc.span().start().to_usize();
    let end = loc.span().end().to_usize();
    if start <= end && end <= source.len() {
        return Some(source[start..end].to_string());
    }
    None
}

struct SimplifiablePattern {
    original_expr: String,
    simplified_expr: String,
}

impl SimplifiablePattern {
    fn new(env: &GlobalEnv, original: &ExpData, simplified: &ExpData) -> Self {
        Self {
            original_expr: get_text_from_expr(env, original).unwrap_or_default(),
            simplified_expr: get_text_from_expr(env, simplified).unwrap_or_default(),
        }
    }

    fn new_with_text(env: &GlobalEnv, original: &ExpData, simplified_text: String) -> Self {
        Self {
            original_expr: get_text_from_expr(env, original).unwrap_or_default(),
            simplified_expr: simplified_text,
        }
    }

    fn to_message(&self) -> String {
        format!(
            "This boolean expression can be simplified. The expression `{}` is equivalent to just `{}`. Consider replacing with the simpler form.",
            self.original_expr, self.simplified_expr
        )
    }
}

#[derive(Default)]
pub struct SimplifiableBooleanExpression;

impl SimplifiableBooleanExpression {
    /// Check if two expressions are structurally equal
    fn is_expression_equal(&self, expr1: &ExpData, expr2: &ExpData) -> bool {
        match (expr1, expr2) {
            (ExpData::LocalVar(_, s1), ExpData::LocalVar(_, s2)) => s1 == s2,
            (ExpData::Value(_, v1), ExpData::Value(_, v2)) => v1 == v2,
            (ExpData::Temporary(_, t1), ExpData::Temporary(_, t2)) => t1 == t2,
            _ => false,
        }
    }

    /// Check for absorption law patterns: `a && b || a` or `a || a && b`
    fn check_absorption_law(
        &self,
        env: &GlobalEnv,
        expr: &ExpData,
        left: &ExpData,
        right: &ExpData,
    ) -> Option<SimplifiablePattern> {
        let check_pattern = |left: &ExpData, right: &ExpData| {
            if let ExpData::Call(_, And, and_args) = left {
                if and_args.len() == 2 {
                    return self.is_expression_equal(and_args[0].as_ref(), right)
                        || self.is_expression_equal(and_args[1].as_ref(), right);
                }
            }
            false
        };

        if check_pattern(left, right) {
            // Pattern 1: (a && b) || a  →  a
            Some(SimplifiablePattern::new(env, expr, right))
        } else if check_pattern(right, left) {
            // Pattern 2: a || (a && b)  →  a
            Some(SimplifiablePattern::new(env, expr, left))
        } else {
            None
        }
    }

    /// Check for idempotence patterns: `a && a` or `a || a`
    fn check_idempotence(
        &self,
        env: &GlobalEnv,
        expr: &ExpData,
        left: &ExpData,
        right: &ExpData,
    ) -> Option<SimplifiablePattern> {
        if self.is_expression_equal(left, right) {
            return Some(SimplifiablePattern::new(env, expr, left));
        }
        None
    }

    /// Check for contradiction and tautology patterns: `a && !a` or `a || !a`
    fn check_contradiction_tautology(
        &self,
        env: &GlobalEnv,
        expr: &ExpData,
        op: &Operation,
        left: &ExpData,
        right: &ExpData,
    ) -> Option<SimplifiablePattern> {
        let is_negation_pair = |expr1: &ExpData, expr2: &ExpData| -> bool {
            matches!(expr2, ExpData::Call(_, Not, not_args)
                if not_args.len() == 1 && self.is_expression_equal(expr1, not_args[0].as_ref()))
        };

        if is_negation_pair(left, right) || is_negation_pair(right, left) {
            let result_value = matches!(op, Or);
            let result_expr = ExpData::Value(env.new_node_id(), Value::Bool(result_value));
            Some(SimplifiablePattern::new(env, expr, &result_expr))
        } else {
            None
        }
    }

    /// Check for distributive law patterns: `(a && b) || (a && c)` or `(a || b) && (a || c)`
    fn check_distributive_law(
        &self,
        env: &GlobalEnv,
        expr: &ExpData,
        op: &Operation,
        left: &ExpData,
        right: &ExpData,
    ) -> Option<SimplifiablePattern> {
        let try_distribute = |outer_op: Operation,
                              inner_op: Operation,
                              left_args: &[Exp],
                              right_args: &[Exp]|
         -> Option<SimplifiablePattern> {
            if left_args.len() != 2 || right_args.len() != 2 {
                return None;
            }

            for (i, left_elem) in left_args.iter().enumerate() {
                for (j, right_elem) in right_args.iter().enumerate() {
                    if self.is_expression_equal(left_elem.as_ref(), right_elem.as_ref()) {
                        let other_left = &left_args[1 - i];
                        let other_right = &right_args[1 - j];

                        if let (Some(common_text), Some(left_text), Some(right_text)) = (
                            get_text_from_expr(env, left_elem.as_ref()),
                            get_text_from_expr(env, other_left.as_ref()),
                            get_text_from_expr(env, other_right.as_ref()),
                        ) {
                            let inner_op_str = match inner_op {
                                And => " && ",
                                Or => " || ",
                                _ => return None,
                            };
                            let outer_op_str = match outer_op {
                                And => " && ",
                                Or => " || ",
                                _ => return None,
                            };

                            let simplified_text = format!(
                                "{}{}({}{}{})",
                                common_text, outer_op_str, left_text, inner_op_str, right_text
                            );

                            return Some(SimplifiablePattern::new_with_text(
                                env,
                                expr,
                                simplified_text,
                            ));
                        }
                    }
                }
            }
            None
        };

        match (op, left, right) {
            (Or, ExpData::Call(_, And, left_args), ExpData::Call(_, And, right_args)) => {
                try_distribute(And, Or, left_args, right_args)
            },
            (And, ExpData::Call(_, Or, left_args), ExpData::Call(_, Or, right_args)) => {
                try_distribute(Or, And, left_args, right_args)
            },
            _ => None,
        }
    }
}

impl ExpChecker for SimplifiableBooleanExpression {
    fn get_name(&self) -> String {
        "simplifiable_boolean_expression".to_string()
    }

    fn visit_expr_pre(&mut self, env: &GlobalEnv, expr: &ExpData) {
        let Call(id, op, args) = expr else { return };

        let pattern = match op {
            And | Or if args.len() == 2 => {
                let (left, right) = (&args[0].as_ref(), &args[1].as_ref());

                // For Or operations, try absorption law first
                let absorption = if matches!(op, Or) {
                    self.check_absorption_law(env, expr, left, right)
                } else {
                    None
                };

                absorption
                    .or_else(|| self.check_idempotence(env, expr, left, right))
                    .or_else(|| self.check_contradiction_tautology(env, expr, op, left, right))
                    .or_else(|| self.check_distributive_law(env, expr, op, left, right))
            },
            _ => None,
        };

        if let Some(pattern) = pattern {
            self.report(env, &env.get_node_loc(*id), &pattern.to_message());
        }
    }
}
