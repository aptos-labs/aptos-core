// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for needless return
//! at the end of a function.
//! Each block evaluates to its last expression. If the last expression finishes
//! with `;`, it has a unit value. The main body of the function does this
//! too, so using `return x` (or `return;`) as the last statement is not needed, as
//! the `x` is enough.

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{Exp, ExpData, Operation},
    model::{FunctionEnv, GlobalEnv},
};

#[derive(Default)]
pub struct NeedlessReturn {
    is_outermost_fn: bool,
}

impl ExpChecker for NeedlessReturn {
    fn get_name(&self) -> String {
        "needless_return".to_string()
    }

    fn visit_expr_pre(&mut self, fenv: &FunctionEnv, expr: &ExpData) {
        if self.is_outermost_fn {
            return;
        } else {
            self.is_outermost_fn = true;
        }

        self.check_expr(fenv, expr);
    }
}

impl NeedlessReturn {
    fn check_expr(&mut self, fenv: &FunctionEnv, expr: &ExpData) {
        let env = fenv.env();

        match expr {
            ExpData::Sequence(_, seq) => {
                self.sequence_has_explicit_return(env, seq);
            },
            ExpData::IfElse(.., if_branch, else_branch) => {
                /*
                Cases:
                A: Function returns void, this can happen
                    if (b) {
                        // ...
                        return; <----|
                        // ()        |
                    } else {         |--- This `;` are optional, as the compiler adds a `()` after it,
                        // ...       |    and the function returns `()`.
                        return; <----|
                        // ()
                    }
                B: Function returns an element of type T (the return type, including `()`) with more code before that.
                    if (b) {
                        // ...
                        return T
                    } else {
                        // ...
                        return T
                    }
                C: Function returns an element of type T with no code before that. (Branches will not be a Sequence)
                    if (b) {
                        return T
                    } else {
                        return T
                    }
                */
                if is_non_empty_tuple(else_branch.as_ref()) {
                    // The expression is a tuple with at least one item.
                    // No `else` branch.
                    return;
                };

                // Case C branches.
                self.report_if_return(env, if_branch);
                self.report_if_return(env, else_branch);

                if let ExpData::Sequence(_, seq) = if_branch.as_ref() {
                    // Case A & B
                    self.sequence_has_explicit_return(env, seq);
                }

                if let ExpData::Sequence(_, seq) = else_branch.as_ref() {
                    // Case A & B
                    self.sequence_has_explicit_return(env, seq);
                }
            },
            _ => {
                // This will handle the case where the function looks like
                //   public fun foo(...): T {
                //      return T
                //   }
                self.report_if_return(env, expr);
            },
        }
    }

    fn report_if_return(&mut self, env: &GlobalEnv, expr: &ExpData) -> bool {
        if let ExpData::Return(nid_r, _) = expr {
            self.report(
                env,
                &env.get_node_loc(*nid_r),
                "The `return` keyword is unnecessary here and can be removed.",
            );
            true
        } else {
            false
        }
    }

    fn sequence_has_explicit_return(&mut self, env: &GlobalEnv, seq: &[Exp]) {
        let Some(last_val) = seq.last() else { return };

        if self.report_if_return(env, last_val) {
            return;
        }
        self.semicolon_case(env, seq, last_val);
    }

    // If the function returns `()`, check the previous one.
    // The semicolon "adds" an empty tuple, so we need to check if the
    // previous expression has a `return`
    // Case:                         |
    // ```                           |
    //   public fun foo(...) {       |
    //      return;                  |
    //      // ()     <--------------|
    //   }
    //
    // ```
    fn semicolon_case(&mut self, env: &GlobalEnv, seq: &[Exp], expr: &ExpData) {
        if seq.len() < 2 {
            return;
        }
        if !is_non_empty_tuple(expr) {
            // The expression is either an empty tuple or not a tuple at all.
            if let Some(snd_to_last) = seq.get(seq.len() - 2) {
                self.report_if_return(env, snd_to_last);
            }
        }
    }
}

fn is_non_empty_tuple(expr: &ExpData) -> bool {
    if let ExpData::Call(_, Operation::Tuple, args) = expr {
        !args.is_empty()
    } else {
        false
    }
}
