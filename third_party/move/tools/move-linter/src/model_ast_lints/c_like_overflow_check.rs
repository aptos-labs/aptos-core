// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{Exp, ExpData, Operation},
    model::{FunctionEnv, GlobalEnv},
};

const MSG: &str =
    "C-like overflow checks are not necessary, as overflows automatically abort execution in Move";

#[derive(Default)]

pub struct CLikeOverflowCheck;

impl ExpChecker for CLikeOverflowCheck {
    fn get_name(&self) -> String {
        "c_like_overflow_checks".to_string()
    }

    fn visit_expr_pre(&mut self, env: &FunctionEnv, expr: &ExpData) {
        let env = env.env();
        if let ExpData::IfElse(_, cond, _, _) = expr.clone() {
            if is_c_like_check_cmp(env, &cond) {
                self.report(env, &env.get_node_loc(cond.node_id()), MSG);
            }
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
/// It checks for (Gt | Lt | Le | Ge) operands only, as those are the
/// most commonly used for this type of check.
fn is_c_like_check_cmp(env: &GlobalEnv, exp: &ExpData) -> bool {
    let ExpData::Call(_, operation, operands_vec) = exp else {
        return false;
    };

    if !op_is_cmp(operation) || operands_vec.len() != 2 {
        return false;
    }

    let lhs_exp = &operands_vec[0];
    let rhs_exp = &operands_vec[1];

    let lhs_add_sub_operands = get_add_or_sub_operands(&lhs_exp);
    let rhs_add_sub_operands = get_add_or_sub_operands(&rhs_exp);

    // Checks if one side of the expression (the one with the format `a cmp b`) contains
    // one expression "equal" to the other side (the one with the format `a` or `b`)
    // It only checks for `ExpData::{LocalVar, Value, Temporary, Call}`.
    match (lhs_add_sub_operands, rhs_add_sub_operands) {
        (Some(l_ops), None) => l_ops
            .iter()
            .any(|operand_in_lhs| expr_are_equal(env, *operand_in_lhs, rhs_exp)),
        (None, Some(r_ops)) => r_ops
            .iter()
            .any(|operand_in_rhs| expr_are_equal(env, *operand_in_rhs, lhs_exp)),
        _ => false,
    }
}

fn op_is_cmp(op: &Operation) -> bool {
    matches!(
        op,
        Operation::Gt | Operation::Lt | Operation::Le | Operation::Ge
    )
}

fn op_is_add_or_sub(op: &Operation) -> bool {
    matches!(op, Operation::Add | Operation::Sub)
}

fn get_add_or_sub_operands(exp_data: &ExpData) -> Option<[&Exp; 2]> {
    if let ExpData::Call(_, op, operands_vec) = exp_data {
        if op_is_add_or_sub(op) && operands_vec.len() == 2 {
            return Some([&operands_vec[0], &operands_vec[1]]);
        }
    }
    None
}

fn expr_are_equal(env: &GlobalEnv, exp_a: &ExpData, exp_b: &ExpData) -> bool {
    use ExpData::*;
    match (exp_a, exp_b) {
        (LocalVar(_, s1), LocalVar(_, s2)) => s1 == s2,
        (Value(_, v1), Value(_, v2)) => v1 == v2,
        (Temporary(_, t1), Temporary(_, t2)) => t1 == t2,
        (Call(_, op1, args1), Call(_, op2, args2)) => {
            op1 == op2
                && args1.len() == args2.len()
                && args1
                    .iter()
                    .zip(args2.iter())
                    .all(|(a1, a2)| expr_are_equal(env, a1, a2))
        },
        _ => false,
    }
}
