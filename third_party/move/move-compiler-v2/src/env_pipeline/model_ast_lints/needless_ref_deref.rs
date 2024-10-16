// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for immutable reference
//! taken for a dereference (`&*`). Such pairs of operators are needless and can be
//! removed to make the code easier to read.

use crate::{env_pipeline::model_ast_lints::ExpressionLinter, lint_common::LintChecker};
use move_model::{
    ast::{ExpData, Operation},
    model::GlobalEnv,
    ty::ReferenceKind,
};

#[derive(Default)]
pub struct NeedlessRefDeref;

impl ExpressionLinter for NeedlessRefDeref {
    fn get_lint_checker(&self) -> LintChecker {
        LintChecker::NeedlessRefDeref
    }

    fn visit_expr_pre(&mut self, env: &GlobalEnv, expr: &ExpData) {
        use ExpData::Call;
        use Operation::{Borrow, Deref};
        use ReferenceKind::Immutable;
        let Call(id, Borrow(Immutable), args) = expr else {
            return;
        };
        debug_assert!(
            args.len() == 1,
            "there should be exactly one argument for borrow"
        );
        let Call(_, Deref, _) = args[0].as_ref() else {
            return;
        };
        self.warning(
            env,
            &env.get_node_loc(*id),
            "Needless pair of `&` and `*` operators: consider removing them",
        );
    }
}
