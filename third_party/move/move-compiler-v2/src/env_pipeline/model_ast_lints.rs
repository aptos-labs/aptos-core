// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module (and its submodules) contain various model-AST-based lint checks.

mod blocks_in_conditions;
mod needless_bool;
mod needless_deref_ref;
mod needless_ref_deref;
mod needless_ref_in_field_access;
mod simpler_numeric_expression;
mod unnecessary_boolean_identity_comparison;
mod unnecessary_numerical_extreme_comparison;
mod while_true;

use crate::lint_common::{lint_skips_from_attributes, LintChecker};
use move_compiler::shared::known_attributes::LintAttribute;
use move_model::{
    ast::ExpData,
    model::{FunctionEnv, GlobalEnv, Loc},
};
use std::collections::BTreeSet;

/// Perform various lint checks on the model AST.
pub fn checker(env: &mut GlobalEnv) {
    for module in env.get_modules() {
        if module.is_primary_target() {
            let module_lint_skips = lint_skips_from_attributes(env, module.get_attributes());
            for function in module.get_functions() {
                if function.is_native() {
                    continue;
                }
                check_function(&function, &module_lint_skips);
            }
        }
    }
}

/// Implement this trait for lint checks that can be performed by looking at an
/// expression as we traverse the model AST.
/// Implement at least one of the `visit` methods to be a useful lint.
trait ExpressionLinter {
    /// The corresponding lint checker enumerated value.
    fn get_lint_checker(&self) -> LintChecker;

    /// Examine `expr` before any of its children have been visited.
    /// Potentially emit lint warnings using `self.warning()`.
    fn visit_expr_pre(&mut self, _env: &GlobalEnv, _expr: &ExpData) {}

    /// Examine `expr` after all its children have been visited.
    /// Potentially emit lint warnings using `self.warning()`.
    fn visit_expr_post(&mut self, _env: &GlobalEnv, _expr: &ExpData) {}

    /// Emit a lint warning with the `msg` highlighting the `loc`.
    fn warning(&self, env: &GlobalEnv, loc: &Loc, msg: &str) {
        env.lint_diag_with_notes(loc, msg, vec![
            format!(
                "To suppress this warning, annotate the function/module with the attribute `#[{}({})]`.",
                LintAttribute::SKIP,
                self.get_lint_checker()
            ),
        ]);
    }
}

/// Perform the lint checks on the code in `function`.
fn check_function(function: &FunctionEnv, module_lint_skips: &[LintChecker]) {
    let env = function.module_env.env;
    let function_lint_skips = lint_skips_from_attributes(env, function.get_attributes());
    let mut lint_skips = BTreeSet::from_iter(function_lint_skips);
    lint_skips.extend(module_lint_skips);
    let mut expression_linters = get_applicable_lints(lint_skips);
    if let Some(def) = function.get_def() {
        let mut visitor = |post: bool, e: &ExpData| {
            if !post {
                for exp_lint in expression_linters.iter_mut() {
                    exp_lint.visit_expr_pre(env, e);
                }
            } else {
                for exp_lint in expression_linters.iter_mut() {
                    exp_lint.visit_expr_post(env, e);
                }
            }
            true
        };
        def.visit_pre_post(&mut visitor);
    }
}

/// Returns a pipeline of "expression linters" to run, skipping the ones in `lint_skips`.
fn get_applicable_lints(lint_skips: BTreeSet<LintChecker>) -> Vec<Box<dyn ExpressionLinter>> {
    get_default_expression_linter_pipeline()
        .into_iter()
        .filter(|lint| !lint_skips.contains(&lint.get_lint_checker()))
        .collect()
}

/// Returns a default pipeline of "expression linters" to run.
fn get_default_expression_linter_pipeline() -> Vec<Box<dyn ExpressionLinter>> {
    vec![
        Box::<blocks_in_conditions::BlocksInConditions>::default(),
        Box::<needless_bool::NeedlessBool>::default(),
        Box::<needless_ref_in_field_access::NeedlessRefInFieldAccess>::default(),
        Box::<needless_deref_ref::NeedlessDerefRef>::default(),
        Box::<needless_ref_deref::NeedlessRefDeref>::default(),
        Box::<simpler_numeric_expression::SimplerNumericExpression>::default(),
        Box::<unnecessary_boolean_identity_comparison::UnnecessaryBooleanIdentityComparison>::default(),
        Box::<unnecessary_numerical_extreme_comparison::UnnecessaryNumericalExtremeComparison>::default(),
        Box::<while_true::WhileTrue>::default(),
    ]
}
