// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for expressions that look
//! like overflow checks done in a C style. This pattern in move does not make sense,
//! as it either aborts immediately or is always `true` or `false`.
//! E.g.
//!      `a > a + b`  => Always false if (a + b) does not overflow, else abort.
//!      `a < a + b`  => Always true if (a + b) does not overflow, else abort.
use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{Exp, ExpData, Operation},
    model::FunctionEnv,
};

const MSG: &str =
    "This looks like a C-style overflow check. In Move, overflows abort, and such checks are unnecessary.";

#[derive(Default)]

pub struct AbortingOverflowChecks;

impl ExpChecker for AbortingOverflowChecks {
    fn get_name(&self) -> String {
        "aborting_overflow_checks".to_string()
    }

    fn visit_expr_pre(&mut self, env: &FunctionEnv, expr: &ExpData) {
        let g_env = env.env();
        if is_c_like_check_cmp(expr) {
            self.report(g_env, &g_env.get_node_loc(expr.node_id()), MSG);
        }
    }
}

/// Checks if an Exp is a binary comparison and looks if the operands
/// of the comparison are formatted like a C overflow check:
/// ```C
///     if (a > a + b) {
///         //...
///     }
/// ```
/// It checks for (Gt | Lt) operands, with one side matching (Add | Sub) operations.
/// Excludes patterns like `a > a - b` and `a - b < a` as they can be true, false, or abort.
fn is_c_like_check_cmp(exp: &ExpData) -> bool {
    let ExpData::Call(_, operation, operands_vec) = exp else {
        return false;
    };

    // Only look for strict comparisons.
    if !matches!(operation, Operation::Gt | Operation::Lt) || operands_vec.len() != 2 {
        return false;
    }

    let [lhs_exp, rhs_exp] = &operands_vec[..] else {
        return false;
    };

    let lhs_add_sub_operands = get_add_or_sub_operands(lhs_exp);
    let rhs_add_sub_operands = get_add_or_sub_operands(rhs_exp);

    // Check if one side of the expression contains one expression "equal" to the other side
    let lhs_matches_rhs = has_matching_operand(lhs_add_sub_operands, rhs_exp);
    let rhs_matches_lhs = has_matching_operand(rhs_add_sub_operands, lhs_exp);
    let has_matches = lhs_matches_rhs || rhs_matches_lhs;

    has_matches
        && !expression_has_different_results(
            operation,
            lhs_exp,
            rhs_exp,
            lhs_matches_rhs,
            rhs_matches_lhs,
        )
}

/// Checks if the boolean expression could result in any of (true, false, abort).
/// We do not warn in such cases.
fn expression_has_different_results(
    operation: &Operation,
    lhs_exp: &ExpData,
    rhs_exp: &ExpData,
    lhs_matches_rhs: bool,
    rhs_matches_lhs: bool,
) -> bool {
    match (
        operation,
        is_subtraction_operation(lhs_exp),
        is_subtraction_operation(rhs_exp),
    ) {
        (Operation::Gt, _, true) if rhs_matches_lhs => true,
        (Operation::Lt, true, _) if lhs_matches_rhs => true,
        _ => false,
    }
}

fn is_subtraction_operation(exp_data: &ExpData) -> bool {
    matches!(exp_data, ExpData::Call(_, Operation::Sub, _))
}

fn has_matching_operand(add_sub_operands: Option<[&Exp; 2]>, target_exp: &ExpData) -> bool {
    add_sub_operands.is_some_and(|ops| ops.iter().any(|op| expr_are_equal(op, target_exp)))
}

fn get_add_or_sub_operands(exp_data: &ExpData) -> Option<[&Exp; 2]> {
    match exp_data {
        ExpData::Call(_, op, operands)
            if matches!(op, Operation::Add | Operation::Sub) && operands.len() == 2 =>
        {
            Some([&operands[0], &operands[1]])
        },
        _ => None,
    }
}

/// This function performs a structural comparison of two expressions. It handles the following expression types:
/// - Local variables: compared by symbol equality
/// - Values: compared by value equality
/// - Temporaries: compared by temporary ID equality
/// - Function calls: compared by operation type and recursive argument comparison
///     - In this case, also checks that the operation is NOT a MoveFunction,
///       as they could be side-effecting.
fn expr_are_equal(exp_a: &ExpData, exp_b: &ExpData) -> bool {
    use ExpData::*;

    match (exp_a, exp_b) {
        (LocalVar(_, s1), LocalVar(_, s2)) => s1 == s2,
        (Value(_, v1), Value(_, v2)) => v1 == v2,
        (Temporary(_, t1), Temporary(_, t2)) => t1 == t2,
        (Call(_, op1, args1), Call(_, op2, args2)) => {
            (!matches!(op1, Operation::MoveFunction(..)))
                && op1 == op2
                && args1.len() == args2.len()
                && args1
                    .iter()
                    .zip(args2.iter())
                    .all(|(a1, a2)| expr_are_equal(a1, a2))
        },
        _ => false,
    }
}
