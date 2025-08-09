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

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{ExpData, Operation, Value},
    model::{FunctionEnv, Loc, NodeId},
};

const DEFAULT_THRESHOLD: i32 = 10;

type FunKey = (move_model::model::ModuleId, move_model::model::FunId);

#[derive(Default)]
pub struct CyclomaticComplexity {
    complexity: i32,
    reported: bool,
    root_node: Option<NodeId>,
    current_fun: Option<FunKey>,
}

impl CyclomaticComplexity {
    fn bump(&mut self, delta: i32) {
        self.complexity = self.complexity.saturating_add(delta);
    }

    /// Returns true if the Loop expression matches the pattern of an expanded for loop.
    fn is_expanded_for_loop(&self, loop_body: &ExpData) -> bool {
        if let ExpData::IfElse(_, condition, then_branch, else_branch) = loop_body {
            let condition_is_true =
                matches!(condition.as_ref(), ExpData::Value(_, Value::Bool(true)));

            let else_is_break = matches!(else_branch.as_ref(), ExpData::LoopCont(_, 0, false));

            let then_has_for_pattern =
                if let ExpData::Sequence(_, statements) = then_branch.as_ref() {
                    self.check_for_pattern_in_sequence(statements)
                } else {
                    false
                };

            condition_is_true && else_is_break && then_has_for_pattern
        } else {
            false
        }
    }

    /// Returns true if the sequence contains the for loop pattern.
    fn check_for_pattern_in_sequence(&self, statements: &[move_model::ast::Exp]) -> bool {
        if statements.len() >= 2 {
            let first_is_flag_management =
                if let ExpData::IfElse(_, condition, then_branch, else_branch) =
                    statements[0].as_ref()
                {
                    let condition_is_flag = matches!(condition.as_ref(), ExpData::LocalVar(_, _));

                    let then_is_increment =
                        matches!(then_branch.as_ref(), ExpData::Assign(_, _, _));

                    let else_is_flag_set =
                        if let ExpData::Assign(_, _, value) = else_branch.as_ref() {
                            matches!(value.as_ref(), ExpData::Value(_, Value::Bool(true)))
                        } else {
                            false
                        };

                    condition_is_flag && then_is_increment && else_is_flag_set
                } else {
                    false
                };

            let has_limit_check = statements[1..]
                .iter()
                .any(|stmt| Self::is_limit_check_statement(stmt.as_ref()));

            first_is_flag_management && has_limit_check
        } else {
            false
        }
    }

    /// Returns true if the statement matches the pattern of a limit check with break.
    fn is_limit_check_statement(stmt: &ExpData) -> bool {
        match stmt {
            ExpData::IfElse(_, condition, _then_branch, else_branch) => {
                let condition_is_comparison = if let ExpData::Call(_, op, args) = condition.as_ref()
                {
                    matches!(op, Operation::Lt) && args.len() == 2
                } else {
                    false
                };

                let else_is_break = matches!(else_branch.as_ref(), ExpData::LoopCont(_, 0, false));

                condition_is_comparison && else_is_break
            },
            ExpData::Sequence(_, nested_statements) => nested_statements
                .iter()
                .any(|nested_stmt| Self::is_limit_check_statement(nested_stmt.as_ref())),
            _ => false,
        }
    }

    /// Returns true if the Loop expression matches the pattern of a while loop.
    fn is_while_loop(&self, loop_body: &ExpData) -> bool {
        match loop_body {
            ExpData::IfElse(_, _, _, else_expr) => {
                matches!(else_expr.as_ref(), ExpData::LoopCont(_, nest, is_continue) if *nest == 0 && !*is_continue)
            },
            _ => false,
        }
    }

    /// Returns `true` iff the given `return` statement corresponds to the last
    /// top-level statement of the function body.
    fn is_final_return(function: &FunctionEnv, ret_node: NodeId) -> bool {
        if let Some(def) = function.get_def() {
            use move_model::ast::ExpData::*;
            return match def.as_ref() {
                Return(id, _) => *id == ret_node,
                Sequence(_, seq) => seq.last().map_or(
                    false,
                    |e| matches!(e.as_ref(), Return(id, _) if *id == ret_node),
                ),
                _ => false,
            };
        }
        false
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

impl ExpChecker for CyclomaticComplexity {
    fn get_name(&self) -> String {
        "cyclomatic_complexity".to_string()
    }

    fn visit_expr_pre(&mut self, function: &FunctionEnv, expr: &ExpData) {
        let fun_key: FunKey = (function.module_env.get_id(), function.get_id());
        if self.current_fun.map(|k| k != fun_key).unwrap_or(true) {
            self.current_fun = Some(fun_key);
            self.complexity = 1;
            self.reported = false;
            self.root_node = function.get_def().as_ref().map(|e| e.node_id());
        }

        use ExpData::*;
        match expr {
            // loop, while, for
            Loop(_, inner_expr) => {
                let delta = if self.is_expanded_for_loop(inner_expr.as_ref()) {
                    // the for is expanded into:
                    // ```
                    // loop {
                    //     if (true) {
                    //         if (flag) {
                    //             increment;
                    //         } else {
                    //             flag = true;
                    //         }
                    //         if (i < limit) {
                    //             body;
                    //         } else {
                    //             break;
                    //         }
                    //     } else {
                    //         break;
                    //     }
                    // }
                    // ```
                    // For loop expansion generates: Loop(+1) + IfElse(+1) + IfElse(+1) + LoopCont(+1) + LoopCont(+1) = +5 extra
                    // But we want for to count as +1 total, so we subtract 4 here
                    -4
                } else if self.is_while_loop(inner_expr.as_ref()) {
                    // the while is expanded into:
                    // ```
                    // loop {
                    //     if (condition) {
                    //         // loop body
                    //     } else {
                    //         break;
                    //     }
                    // }
                    // ```
                    // While loop expansion generates: Loop(+1) + IfElse(+1) + LoopCont(+1) = +3 extra
                    // But we want while to count as +1 total, so we subtract 1 here
                    -1
                } else {
                    // Regular loop
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
                if !CyclomaticComplexity::is_final_return(function, expr.node_id()) {
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
