// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for redundant boolean expressions
//! that can be simplified using boolean algebra laws.
//!
//! Currently detects:
//! 1. `a && b || a` which can be simplified to just `a` (absorption law)
//! 2. `a || a && b` which can be simplified to just `a` (absorption law)
//! 3. `a && a` which can be simplified to just `a` (idempotence)
//! 4. `a || a` which can be simplified to just `a` (idempotence)
//! 5. `a && true` which can be simplified to just `a` (identity law)
//! 6. `true && a` which can be simplified to just `a` (identity law)
//! 7. `a || false` which can be simplified to just `a` (identity law)
//! 8. `false || a` which can be simplified to just `a` (identity law)
//! 9. `a && false` which can be simplified to just `false` (annihilation law)
//! 10. `false && a` which can be simplified to just `false` (annihilation law)
//! 11. `a || true` which can be simplified to just `true` (annihilation law)
//! 12. `true || a` which can be simplified to just `true` (annihilation law)
//! 13. `a && !a` which can be simplified to just `false` (contradiction)
//! 14. `!a && a` which can be simplified to just `false` (contradiction)
//! 15. `a || !a` which can be simplified to just `true` (tautology)
//! 16. `!a || a` which can be simplified to just `true` (tautology)

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{Exp, ExpData, Operation, Value},
    model::GlobalEnv,
};

#[derive(Default)]
pub struct RedundantBooleanExpression;

/// Convert an expression to a readable string representation
fn display_exp(env: &GlobalEnv, exp: &ExpData) -> String {
    match exp {
        ExpData::LocalVar(_, s) => s.display(env.symbol_pool()).to_string(),
        ExpData::Temporary(..) => exp.display(env).to_string(),
        ExpData::Value(_, Value::Bool(b)) => b.to_string(),
        ExpData::Value(_, Value::Number(n)) => n.to_string(),
        _ => "?".to_string(),
    }
}

/// Detected redundant boolean pattern
struct RedundantPattern {
    original_expr: String,
    simplified_expr: String,
}

impl RedundantPattern {
    fn new(env: &GlobalEnv, original: &ExpData, simplified: &ExpData) -> Self {
        Self {
            original_expr: display_exp(env, original),
            simplified_expr: display_exp(env, simplified),
        }
    }

    fn to_message(&self) -> String {
        format!(
            "This boolean expression can be simplified. The expression `{}` is equivalent to just `{}`. Consider replacing with the simpler form.",
            self.original_expr, self.simplified_expr
        )
    }
}

