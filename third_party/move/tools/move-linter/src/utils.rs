// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module holds utility functions for the Move linter.
use legacy_move_compiler::parser::syntax::FOR_LOOP_UPDATE_ITER_FLAG;
use move_model::{
    ast::{
        ExpData,
        ExpData::{IfElse, LocalVar, Loop, Sequence},
        Operation,
    },
    model::FunctionEnv,
};

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

/// Detects if the given Loop expression matches the pattern of a while loop.
/// ```
/// loop {
///     if (condition) {
///         // loop body
///     } else {
///         break;
///     }
/// }
/// ```
///
pub(crate) fn detect_while_loop(expr: &ExpData) -> bool {
    let Loop(_, loop_body) = expr else {
        return false;
    };
    match loop_body.as_ref() {
        ExpData::IfElse(_, _, _, else_expr) => {
            matches!(else_expr.as_ref(), ExpData::LoopCont(_, nest, is_continue) if *nest == 0 && !*is_continue)
        },
        _ => false,
    }
}

/// Detects if the Loop expression matches the pattern of an expanded for loop.
///
/// The expanded for loop has this structure:
/// ```
/// loop {
///     if (true) {
///         if (flag) {
///             increment;
///         } else {
///             flag = true;
///         }
///         if (i < limit) {
///             body;
///         } else {
///             break;
///         }
///     } else {
///         break;
///     }
/// }
/// ```
pub(crate) fn detect_for_loop(expr: &ExpData, function: &FunctionEnv) -> bool {
    let Loop(_, body) = expr else { return false };

    let IfElse(_, _, then, _) = body.as_ref() else {
        return false;
    };

    let Sequence(_, stmts) = then.as_ref() else {
        return false;
    };
    let Some(stmt) = stmts.first() else {
        return false;
    };
    let IfElse(_, cond, _, _) = stmt.as_ref() else {
        return false;
    };
    let LocalVar(_, name) = cond.as_ref() else {
        return false;
    };
    name.display(function.symbol_pool()).to_string() == FOR_LOOP_UPDATE_ITER_FLAG
}
