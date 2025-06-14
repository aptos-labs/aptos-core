// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module holds utility functions for the Move linter.

use move_model::ast::{ExpData, Operation};

/// Returns `true` if two expressions represent the same simple access pattern.
/// This compares nested `Select`, `Borrow`, and local variable references for structural equality.
/// `Deref` calls can occur anywhere without affecting the result.
/// Patterns that use global storage or non-builtin function (including vector operations)
/// are not considered simple access patterns for the purpose of this function and return `false`.
pub(crate) fn is_simple_access_equal(expr1: &ExpData, expr2: &ExpData) -> bool {
    match (expr1, expr2) {
        (ExpData::Call(_, Operation::Deref, args), expr)
        | (expr, ExpData::Call(_, Operation::Deref, args)) => {
            is_simple_access_equal(&args[0], expr)
        },
        (ExpData::Call(_, op1, args1), ExpData::Call(_, op2, args2)) => {
            op1 == op2
                && matches!(
                    op1,
                    Operation::Select(_, _, _)
                        | Operation::Borrow(_)
                        | Operation::SelectVariants(_, _, _)
                )
                && args1
                    .iter()
                    .zip(args2.iter())
                    .all(|(a1, a2)| is_simple_access_equal(a1, a2))
        },
        (ExpData::LocalVar(_, s1), ExpData::LocalVar(_, s2)) => s1 == s2,
        _ => false,
    }
}

/// Checks if two expressions are structurally equal.
///
/// This function performs a structural comparison of two expressions. It handles the following expression types:
/// - Local variables: compared by symbol equality
/// - Values: compared by value equality
/// - Temporaries: compared by temporary ID equality
/// - Function calls: compared by operation type and recursive argument comparison
///
/// # Arguments
/// * `expr1` - The first expression to compare
/// * `expr2` - The second expression to compare
///
/// # Returns
/// * `true` if the expressions are structurally identical, `false` otherwise
///
/// # Note
/// This function only handles a subset of expression types. For unsupported
/// expression types, it will return `false` even if they might be equivalent.
pub(crate) fn is_expression_equal(expr1: &ExpData, expr2: &ExpData) -> bool {
    use ExpData::*;

    match (expr1, expr2) {
        (LocalVar(_, s1), LocalVar(_, s2)) => s1 == s2,
        (Value(_, v1), Value(_, v2)) => v1 == v2,
        (Temporary(_, t1), Temporary(_, t2)) => t1 == t2,
        (Call(_, op1, args1), Call(_, op2, args2)) => {
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
