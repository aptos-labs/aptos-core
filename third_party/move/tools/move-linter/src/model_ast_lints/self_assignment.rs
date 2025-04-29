// Copyright (c) Aptos Foundation
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
    ast::{Exp, ExpData, Pattern},
    model::{GlobalEnv, Loc},
};

#[derive(Default)]
pub struct SelfAssignment;

impl ExpChecker for SelfAssignment {
    fn get_name(&self) -> String {
        "self_assignment".to_string()
    }

    fn visit_expr_pre(&mut self, env: &GlobalEnv, expr: &ExpData) {
        let mut report_loc = env.get_node_loc(expr.node_id());
        let (lhs, rhs) = match expr {
            ExpData::Mutate(_, lhs, rhs) => (lhs.clone(), rhs),
            ExpData::Assign(_, Pattern::Var(_, s), rhs) => {
                (Exp::from(ExpData::LocalVar(rhs.node_id(), *s)), rhs)
            },
            ExpData::Block(_, Pattern::Var(lhs_id, s), Some(rhs), _) => {
                report_loc = Loc::enclosing(&[
                    env.get_node_loc(*lhs_id).at_start(),
                    env.get_node_loc(rhs.node_id()).at_end(),
                ]);
                (Exp::from(ExpData::LocalVar(rhs.node_id(), *s)), rhs)
            },
            _ => return,
        };
        if utils::is_simple_access_equal(lhs.as_ref(), rhs) {
            self.report(
                env,
                &report_loc,
                "This is an unnecessary self assignment. Consider removing it.",
            );
        }
    }
}
