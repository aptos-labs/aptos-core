// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module (and its submodules) contain various model-AST-based lint checks.

mod blocks_in_conditions;
mod unnecessary_boolean_identity_comparison;
mod unnecessary_numerical_extreme_comparison;
mod while_true;

use move_model::{
    ast::ExpData,
    model::{FunctionEnv, GlobalEnv},
};

/// Perform various lint checks on the model AST.
pub fn checker(env: &mut GlobalEnv) {
    for module in env.get_modules() {
        if module.is_primary_target() {
            for function in module.get_functions() {
                if function.is_native() {
                    continue;
                }
                check_function(&function);
            }
        }
    }
}

/// Implement this trait for lint checks that can be performed by looking at an
/// expression as we traverse the model AST.
/// Implement at least one of the `visit` methods to be a useful lint.
trait ExpressionLinter {
    /// The name of the lint.
    fn get_name(&self) -> &'static str;

    /// Examine `expr` before any of its children have been visited.
    /// Potentially emit lint warnings using `env.lint_diag()`.
    fn visit_expr_pre(&mut self, _env: &GlobalEnv, _expr: &ExpData) {}

    /// Examine `expr` after all its children have been visited.
    /// Potentially emit lint warnings using `env.lint_diag()`.
    fn visit_expr_post(&mut self, _env: &GlobalEnv, _expr: &ExpData) {}
}

/// Perform the lint checks on the code in `function`.
fn check_function(function: &FunctionEnv) {
    let mut expression_linters = get_expression_linter_pipeline();
    if let Some(def) = function.get_def() {
        let mut visitor = |post: bool, e: &ExpData| {
            let env = function.module_env.env;
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

/// Returns a pipeline of "expression linters" to run.
fn get_expression_linter_pipeline() -> Vec<Box<dyn ExpressionLinter>> {
    vec![
        Box::<blocks_in_conditions::BlocksInConditions>::default(),
        Box::<unnecessary_boolean_identity_comparison::UnnecessaryBooleanIdentityComparison>::default(),
        Box::<unnecessary_numerical_extreme_comparison::UnnecessaryNumericalExtremeComparison>::default(),
        Box::<while_true::WhileTrue>::default(),
    ]
}
