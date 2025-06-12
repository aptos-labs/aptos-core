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
    ast::{Exp, ExpData, Operation},
    model::GlobalEnv,
};
use ExpData::Call;
use Operation::{And, Not, Or};

enum SimplerBoolPatternType {
    AbsorptionLaw,
    Idempotence,
    Contradiction,
    Tautology,
    DistributiveLaw,
}

impl SimplerBoolPatternType {
    fn to_message(&self) -> &str {
        match self {
            SimplerBoolPatternType::AbsorptionLaw => "This boolean expression can be simplified using absorption law. The expression `a && b || a` is equivalent to just `a`.",
            SimplerBoolPatternType::Idempotence => "This boolean expression can be simplified using idempotence. The expression `a && a` is equivalent to just `a`.",
            SimplerBoolPatternType::Contradiction => "This boolean expression can be simplified using contradiction. The expression `a && !a` is equivalent to just `false`.",
            SimplerBoolPatternType::Tautology => "This boolean expression can be simplified using tautology. The expression `a || !a` is equivalent to just `true`.",
            SimplerBoolPatternType::DistributiveLaw => "This boolean expression can be simplified using distributive law. The expression `(a && b) || (a && c)` is equivalent to `a && (b || c)`.",
        }
    }
}

#[derive(Default)]
pub struct SimplerBoolExpression;

fn is_move_function(expr: &ExpData) -> bool {
    matches!(expr, ExpData::Call(_, Operation::MoveFunction(_, _), _))
}

fn is_constant(expr: &ExpData) -> bool {
    matches!(expr, ExpData::Value(_, _))
}

/// Check if two expressions are structurally equal
fn is_expression_equal(expr1: &ExpData, expr2: &ExpData) -> bool {
    match (expr1, expr2) {
        (ExpData::LocalVar(_, s1), ExpData::LocalVar(_, s2)) => s1 == s2,
        (ExpData::Value(_, v1), ExpData::Value(_, v2)) => v1 == v2,
        (ExpData::Temporary(_, t1), ExpData::Temporary(_, t2)) => t1 == t2,
        (ExpData::Call(_, op1, args1), ExpData::Call(_, op2, args2)) => {
            if is_move_function(expr1)
                || is_move_function(expr2)
                || args1.iter().any(|a| is_move_function(a))
                || args2.iter().any(|a| is_move_function(a))
            {
                return false;
            }

            op1 == op2
                && args1.len() == args2.len()
                && args1
                    .iter()
                    .zip(args2.iter())
                    .all(|(a1, a2)| is_expression_equal(a1, a2))
        },
        _ => false,
    }
}

impl SimplerBoolExpression {
    /// Check for absorption law patterns: `a && b || a` or `a || a && b`
    fn check_absorption_law(
        &self,
        left: &ExpData,
        right: &ExpData,
    ) -> Option<SimplerBoolPatternType> {
        let check_pattern = |left: &ExpData, right: &ExpData| {
            if let ExpData::Call(_, And, and_args) = left {
                if and_args.len() == 2 {
                    return is_expression_equal(and_args[0].as_ref(), right)
                        || is_expression_equal(and_args[1].as_ref(), right);
                }
            }
            false
        };

        if check_pattern(left, right) {
            // Pattern 1: (a && b) || a  →  a
            Some(SimplerBoolPatternType::AbsorptionLaw)
        } else if check_pattern(right, left) {
            // Pattern 2: a || (a && b)  →  a
            Some(SimplerBoolPatternType::AbsorptionLaw)
        } else {
            None
        }
    }

    /// Check for idempotence patterns: `a && a` or `a || a`
    fn check_idempotence(&self, left: &ExpData, right: &ExpData) -> Option<SimplerBoolPatternType> {
        // If left or right is a constant or known value, we ignore this since it's already implemented in `nonminimal_bool`
        if is_constant(left) || is_constant(right) {
            return None;
        }

        if is_expression_equal(left, right) {
            return Some(SimplerBoolPatternType::Idempotence);
        }
        None
    }

    /// Check for contradiction and tautology patterns: `a && !a` or `a || !a`
    fn check_contradiction_tautology(
        &self,
        op: &Operation,
        left: &ExpData,
        right: &ExpData,
    ) -> Option<SimplerBoolPatternType> {
        let is_negation_pair = |expr1: &ExpData, expr2: &ExpData| -> bool {
            matches!(expr2, ExpData::Call(_, Not, not_args)
                if not_args.len() == 1 && is_expression_equal(expr1, not_args[0].as_ref()))
        };

        // If left or right is a constant or known value, we ignore this since it's already implemented in `nonminimal_bool`
        if is_constant(left) || is_constant(right) {
            return None;
        }

        if is_negation_pair(left, right) || is_negation_pair(right, left) {
            let result_value = matches!(op, Or);
            Some(if result_value {
                SimplerBoolPatternType::Tautology
            } else {
                SimplerBoolPatternType::Contradiction
            })
        } else {
            None
        }
    }

    /// Check for distributive law patterns: `(a && b) || (a && c)` or `(a || b) && (a || c)`
    fn check_distributive_law(
        &self,
        op: &Operation,
        left: &ExpData,
        right: &ExpData,
    ) -> Option<SimplerBoolPatternType> {
        let try_distribute =
            |left_args: &[Exp], right_args: &[Exp]| -> Option<SimplerBoolPatternType> {
                if left_args.len() != 2 || right_args.len() != 2 {
                    return None;
                }

                for left_elem in left_args.iter() {
                    for right_elem in right_args.iter() {
                        if is_expression_equal(left_elem.as_ref(), right_elem.as_ref()) {
                            return Some(SimplerBoolPatternType::DistributiveLaw);
                        }
                    }
                }
                None
            };

        match (op, left, right) {
            (Or, ExpData::Call(_, And, left_args), ExpData::Call(_, And, right_args))
            | (And, ExpData::Call(_, Or, left_args), ExpData::Call(_, Or, right_args)) => {
                try_distribute(left_args, right_args)
            },
            _ => None,
        }
    }
}

impl ExpChecker for SimplerBoolExpression {
    fn get_name(&self) -> String {
        "simpler_bool_expression".to_string()
    }

    fn visit_expr_pre(&mut self, env: &GlobalEnv, expr: &ExpData) {
        let Call(id, op, args) = expr else { return };

        let pattern = match op {
            And | Or if args.len() == 2 => {
                let (left, right) = (&args[0].as_ref(), &args[1].as_ref());

                // For Or operations, try absorption law first
                let absorption = if matches!(op, Or) {
                    self.check_absorption_law(left, right)
                } else {
                    None
                };

                absorption
                    .or_else(|| self.check_idempotence(left, right))
                    .or_else(|| self.check_contradiction_tautology(op, left, right))
                    .or_else(|| self.check_distributive_law(op, left, right))
            },
            _ => None,
        };

        if let Some(pattern) = pattern {
            self.report(env, &env.get_node_loc(*id), pattern.to_message());
        }
    }
}
