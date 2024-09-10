// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for needless references
//! taken for field access.
//! E.g., `(&s).f` can be simplified to `s.f`.
//!       `(&mut s).f = 42;` can be simplified to `s.f = 42;`.
//! making code easier to read in these cases.

use crate::{env_pipeline::model_ast_lints::ExpressionLinter, lint_common::LintChecker};
use move_model::{
    ast::{ExpData, Operation},
    model::GlobalEnv,
};

#[derive(Default)]
pub struct NeedlessRefInFieldAccess;

impl ExpressionLinter for NeedlessRefInFieldAccess {
    fn get_lint_checker(&self) -> LintChecker {
        LintChecker::NeedlessRefInFieldAccess
    }

    fn visit_expr_pre(&mut self, env: &GlobalEnv, expr: &ExpData) {
        use ExpData::Call;
        if let Call(_, Operation::Select(.., field_id), args) = expr {
            debug_assert!(
                args.len() == 1,
                "there should be exactly one argument for field access"
            );
            if let Call(id, Operation::Borrow(kind), ..) = args[0].as_ref() {
                let field_name = field_id.symbol().display(env.symbol_pool()).to_string();
                let ref_kind = kind.to_string();
                self.warning(
                    env,
                    &env.get_node_loc(*id),
                    &format!(
                        "Needless {} taken for field access: \
                        consider removing {} and directly accessing the field `{}`",
                        ref_kind, ref_kind, field_name
                    ),
                );
            }
        }
    }
}
