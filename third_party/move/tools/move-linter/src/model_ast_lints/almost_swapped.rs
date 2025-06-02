// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for expression patterns
//! that look like a failed swap attempt. This currently only detects simple access
//! patterns such as `a = b; b = a;`, and `a.b = c.d; c.d = a.b;`. Notably, this does
//! not detect patterns involving non-builtin functions (including vector operations)
//! nor does it detect patterns that deal with global storage. Support for these cases
//! may be added in the future.
use crate::utils;
use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{Exp, ExpData, ExpData::Sequence, Pattern},
    model::{GlobalEnv, Loc},
};

#[derive(Default)]
pub struct AlmostSwapped;

impl ExpChecker for AlmostSwapped {
    fn get_name(&self) -> String {
        "almost_swapped".to_string()
    }

    fn visit_expr_pre(&mut self, env: &GlobalEnv, expr: &ExpData) {
        if let Sequence(_, exprs) = expr {
            for pair in exprs.windows(2) {
                let (first, second) = (&pair[0], &pair[1]);
                let (lhs1, rhs1, lhs2, rhs2) = match (first.as_ref(), second.as_ref()) {
                    (ExpData::Mutate(_, e1, e2), ExpData::Mutate(_, e3, e4)) => {
                        (e1.clone(), e2.clone(), e3.clone(), e4.clone())
                    },
                    // The order of expressions does not matter
                    (ExpData::Mutate(_, e1, e2), ExpData::Assign(_, Pattern::Var(_, v1), e4))
                    | (ExpData::Assign(_, Pattern::Var(_, v1), e4), ExpData::Mutate(_, e1, e2)) => {
                        // The node id isn't used, so any node id will work.
                        let new_local = Exp::from(ExpData::LocalVar(e4.node_id(), *v1));
                        (e1.clone(), e2.clone(), new_local, e4.clone())
                    },
                    (
                        ExpData::Assign(_, Pattern::Var(_, v1), e2),
                        ExpData::Assign(_, Pattern::Var(_, v2), e4),
                    ) => {
                        // The node id isn't used, so any node id will work.
                        let l1 = Exp::from(ExpData::LocalVar(e2.node_id(), *v1));
                        let l2 = Exp::from(ExpData::LocalVar(e4.node_id(), *v2));
                        (l1, e2.clone(), l2, e4.clone())
                    },
                    _ => continue,
                };
                if utils::is_simple_access_equal(lhs1.as_ref(), rhs2.as_ref())
                    && utils::is_simple_access_equal(rhs1.as_ref(), lhs2.as_ref())
                {
                    let new_loc = Loc::enclosing(&[
                        env.get_node_loc(first.node_id()),
                        env.get_node_loc(second.node_id()),
                    ]);
                    self.report(
                        env,
                        &new_loc,
                        "This looks like a swap, but one assignment overwrites the other.",
                    );
                }
            }
        }
    }
}
