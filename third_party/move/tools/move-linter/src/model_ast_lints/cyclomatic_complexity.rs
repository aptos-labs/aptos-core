// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Cyclomatic complexity measures the number of independent execution paths
//! through a function.  A high value generally correlates with code that is
//! harder to test and maintain.
//!
//! This linter performs an approximation while traversing the Move
//! expression tree:
//!   1. The complexity score starts at **1**.
//!   2. The score is incremented for each control-flow decision point found,
//!      including:
//!         * `if … else …` expressions
//!         * `loop`, `while`, and `for` constructs
//!         * `return` statements
//!         * `break`/`continue` (`LoopCont`) nodes
//!         * the number of `match` arms minus one (equivalent to counting the
//!           number of additional branches)
//!         * short-circuit boolean operators `&&` and `||`
//!
//! When the accumulated score exceeds `DEFAULT_THRESHOLD` (currently **10**),
//! the linter emits a diagnostic suggesting that the function be simplified or
//! decomposed.
//!
//! NOTE: The threshold is intentionally conservative.

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::ExpData,
    model::{FunctionEnv, Loc, NodeId},
};

const DEFAULT_THRESHOLD: usize = 10;

type FunKey = (move_model::model::ModuleId, move_model::model::FunId);

#[derive(Default)]
pub struct CyclomaticComplexity {
    complexity: usize,
    reported: bool,
    root_node: Option<NodeId>,
    current_fun: Option<FunKey>,
}

impl CyclomaticComplexity {
    fn bump(&mut self, delta: usize) {
        self.complexity = self.complexity.saturating_add(delta);
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
            IfElse(..) | Loop(..) | LoopCont(..) => self.bump(1),
            Return(_, _) => {
                if !CyclomaticComplexity::is_final_return(function, expr.node_id()) {
                    self.bump(1);
                }
            },
            Match(_, _, arms) if !arms.is_empty() => self.bump(arms.len() - 1),
            Call(_, op, _) => {
                use move_model::ast::Operation::{And, Or};
                if matches!(op, And | Or) {
                    self.bump(1);
                }
            },
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
