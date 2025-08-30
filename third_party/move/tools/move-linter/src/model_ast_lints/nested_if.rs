// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for nested if statements
//! that can be simplified using the `&&` operator.
//!
//! For example:
//! ```move
//! if (a) {
//!     if (b) {
//!         // some code
//!     }
//! }
//! ```
//! can be simplified to:
//! ```move
//! if (a && b) {
//!     // some code
//! }
//! ```

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{ExpData, Operation},
    model::FunctionEnv,
};

#[derive(Default)]
pub struct NestedIf;

impl ExpChecker for NestedIf {
    fn get_name(&self) -> String {
        "nested_if".to_string()
    }

    fn visit_expr_pre(&mut self, function: &FunctionEnv, expr: &ExpData) {
        use ExpData::IfElse;

        // Look for outer if-else
        let IfElse(outer_id, _, then_branch, outer_else) = expr else {
            return;
        };

        // Check if the outer if has no else branch (or empty else branch)
        if !matches!(outer_else.as_ref(), ExpData::Call(_, Operation::Tuple, args) if args.is_empty())
        {
            return;
        }

        // Check if the then branch contains an if statement
        let IfElse(.., inner_else) = then_branch.as_ref() else {
            return;
        };

        // Check if the inner if also has no else branch (or empty else branch)
        if !matches!(inner_else.as_ref(), ExpData::Call(_, Operation::Tuple, args) if args.is_empty())
        {
            return;
        }

        // Report the issue
        let env = function.env();
        self.report(
            env,
            &env.get_node_loc(*outer_id),
            "Nested `if` statements can be collapsed into a single `if` by using logical conjunction (`&&`) of the conditions"
        );
    }
}
