// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for needless return
//! at the end of a function.
//! Each block returns its last expression. If the last expression finishes
//! with `;`, a tuple is returned. The main body of the function does this
//! too, so using `return x` (or `return;`) as the last statement is not needed, as
//! the `x` is enough.

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{ExpData, Operation},
    model::{FunctionEnv, GlobalEnv},
    symbol::Symbol,
};
use std::collections::HashSet;

#[derive(Default)]
pub struct NeedlessReturn {
    fn_seen: HashSet<Symbol>,
}

impl ExpChecker for NeedlessReturn {
    fn get_name(&self) -> String {
        "needless_return".to_string()
    }

    fn visit_expr_pre(&mut self, fenv: &FunctionEnv, expr: &ExpData) {
        if !self.fn_seen.insert(fenv.get_name()) {
            return;
        }

        let env = fenv.env();

        match expr {
            ExpData::Sequence(_, seq) => {
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

                // The semicolon "adds" an empty tuple after all statements.
                // If the function does not return anyhing, this can happen
                // Case:                         |
                // ```                           |
                //   public fun foo(...) {       |
                //      return;                  |
                //      // ()     <--------------|
                //   }
                //
                // ```
                if let ExpData::Call(_, Operation::Tuple, ..) = &**last_val {
                    if let Some(snd_to_last) = seq.get(seq.len() - 2) {
                        self.report_if_return(env, snd_to_last);
                    }
                }
            },
            _ => {
                self.report_if_return(env, expr);
            },
        }
    }
}

impl NeedlessReturn {
    fn report_if_return(&mut self, env: &GlobalEnv, expr: &ExpData) -> bool {
        if let ExpData::Return(nid_r, _) = expr {
            self.report(
                env,
                &env.get_node_loc(*nid_r),
                "The return keyword can be removed, as the last value is automatically returned.",
            );
            true
        } else {
            false
        }
    }
}
