// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Cyclomatic complexity measures the number of linearly independent execution paths
//! through a function.  A high value generally correlates with code that is
//! harder to test and maintain.
//!
//! This linter performs an approximation while traversing the Move
//! expression tree:
//!   1. The complexity score starts at **1**.
//!   2. The score is incremented for each control-flow decision point found:
//!         * +1 for each `if`
//!         * +1 for each `else if`
//!         * +1 for each `loop`, `while`, or `for`
//!         * +1 for each `break` or `continue`
//!         * +1 for each `return` statement that is not the final expression in the function
//!         * +n where n = (number of match arms - 1)
//!
//! When the accumulated score exceeds `DEFAULT_THRESHOLD` (currently **10**),
//! the linter emits a diagnostic suggesting that the function be simplified or
//! decomposed.
//!
//! NOTE: The threshold is intentionally conservative.

use crate::utils::{detect_for_loop, detect_while_loop};
use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::ExpData,
    model::{FunctionEnv, Loc, NodeId},
};

const DEFAULT_THRESHOLD: i32 = 10;

pub struct CyclomaticComplexity {
    complexity: i32,
    reported: bool,
    root_node: Option<NodeId>,
    final_return_node: Option<NodeId>,
}

impl CyclomaticComplexity {
    fn bump(&mut self, delta: i32) {
        self.complexity = self.complexity.saturating_add(delta);
    }

    /// Returns the NodeId of the final return statement in the function, if any.
    fn get_final_return(function: &FunctionEnv) -> Option<NodeId> {
        if let Some(def) = function.get_def() {
            return Self::get_final_return_from_exp(def);
        }
        None
    }

    /// Helper method to recursively find the final return statement in an expression.
    fn get_final_return_from_exp(expr: &move_model::ast::Exp) -> Option<NodeId> {
        use move_model::ast::ExpData::*;
        match expr.as_ref() {
            Return(id, _) => Some(*id),
            Sequence(_, seq) => seq.last().and_then(Self::get_final_return_from_exp),
            Block(_, _, _, body) => Self::get_final_return_from_exp(body),
            _ => None,
        }
    }

    fn maybe_report(&mut self, function: &FunctionEnv) {
        if self.reported || self.complexity <= DEFAULT_THRESHOLD {
            return;
        }
        let env = function.env();
        let loc: Loc = function.get_loc();
        self.reported = true;
        self.report(
            env,
            &loc,
            &format!(
                "Function `{}` has cyclomatic complexity {}, which exceeds the allowed threshold of {}",
                function.get_full_name_str(),
                self.complexity,
                DEFAULT_THRESHOLD,
            ),
        );
    }
}

impl Default for CyclomaticComplexity {
    fn default() -> Self {
        Self {
            complexity: 1,
            reported: false,
            root_node: None,
            final_return_node: None,
        }
    }
}

impl ExpChecker for CyclomaticComplexity {
    fn get_name(&self) -> String {
        "cyclomatic_complexity".to_string()
    }

    fn visit_expr_pre(&mut self, function: &FunctionEnv, expr: &ExpData) {
        if self.root_node.is_none() {
            self.root_node = function.get_def().map(|def| def.node_id());
            self.final_return_node = Self::get_final_return(function);
        }

        use ExpData::*;

        match expr {
            // loop, while, for
            Loop(_, _) => {
                let delta = if detect_for_loop(expr, function) {
                    // For loop expansion generates: Loop(+1) + IfElse(+1) + IfElse(+1) + LoopCont(+1) + LoopCont(+1) = +5 extra
                    // But we want for to count as +1 total, so we subtract 4 here
                    -4
                } else if detect_while_loop(expr) {
                    // While loop expansion generates: Loop(+1) + IfElse(+1) + LoopCont(+1) = +3 extra
                    // But we want while to count as +1 total, so we subtract 1 here
                    -1
                } else {
                    1
                };
                self.bump(delta);
            },
            // if and else if (+1)
            IfElse(..) => self.bump(1),
            // break and continue (+1)
            LoopCont(..) => self.bump(1),

            // return, if is not the last statement (+1)
            Return(_, _) => {
                if self.final_return_node != Some(expr.node_id()) {
                    self.bump(1);
                }
            },
            // match (+n-1)
            Match(_, _, arms) if !arms.is_empty() => self.bump(arms.len() as i32 - 1),

            _ => {},
        }
    }

    fn visit_expr_post(&mut self, function: &FunctionEnv, expr: &ExpData) {
        if let Some(root) = self.root_node {
            if expr.node_id() == root {
                self.maybe_report(function);
            }
        }
    }
}
