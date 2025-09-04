// Copyright (c) Velor Foundation
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
