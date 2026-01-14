// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for empty `else` branches
//!   public fun empty_if(x: u64): u64 {
//!    if (x > 35) {  // <----
//!    } else {
//!        x = x + 1;
//!    };
//!    x
//!  }

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{ExpData, Operation},
    model::FunctionEnv,
};

#[derive(Default)]
pub struct EmptyIf;

impl ExpChecker for EmptyIf {
    fn get_name(&self) -> String {
        "empty_if".to_string()
    }

    fn visit_expr_pre(&mut self, function: &FunctionEnv, expr: &ExpData) {
        let ExpData::IfElse(full_nid, .., if_branch, else_branch) = expr else {
            return;
        };

        let ExpData::Call(_, Operation::Tuple, args) = if_branch.as_ref() else {
            return;
        };

        if !args.is_empty() {
            return;
        }

        if let ExpData::Call(_, Operation::Abort(_), _) = else_branch.as_ref() {
            return;
        }

        let genv = function.env();

        let message = match else_branch.as_ref() {
            ExpData::Call(.., Operation::Tuple, eargs) if eargs.is_empty() => {
                "Empty `if` branch. Consider simplifying by removing or rewriting."
            },
            ExpData::LoopCont(_, _, _) => {
                return;
            },
            _ => "Empty `if` branch. Consider simplifying this `if-else`.",
        };

        self.report(genv, &genv.get_node_loc(*full_nid), message);
    }
}
