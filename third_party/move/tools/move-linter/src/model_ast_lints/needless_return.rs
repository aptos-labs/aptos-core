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

        let env = fenv.env();

        self.check_expr(env, expr, true);
    }
}

impl NeedlessReturn {
    fn check_expr(&mut self, env: &GlobalEnv, expr: &ExpData, main_function: bool) {
        match expr {
            ExpData::Sequence(_, seq) => {
                if main_function {
                    seq.iter().for_each(|x| self.check_expr(env, x, false));
                } else {
                    self.sequence_ends_in_explicit_return(env, seq);
                }
            },

            ExpData::IfElse(.., if_branch, else_branch) => {
                let span_end = { |x: &Exp| env.get_node_loc(x.node_id()).span().end() };

                if span_end(if_branch) == span_end(else_branch) {
                    // In this case, there's no `else` branch.
                    //   fun test(b: bool) {
                    //      if (b) {
                    //          return
                    //      };
                    //  }
                    //
                    return;
                }

                // This case is for empty `else` branches with a semicolon in the return
                //   fun test(b: bool) {
                //      if (b) {
                //          return;
                //      };
                //  }
                // As it gets compiled differently
                is_empty_tuple(else_branch.as_ref());

                self.report_if_return(env, if_branch);
                if let ExpData::Sequence(_, seq) = if_branch.as_ref() {
                    self.sequence_ends_in_explicit_return(env, seq);
                }

                self.report_if_return(env, else_branch);
                if let ExpData::Sequence(_, seq) = else_branch.as_ref() {
                    self.sequence_ends_in_explicit_return(env, seq);
                }
            },
            _ => {
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

    fn sequence_ends_in_explicit_return(&mut self, env: &GlobalEnv, seq: &Vec<Exp>) {
        let Some(last_val) = seq.last() else { return };

        // Case:
        // ```
        //   public fun foo(...): ... {
        //      return ...
        //   }
        //
        // ```
        if self.report_if_return(env, last_val) {
            return;
        }
        self.semicolon_case(env, seq, &last_val);
    }

    // The semicolon "adds" an empty tuple after all statements.
    // If the function does not return anything, this can happen
    // Case:                         |
    // ```                           |
    //   public fun foo(...) {       |
    //      return;                  |
    //      // ()     <--------------|
    //   }
    //
    // ```
    fn semicolon_case(&mut self, env: &GlobalEnv, seq: &Vec<Exp>, expr: &ExpData) {
        if seq.len() < 2 {
            return;
        }
        if !is_empty_tuple(expr) {
            if let Some(snd_to_last) = seq.get(seq.len() - 2) {
                self.report_if_return(env, snd_to_last);
            }
        }
    }
}

fn is_empty_tuple(expr: &ExpData) -> bool {
    matches!(expr, ExpData::Call(_, Operation::Tuple, args) if !args.is_empty())
}
