// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module holds utility functions for the Move linter.
use move_model::{
    ast::{ExpData, ExpData::Loop, Operation, Pattern, Value},
    symbol::Symbol,
};
use num::BigInt;

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
/// ```ignore
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

/// Detects whether `expr` is a lowered `for` loop *with a `continue`*, which is
/// wrapped in an inner single-pass loop:
/// ```ignore
/// loop {                          // <- expr (the outer loop)
///     if (i < $ub) {
///         loop { body; break };   // inner, single-pass loop
///         i = i + 1;
///     } else break
/// }
/// ```
/// This is the model shape produced by the first-class `for` lowering in
/// `move-model`'s `ExpTranslator::translate_for_loop` (the iterator is advanced
/// at the bottom of the loop, and a `continue` becomes a `break` of the inner
/// loop so it falls through to the increment). It is not the pre-lowering
/// desugaring: the model AST has no `for` node, so this matches on structure.
///
/// A `for` loop *without* a `continue` has no inner loop and is shaped exactly
/// like a `while` loop (see [`detect_while_loop`]). Recognizing this shape lets
/// consumers (e.g. the cyclomatic complexity lint) treat such a `for` loop as a
/// single loop, rather than counting the inner loop and breaks introduced by
/// lowering as separate decisions.
pub(crate) fn detect_for_loop(expr: &ExpData) -> bool {
    use ExpData::{IfElse, LoopCont, Sequence};
    let Loop(_, body) = expr else {
        return false;
    };
    let IfElse(_, cond, then, else_) = body.as_ref() else {
        return false;
    };
    // The else-branch breaks the outer loop.
    if !matches!(else_.as_ref(), LoopCont(_, 0, false)) {
        return false;
    }
    // The then-branch is *exactly* `{ loop { body; break }; i = i + 1 }`: an
    // inner single-pass loop followed by the iterator increment, and nothing
    // else. Matching only the leading inner loop would misclassify an ordinary
    // `while`/`for` loop whose body merely happens to begin with a
    // `loop { ...; break }`.
    let Sequence(_, stmts) = then.as_ref() else {
        return false;
    };
    let [inner, incr] = stmts.as_slice() else {
        return false;
    };
    // The inner loop runs once and ends by breaking itself.
    let Loop(_, inner_body) = inner.as_ref() else {
        return false;
    };
    let Sequence(_, inner_stmts) = inner_body.as_ref() else {
        return false;
    };
    if !matches!(
        inner_stmts.last().map(|e| e.as_ref()),
        Some(LoopCont(_, 0, false))
    ) {
        return false;
    }
    // The increment must advance the very variable the loop condition tests,
    // i.e. `i = i + 1` guarded by `i < $ub` — the exact shape produced when
    // lowering a `for` loop that contains a `continue`.
    match (for_iter_increment_var(incr), for_loop_cond_var(cond)) {
        (Some(incr_var), Some(cond_var)) => incr_var == cond_var,
        _ => false,
    }
}

/// Returns the variable advanced by a lowered `for`-loop iterator increment of
/// the form `i = i + 1`, or `None` if `expr` is not such an increment.
fn for_iter_increment_var(expr: &ExpData) -> Option<Symbol> {
    use ExpData::{Assign, Call, LocalVar, Value as ExpValue};
    let Assign(_, Pattern::Var(_, target), rhs) = expr else {
        return None;
    };
    let Call(_, Operation::Add, args) = rhs.as_ref() else {
        return None;
    };
    let [lhs, one] = args.as_slice() else {
        return None;
    };
    let LocalVar(_, var) = lhs.as_ref() else {
        return None;
    };
    if var != target {
        return None;
    }
    matches!(one.as_ref(), ExpValue(_, Value::Number(n)) if *n == BigInt::from(1))
        .then_some(*target)
}

/// Returns the iterator variable compared in a lowered `for`-loop condition
/// `i < $ub`. The comparison may be preceded by a loop-invariant spec block
/// (`{ spec; i < $ub }`), in which case it is the sequence's final element.
fn for_loop_cond_var(cond: &ExpData) -> Option<Symbol> {
    use ExpData::{Call, LocalVar, Sequence};
    let cmp = match cond {
        Sequence(_, exps) => exps.last()?.as_ref(),
        other => other,
    };
    let Call(_, Operation::Lt, args) = cmp else {
        return None;
    };
    match args.first()?.as_ref() {
        LocalVar(_, sym) => Some(*sym),
        _ => None,
    }
}
