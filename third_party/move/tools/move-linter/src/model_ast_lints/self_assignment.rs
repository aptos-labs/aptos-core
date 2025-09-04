// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for expression patterns
//! that perform a self assignment. This currently only detects simple access
//! patterns such as `a = a;`, `a.b = a.b;`, and `let a = a;`. Notably, this does
//! not detect patterns involving non-builtin functions (including vector operations)
//! nor does it detect patterns that deal with global storage. Support for these cases
//! may be added in the future.
use crate::utils;
use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{ExpData, Pattern},
    model::{FunctionEnv, Loc},
};

#[derive(Default)]
pub struct SelfAssignment;

impl ExpChecker for SelfAssignment {
    fn get_name(&self) -> String {
        "self_assignment".to_string()
    }

    fn visit_expr_pre(&mut self, function: &FunctionEnv, expr: &ExpData) {
        let env = function.env();
        let mut report_loc: Loc = env.get_node_loc(expr.node_id());
        let mut error = false;
        match expr {
            ExpData::Mutate(_, lhs, rhs) => {
                if utils::is_simple_access_equal(lhs.as_ref(), rhs) {
                    error = true;
                }
            },
            ExpData::Assign(_, Pattern::Var(_, lhs_sym), rhs) => {
                if let ExpData::LocalVar(_, rhs_sym) = rhs.as_ref() {
                    if lhs_sym == rhs_sym {
                        error = true;
                    }
                }
            },
            ExpData::Block(_, Pattern::Var(lhs_id, lhs_sym), Some(rhs), _) => {
                if let ExpData::LocalVar(_, rhs_sym) = rhs.as_ref() {
                    if lhs_sym == rhs_sym {
                        error = true;
                        report_loc = Loc::enclosing(&[
                            env.get_node_loc(*lhs_id).at_start(),
                            env.get_node_loc(rhs.node_id()).at_end(),
                        ]);
                    }
                }
            },
            _ => return,
        };
        if error {
            self.report(
                env,
                &report_loc,
                "This is an unnecessary self assignment. Consider removing it.",
            );
        }
    }
}
