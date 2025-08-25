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
        let ExpData::IfElse(.., if_branch, _) = expr else {
            return;
        };

        let ExpData::Call(nid, Operation::Tuple, args) = &**if_branch else {
            return;
        };

        if !args.is_empty() {
            return;
        }
        let genv = function.env();

        self.report(
            genv,
            &genv.get_node_loc(*nid),
            "Empty `if` branch. Consider simplifying this `if-else`.",
        );
    }
}
