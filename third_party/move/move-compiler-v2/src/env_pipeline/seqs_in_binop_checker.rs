// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Pre-requisites: this checker should be run before inlining in its current
//! incarnation.
//
//! This module implements a checker that looks for non-trivial sequences
//! within binary operations. The v1 compiler's evaluation order semantics
//! in the presence of sequences within binary operations are not easily
//! understood or explainable (see examples below).
//!
//! Therefore, in compiler v2 (and above), if the language version is less than
//! 2.0, we will emit an error in such cases. We expect such uses to be rare,
//! and the user can easily rewrite the code to get explicit evaluation order
//! that they want.
//!
//! In language version 2.0 and above, we will allow sequences within binary
//! operations, but the evaluation order will be consistently left-to-right,
//! following the evaluation order semantics used in normal function calls.
//!
//! Consider the following examples to see some samples of evaluation order used
//! by compiler v1 in the presence of sequences within the context of binary
//! operations. They are meant to showcase how concisely describing the v1 ordering
//! is hard (as opposed to, a left-to-right evaluation ordering everywhere).
//!
//! We number the sub-expressions in their order of their evaluation.
//! Some (sub-)expressions are left un-numbered if they are irrelevant to the
//! understanding of the evaluation order.
//!
//! case 1: `add` is a user-defined function.
//! ```move
//! let x = 1;
//! add({x = x - 1; x + 8}, {x = x + 3; x - 3}) + {x = x * 2; x * 2}
//!      ^^^^^^^^^  ^^^^^    ^^^^^^^^^  ^^^^^      ^^^^^^^^^  ^^^^^
//!         |        |         |          |            |        |
//!         |        |         |          |            |        |
//!         1        |         |          |            |        |
//!                  2         |          |            |        |
//!                            3          |            |        |
//!                                       |            4        |
//!                                       5                     |
//!                                                             6
//! ```
//!
//! case 2:
//! ```move
//! fun aborter(x: u64): u64 {
//!     abort x
//! }
//!
//! public fun test(): u64 {
//!     let x = 1;
//!     aborter(x) + {x = x + 1; aborter(x + 100); x} + x
//!     ^^^^^^^^^^    ^^^^^^^^^  ^^^^^^^^^^^^^^^^
//!        |              |              |
//!        |              1              |
//!        |                             2
//!     never evaluated
//! }
//! ```
//!
//! case 3:
//! ```move
//! (abort 0) + {(abort 14); 0} + 0
//!  ^^^^^^^      ^^^^^^^^
//!     |              |
//!     1              |
//!                 never evaluated
//! ```
//!
//! case 4:
//! ```move
//! {250u8 + 50u8} + {abort 55; 5u8}
//!  ^^^^^^^^^^^^     ^^^^^^^^
//!      |               |
//!      |               1
//!   never evaluated
//! ```
//!
//! case 5:
//! ```move
//! let x = 1;
//! x + {x = x + 1; x} + {x = x + 1; x}
//! ^    ^^^^^^^^^  ^     ^^^^^^^^^  ^
//! |       |       |        |       |
//! |       1       |        |       |
//! |               |        2       |
//! 3               3                3
//! ```

use codespan_reporting::diagnostic::Severity;
use move_model::{
    ast::{Exp, ExpData, Operation},
    model::{FunctionEnv, GlobalEnv},
};
use std::collections::BTreeMap;

/// Perform the check detailed in the module documentation at the top of this file.
/// This check is performed on all non-native functions in all target modules.
/// Violations of the check are reported as errors on the `env`.
pub fn checker(env: &mut GlobalEnv) {
    for module in env.get_modules() {
        if module.is_target() {
            for function in module.get_functions() {
                if function.is_native() {
                    continue;
                }
                check_function(&function);
            }
        }
    }
}

