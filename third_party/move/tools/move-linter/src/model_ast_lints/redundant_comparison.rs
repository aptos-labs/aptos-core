// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
//! This module detects redundant, contradictory, and tautological
//! numerical comparisons over the same variable in boolean expressions
//! combined with `&&` and `||`.

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{Exp, ExpData, Operation, Value},
    model::FunctionEnv,
};
use Operation::*;

#[derive(Debug, Clone, Copy)]
enum RuleType {
    Redundant,
    Contradiction,
    Tautology,
}

#[derive(Debug)]
struct ComparisonRule {
    left_op: Operation,
    right_op: Operation,
    condition: fn(i64, i64) -> bool,
    relationship: RuleType,
    applies_to: Operation,
}

#[derive(Default)]
pub struct RedundantComparison;

impl ExpChecker for RedundantComparison {
    fn get_name(&self) -> String {
        "redundant_comparison".to_string()
    }

    fn visit_expr_pre(&mut self, function_env: &FunctionEnv, expr: &ExpData) {
        match expr {
            ExpData::Call(_, And, args) => {
                if let [left, right] = &args[..] {
                    self.check_logical_expression(function_env, left, right, And);
                }
            },
            ExpData::Call(_, Or, args) => {
                if let [left, right] = &args[..] {
                    self.check_logical_expression(function_env, left, right, Or);
                }
            },
            _ => {},
        }
    }
}