impl RedundantBooleanExpression {
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
        op: &Operation,
        args: &[Exp],
    ) -> Option<RedundantPattern> {
        use Operation::{And, Or};

        // We only care about Or operations with exactly 2 arguments
        if !matches!(op, Or) || args.len() != 2 {
            return None;
        }

        let (left, right) = (&args[0].as_ref(), &args[1].as_ref());

        let check_pattern = |left: &ExpData, right: &ExpData| {
            if let ExpData::Call(_, And, and_args) = left {
                if and_args.len() == 2 {
                    return self.is_expression_equal(and_args[0].as_ref(), right)
                        || self.is_expression_equal(and_args[1].as_ref(), right);
                }
            }
            false
        };

        // Pattern 1: (a && b) || a  →  a
        if check_pattern(left, right) {
            return Some(RedundantPattern::new(env, expr, right));
        }

        // Pattern 2: a || (a && b)  →  a
        if check_pattern(right, left) {
            return Some(RedundantPattern::new(env, expr, left));
        }

        None
    }

    /// Check for idempotence patterns: `a && a` or `a || a`
    fn check_idempotence(
        &self,
        env: &GlobalEnv,
        expr: &ExpData,
        op: &Operation,
        args: &[Exp],
    ) -> Option<RedundantPattern> {
        use Operation::{And, Or};

        // We only care about And/Or operations with exactly 2 arguments
        if !matches!(op, And | Or) || args.len() != 2 {
            return None;
        }

        let (left, right) = (&args[0].as_ref(), &args[1].as_ref());

        // Check if both sides are the same expression: a && a → a, a || a → a
        if self.is_expression_equal(left, right) {
            return Some(RedundantPattern::new(env, expr, left));
        }

        None
    }

    /// Check for identity law patterns: `a && true` or `true && a` or `a || false` or `false || a`
    fn check_identity(
        &self,
        env: &GlobalEnv,
        expr: &ExpData,
        op: &Operation,
        args: &[Exp],
    ) -> Option<RedundantPattern> {
        use Operation::{And, Or};

        // We only care about And/Or operations with exactly 2 arguments
        if !matches!(op, And | Or) || args.len() != 2 {
            return None;
        }

        let (left, right) = (&args[0].as_ref(), &args[1].as_ref());

        match op {
            And => {
                // Pattern: a && true → a
                if let ExpData::Value(_, Value::Bool(true)) = right {
                    return Some(RedundantPattern::new(env, expr, left));
                }
                // Pattern: true && a → a
                if let ExpData::Value(_, Value::Bool(true)) = left {
                    return Some(RedundantPattern::new(env, expr, right));
                }
            },
            Or => {
                // Pattern: a || false → a
                if let ExpData::Value(_, Value::Bool(false)) = right {
                    return Some(RedundantPattern::new(env, expr, left));
                }
                // Pattern: false || a → a
                if let ExpData::Value(_, Value::Bool(false)) = left {
                    return Some(RedundantPattern::new(env, expr, right));
                }
            },
            _ => {},
        }

        None
    }

    /// Check for annihilation law patterns: `a && false` or `false && a` or `a || true` or `true || a`
    fn check_annihilation(
        &self,
        env: &GlobalEnv,
        expr: &ExpData,
        op: &Operation,
        args: &[Exp],
    ) -> Option<RedundantPattern> {
        use Operation::{And, Or};

        // We only care about And/Or operations with exactly 2 arguments
        if !matches!(op, And | Or) || args.len() != 2 {
            return None;
        }

        let (left, right) = (&args[0].as_ref(), &args[1].as_ref());

        match op {
            And => {
                // Pattern: a && false → false
                if let ExpData::Value(_, Value::Bool(false)) = right {
                    return Some(RedundantPattern::new(env, expr, right));
                }
                // Pattern: false && a → false
                if let ExpData::Value(_, Value::Bool(false)) = left {
                    return Some(RedundantPattern::new(env, expr, left));
                }
            },
            Or => {
                // Pattern: a || true → true
                if let ExpData::Value(_, Value::Bool(true)) = right {
                    return Some(RedundantPattern::new(env, expr, right));
                }
                // Pattern: true || a → true
                if let ExpData::Value(_, Value::Bool(true)) = left {
                    return Some(RedundantPattern::new(env, expr, left));
                }
            },
            _ => {},
        }

        None
    }

    /// Check for contradiction and tautology patterns: `a && !a` or `a || !a`
    fn check_contradiction_tautology(
        &self,
        env: &GlobalEnv,
        expr: &ExpData,
        op: &Operation,
        args: &[Exp],
    ) -> Option<RedundantPattern> {
        use Operation::{And, Not, Or};

        // We only care about And/Or operations with exactly 2 arguments
        if !matches!(op, And | Or) || args.len() != 2 {
            return None;
        }

        let (left, right) = (&args[0].as_ref(), &args[1].as_ref());

        // Helper function to check if one expression is the negation of the other
        let is_negation_of = |expr1: &ExpData, expr2: &ExpData| -> bool {
            match expr2 {
                ExpData::Call(_, Not, not_args) if not_args.len() == 1 => {
                    self.is_expression_equal(expr1, not_args[0].as_ref())
                },
                _ => false,
            }
        };

        match op {
            And => {
                // Pattern: a && !a → false
                if is_negation_of(left, right) || is_negation_of(right, left) {
                    let false_expr = ExpData::Value(env.new_node_id(), Value::Bool(false));
                    return Some(RedundantPattern::new(env, expr, &false_expr));
                }
            },
            Or => {
                // Pattern: a || !a → true
                if is_negation_of(left, right) || is_negation_of(right, left) {
                    let true_expr = ExpData::Value(env.new_node_id(), Value::Bool(true));
                    return Some(RedundantPattern::new(env, expr, &true_expr));
                }
            },
            _ => {},
        }

        None
    }
}

impl ExpChecker for RedundantBooleanExpression {
    fn get_name(&self) -> String {
        "redundant_boolean_expression".to_string()
    }

    fn visit_expr_pre(&mut self, env: &GlobalEnv, expr: &ExpData) {
        use ExpData::Call;

        // Only analyze Call expressions
        let Call(id, op, args) = expr else {
            return;
        };

        let patterns = vec![
            self.check_absorption_law(env, expr, op, args),
            self.check_idempotence(env, expr, op, args),
            self.check_identity(env, expr, op, args),
            self.check_annihilation(env, expr, op, args),
            self.check_contradiction_tautology(env, expr, op, args),
        ];

        patterns.iter().for_each(|pattern| {
            if let Some(pattern) = pattern {
                self.report(env, &env.get_node_loc(*id), &pattern.to_message());
            }
        });
    }
}