/// Perform the check detailed in the module documentation on the code in `function`.
/// Violations of the check are reported as errors on the `GlobalEnv` of the `function`.
fn check_function(function: &FunctionEnv) {
    if let Some(def) = function.get_def() {
        // Maintain some state as we traverse down and up the AST.
        // Maintain a stack of pairs (binary operation's node id, binary operation).
        let mut binop_stack = Vec::new();
        // Maintain a triple (binary operator id, binary operator, sequence id), where
        // a non-trivial sequence is found within the context of the binary operator.
        // We use this later to report errors.
        let mut errors = Vec::new();
        // Maintain a map from binary operator id to sequence id, where a non-trivial
        // sequence is found within the context of the binary operator.
        let mut sequences = BTreeMap::new();
        let mut visitor = |post: bool, e: &ExpData| {
            use ExpData::*;
            match e {
                Call(id, op, exps) if op.is_binop() => {
                    if !post {
                        binop_stack.push((*id, op.clone()));
                    } else {
                        let (binop_id, binop) = binop_stack.pop().expect("unbalanced");
                        if let Some(seq_id) = sequences.remove(&binop_id) {
                            // There was a sequence within the context of this binary operation.
                            // We now check if variables are shared between the two expressions, or
                            // if there are control flow redirections.
                            let param_symbols = function
                                .get_parameters()
                                .into_iter()
                                .map(|p| p.0)
                                .collect::<Vec<_>>();
                            let lhs = &exps[0];
                            let rhs = &exps[1];
                            let lhs_vars = lhs.free_vars_and_used_params(&param_symbols);
                            let rhs_vars = rhs.free_vars_and_used_params(&param_symbols);
                            let overlap = lhs_vars.intersection(&rhs_vars).next().is_some();
                            if overlap
                                || contains_control_flow_redirections(lhs)
                                || contains_control_flow_redirections(rhs)
                            {
                                // Note: if needed, we can make this check even more precise by tracking
                                // read and written variables, and checking only for read-write and
                                // write-write conflicts.
                                errors.push((binop_id, binop.clone(), seq_id));
                            }
                        }
                    }
                },
                Sequence(id, seq)
                    if seq.len() > 1 && !seq.iter().all(|exp| exp.is_ok_to_remove_from_code()) =>
                {
                    if let Some((binop_id, _)) = binop_stack.last() {
                        // There is a non-trivial sequence within the context of a binary operation.
                        // Non-trivial currently means that the sequence has more than one expression,
                        // and not all those expressions are potentially side-effect free.
                        // Note: if needed, we can implement a more precise check instead of reusing
                        // `is_ok_to_remove_from_code` to track side-effect-free expressions.
                        sequences.entry(*binop_id).or_insert(*id);
                    }
                },
                _ => {},
            }
            true // continue traversal
        };
        def.visit_pre_post(&mut visitor);
        let env = function.module_env.env;
        for (binop_id, binop, seq_id) in errors {
            let binop_loc = env.get_node_loc(binop_id);
            let seq_loc = env.get_node_loc(seq_id);
            let labels = vec![(seq_loc, "non-empty sequence".to_owned())];
            let binop_as_str = binop.to_string_if_binop().expect("binop");
            let notes = vec![
                "To compile this code, either:".to_owned(),
                "1. upgrade to language version 2.0 or later (which uses strict left-to-right evaluation order),".to_owned(),
                "2. rewrite the code to remove sequences from directly within binary operations,"
                    .to_owned(),
                "   e.g., save intermediate results providing explicit order.".to_owned(),
                "In either of these cases, please ensure to check the code does what you expect it to, because of changed semantics.".to_owned(),
            ];
            env.diag_with_primary_notes_and_labels(
                Severity::Error,
                &binop_loc,
                &format!(
                    "A sequence within an operand of binary operation `{}` can obscure program logic and is not allowed by this compiler.",
                    binop_as_str
                ),
                &format!("binary operation `{}`", binop_as_str),
                notes,
                labels,
            );
        }
    }
}

// Does the `exp` contain at least one of the control flow redirections
// (`abort`, `return`, `break`, `continue`)?
fn contains_control_flow_redirections(exp: &Exp) -> bool {
    let mut result = false;
    let mut visitor = |e: &ExpData| {
        use ExpData::*;
        if matches!(e, LoopCont(..) | Return(..) | Call(_, Operation::Abort, _)) {
            result = true;
            false // stop traversal early
        } else {
            true // continue traversal
        }
    };
    exp.visit_pre_order(&mut visitor);
    result
}