const COMPARISON_RULES: &[ComparisonRule] = &[
    //1: Redundant with && — Le + Lt (x <= 400 && x < 500)
    ComparisonRule {
        left_op: Le,
        right_op: Lt,
        condition: |left_val, right_val| left_val < right_val,
        relationship: RuleType::Redundant,
        applies_to: And,
    },
    // 2: Contradiction with && — Le + Gt (x <= 400 && x > 500)
    ComparisonRule {
        left_op: Le,
        right_op: Gt,
        condition: |left_val, right_val| left_val < right_val,
        relationship: RuleType::Contradiction,
        applies_to: And,
    },
    // 3: Redundant with || — Gt + Ge (x > 10 || x >= 5)
    ComparisonRule {
        left_op: Gt,
        right_op: Ge,
        condition: |left_val, right_val| left_val >= right_val,
        relationship: RuleType::Redundant,
        applies_to: Or,
    },
    // 4: Redundant with || — Lt + Le (x < 5 || x <= 10)
    ComparisonRule {
        left_op: Lt,
        right_op: Le,
        condition: |left_val, right_val| left_val <= right_val,
        relationship: RuleType::Redundant,
        applies_to: Or,
    },
    // 5: Tautology with || — Lt + Ge (x < 5 || x >= 5)
    ComparisonRule {
        left_op: Lt,
        right_op: Ge,
        condition: |left_val, right_val| left_val <= right_val,
        relationship: RuleType::Tautology,
        applies_to: Or,
    },
    // 6: Tautology with || — Le + Gt (x <= 5 || x > 5)
    ComparisonRule {
        left_op: Le,
        right_op: Gt,
        condition: |left_val, right_val| left_val <= right_val,
        relationship: RuleType::Tautology,
        applies_to: Or,
    },
    // 7: Redundant with && — Eq + Lt (x == 5 && x < 10)
    ComparisonRule {
        left_op: Eq,
        right_op: Lt,
        condition: |left_val, right_val| left_val < right_val,
        relationship: RuleType::Redundant,
        applies_to: And,
    },
    // 8: Redundant with && — Eq + Le (x == 5 && x <= 5)
    ComparisonRule {
        left_op: Eq,
        right_op: Le,
        condition: |left_val, right_val| left_val <= right_val,
        relationship: RuleType::Redundant,
        applies_to: And,
    },
    // 9: Redundant with && — Eq + Gt (x == 5 && x > 3)
    ComparisonRule {
        left_op: Eq,
        right_op: Gt,
        condition: |left_val, right_val| left_val > right_val,
        relationship: RuleType::Redundant,
        applies_to: And,
    },
    // 10: Redundant with && — Eq + Ge (x == 5 && x >= 0)
    ComparisonRule {
        left_op: Eq,
        right_op: Ge,
        condition: |left_val, right_val| left_val >= right_val,
        relationship: RuleType::Redundant,
        applies_to: And,
    },
    // 11: Redundant with && — Eq + Neq (x == 5 && x != 6)
    ComparisonRule {
        left_op: Eq,
        right_op: Neq,
        condition: |left_val, right_val| left_val != right_val,
        relationship: RuleType::Redundant,
        applies_to: And,
    },
    // 12: Contradiction with && — Eq + Neq (x == 5 && x != 5)
    ComparisonRule {
        left_op: Eq,
        right_op: Neq,
        condition: |left_val, right_val| left_val == right_val,
        relationship: RuleType::Contradiction,
        applies_to: And,
    },
    // 13: Redundant with && — Lt + Neq (x < 10 && x != 10)
    ComparisonRule {
        left_op: Lt,
        right_op: Neq,
        condition: |left_val, right_val| left_val == right_val,
        relationship: RuleType::Redundant,
        applies_to: And,
    },
    // 14: Redundant with && — Gt + Neq (x > 10 && x != 10)
    ComparisonRule {
        left_op: Gt,
        right_op: Neq,
        condition: |left_val, right_val| left_val == right_val,
        relationship: RuleType::Redundant,
        applies_to: And,
    },
    // 15: Redundant with && — Le + Le (x <= 5 && x <= 5)
    ComparisonRule {
        left_op: Le,
        right_op: Le,
        condition: |left_val, right_val| left_val <= right_val,
        relationship: RuleType::Redundant,
        applies_to: And,
    },
    // 16: Redundant with && — Lt + Lt (x < 5 && x < 5)
    ComparisonRule {
        left_op: Lt,
        right_op: Lt,
        condition: |left_val, right_val| left_val <= right_val,
        relationship: RuleType::Redundant,
        applies_to: And,
    },
    // 17: Redundant with && — Ge + Ge (x >= 5 && x >= 5)
    ComparisonRule {
        left_op: Ge,
        right_op: Ge,
        condition: |left_val, right_val| left_val >= right_val,
        relationship: RuleType::Redundant,
        applies_to: And,
    },
    // 18: Redundant with && — Gt + Gt (x > 5 && x > 5)
    ComparisonRule {
        left_op: Gt,
        right_op: Gt,
        condition: |left_val, right_val| left_val >= right_val,
        relationship: RuleType::Redundant,
        applies_to: And,
    },
    // 19: Contradiction with && — Lt + Ge (x < 10 && x >= 10)
    ComparisonRule {
        left_op: Lt,
        right_op: Ge,
        condition: |left_val, right_val| left_val <= right_val,
        relationship: RuleType::Contradiction,
        applies_to: And,
    },
    // 20: Contradiction with && — Eq + Eq (x == 5 && x == 6)
    ComparisonRule {
        left_op: Eq,
        right_op: Eq,
        condition: |left_val, right_val| left_val != right_val,
        relationship: RuleType::Contradiction,
        applies_to: And,
    },
    // 21: Redundant with && — Ge + Gt (x >= 10 && x > 5)
    ComparisonRule {
        left_op: Ge,
        right_op: Gt,
        condition: |left_val, right_val| left_val > right_val,
        relationship: RuleType::Redundant,
        applies_to: And,
    },
    // 22: Redundant with && — Neq + Neq (x != 5 && x != 5)
    ComparisonRule {
        left_op: Neq,
        right_op: Neq,
        condition: |left_val, right_val| left_val == right_val,
        relationship: RuleType::Redundant,
        applies_to: And,
    },
];

impl RedundantComparison {
    fn check_logical_expression(
        &mut self,
        function_env: &FunctionEnv,
        left: &Exp,
        right: &Exp,
        logical_op: Operation,
    ) {
        // Extract comparison info from both sides
        if let (Some(left_comp), Some(right_comp)) = (
            self.extract_comparison(left),
            self.extract_comparison(right),
        ) {
            // Check if they're comparing the same variable
            if self.same_variable(&left_comp.0, &right_comp.0) {
                self.apply_rules(
                    function_env,
                    &left_comp,
                    &right_comp,
                    left,
                    right,
                    logical_op,
                );
            }
        }
    }

