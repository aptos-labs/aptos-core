// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for collapsible if statements
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
    model::{FunctionEnv, NodeId},
};
use std::collections::HashSet;

#[derive(Default)]
pub struct CollapsibleIf {
    reported_nodes: HashSet<NodeId>,
}

impl ExpChecker for CollapsibleIf {
    fn get_name(&self) -> String {
        "collapsible_if".to_string()
    }

    fn visit_expr_pre(&mut self, function: &FunctionEnv, expr: &ExpData) {
        use ExpData::IfElse;

        // Look for outer if-else
        let IfElse(outer_id, _, then_branch, outer_else) = expr else {
            return;
        };

        // Skip if we've already reported this node as part of a larger collapsible if
        if self.reported_nodes.contains(outer_id) {
            return;
        }

        // Check if the outer if has no else branch (or empty else branch)
        if !matches!(outer_else.as_ref(), ExpData::Call(_, Operation::Tuple, args) if args.is_empty())
        {
            return;
        }

        // Check if the then branch contains an if statement
        let IfElse(inner_id, .., inner_else) = then_branch.as_ref() else {
            return;
        };

        // Check if the inner if also has no else branch (or empty else branch)
        if !matches!(inner_else.as_ref(), ExpData::Call(_, Operation::Tuple, args) if args.is_empty())
        {
            return;
        }

        // Mark the inner if as reported so we don't report it separately
        self.reported_nodes.insert(*inner_id);

        // Report the issue for the outer if
        let env = function.env();
        self.report(
            env,
            &env.get_node_loc(*outer_id),
            "Nested `if` statements can be collapsed into a single `if` by using logical conjunction (`&&`) of the conditions"
        );
    }
}
