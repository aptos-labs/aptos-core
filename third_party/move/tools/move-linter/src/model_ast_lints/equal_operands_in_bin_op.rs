// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Checks for binary operations where both operands are the same,
/// which is likely a mistake.
/// This lint catches usage of same operands in:
///  `%`, `^`, `<`, `>`, `|`, `&`, `/`, `!=`, `>=`, and `<=`
/// This lint does not catch cases where the operands are vector access.
/// The usage of same operands in `&&` and `||` is warned in
/// `simpler_boolean_expression` instead of this one.
use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{ExpData, Operation},
    model::FunctionEnv,
};

#[derive(Default)]
pub struct EqualOperandsInBinOp;

impl ExpChecker for EqualOperandsInBinOp {
    fn get_name(&self) -> String {
        "equal_operands_in_bin_op".to_string()
    }

    fn visit_expr_pre(&mut self, env: &FunctionEnv, expr: &ExpData) {
        let ExpData::Call(nid, op, params) = expr else {
            return;
        };

        if params.len() != 2 || !expressions_are_equal(&params[0], &params[1]) {
            return;
        }

        let Some(message) = binary_operation_result_message(op) else {
            return;
        };

        let env = env.env();
        self.report(env, &env.get_node_loc(*nid), &message);
    }
}

/// Performs a structural comparison of two expressions.
///
/// Handles the following expression types:
/// - Local variables: compared by symbol equality
/// - Values: compared by value equality
/// - Temporaries: compared by temporary ID equality
/// - Function calls: compared by operation type and recursive argument comparison
///   (excludes MoveFunction operations as they could be side-effecting)
fn expressions_are_equal(expr1: &ExpData, expr2: &ExpData) -> bool {
    use ExpData::*;
    match (expr1, expr2) {
        (LocalVar(_, s1), LocalVar(_, s2)) => s1 == s2,
        (Value(_, v1), Value(_, v2)) => v1 == v2,
        (Temporary(_, t1), Temporary(_, t2)) => t1 == t2,
        (Call(_, op1, args1), Call(_, op2, args2)) => {
            !matches!(op1, Operation::MoveFunction(..))
                && op1 == op2
                && args1.len() == args2.len()
                && args1
                    .iter()
                    .zip(args2.iter())
                    .all(|(a1, a2)| expressions_are_equal(a1, a2))
        },
        _ => false,
    }
}

/// Returns a diagnostic message for binary operations with equal operands.
fn binary_operation_result_message(op: &Operation) -> Option<String> {
    use Operation::*;

    let result = match op {
        Mod | Xor => "`0`",
        Le | Ge | Eq => "`true`",
        BitOr | BitAnd => "the same value: `x | x` and `x & x` can be simplified to `x`",
        Div => "`1`",
        Neq | Lt | Gt => "`false`",
        _ => return None,
    };

    Some(format!("This operation always evaluates to {result}."))
}