    fn parse_number(exp: &ExpData) -> Option<i64> {
        if let ExpData::Value(_, Value::Number(n)) = exp {
            n.to_string().parse().ok()
        } else {
            None
        }
    }

    fn extract_comparison(&self, expr: &move_model::ast::Exp) -> Option<(ExpData, Operation, i64)> {
        if let ExpData::Call(_, op, args) = expr.as_ref() {
            if args.len() != 2 {
                return None;
            }
            match (
                Self::parse_number(args[0].as_ref()),
                Self::parse_number(args[1].as_ref()),
            ) {
                (None, Some(num)) => Some((args[0].as_ref().clone(), op.clone(), num)),
                (Some(num), None) => Some((args[1].as_ref().clone(), Self::flip_op(op)?, num)),
                _ => None,
            }
        } else {
            None
        }
    }

    fn flip_op(op: &Operation) -> Option<Operation> {
        Some(match op {
            Lt => Gt,
            Le => Ge,
            Gt => Lt,
            Ge => Le,
            Eq => Eq,
            Neq => Neq,
            _ => return None,
        })
    }

    fn same_variable(&self, var1: &ExpData, var2: &ExpData) -> bool {
        match (var1, var2) {
            (ExpData::LocalVar(_, s1), ExpData::LocalVar(_, s2)) => s1 == s2,
            (ExpData::Temporary(_, t1), ExpData::Temporary(_, t2)) => t1 == t2,
            _ => false,
        }
    }

    fn apply_rules(
        &mut self,
        function_env: &FunctionEnv,
        left_comp: &(ExpData, Operation, i64),
        right_comp: &(ExpData, Operation, i64),
        left_expr: &Exp,
        right_expr: &Exp,
        logical_op: Operation,
    ) {
        let (_, left_op, left_val) = left_comp;
        let (_, right_op, right_val) = right_comp;
        let env = function_env.env();

        let report = |msg: String, expr: &Exp| {
            self.report(env, &env.get_node_loc(expr.node_id()), &msg);
        };

        // Apply rules that match the current logical operation
        for rule in COMPARISON_RULES {
            if rule.applies_to == logical_op {
                // Check if rule matches the operation pattern
                if rule.left_op == *left_op
                    && rule.right_op == *right_op
                    && (rule.condition)(*left_val, *right_val)
                {
                    let msg = match rule.relationship {
                        RuleType::Redundant => {
                            format!(
                                "Redundant comparison: x {} {} is implied by x {} {}",
                                self.op_to_string(&rule.right_op),
                                right_val,
                                self.op_to_string(&rule.left_op),
                                left_val
                            )
                        },
                        RuleType::Contradiction => {
                            format!(
                                "Contradiction: x {} {} and x {} {}",
                                self.op_to_string(left_op),
                                left_val,
                                self.op_to_string(right_op),
                                right_val
                            )
                        },
                        RuleType::Tautology => "Tautology: condition is always true".to_string(),
                    };

                    report(msg, right_expr);
                    return;
                }
                // Check reverse pattern
                else if rule.left_op == *right_op
                    && rule.right_op == *left_op
                    && (rule.condition)(*right_val, *left_val)
                {
                    let msg = match rule.relationship {
                        RuleType::Redundant => {
                            format!(
                                "Redundant comparison: x {} {} is implied by x {} {}",
                                self.op_to_string(&rule.right_op),
                                left_val,
                                self.op_to_string(&rule.left_op),
                                right_val
                            )
                        },
                        RuleType::Contradiction => {
                            format!(
                                "Contradiction: x {} {} and x {} {}",
                                self.op_to_string(left_op),
                                left_val,
                                self.op_to_string(right_op),
                                right_val
                            )
                        },
                        RuleType::Tautology => "Tautology: condition is always true".to_string(),
                    };

                    report(msg, left_expr);
                    return;
                }
            }
        }
    }

    fn op_to_string(&self, op: &Operation) -> &'static str {
        match op {
            Lt => "<",
            Le => "<=",
            Gt => ">",
            Ge => ">=",
            Eq => "==",
            Neq => "!=",
            _ => "?",
        }
    }
}
