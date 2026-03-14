// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module exercises externally provided model-AST-based lint checks.

use crate::{
    external_checks::{
        known_checker_names, ConstantChecker, ExternalChecks, FunctionChecker, StructChecker,
    },
    lint_common::lint_skips_from_attributes,
    Options,
};
use move_model::{
    ast::ExpData,
    model::{FunctionEnv, GlobalEnv},
};
use std::{collections::BTreeSet, sync::Arc};

/// Perform various external lint checks on the model AST.
pub fn checker(env: &mut GlobalEnv) {
    let external_checks = &env
        .get_extension::<Options>()
        .expect("`Options` is available")
        .external_checks;
    if external_checks.is_empty() {
        return;
    }
    let known_checker_names = known_checker_names(external_checks);
    let constant_checkers = external_checks
        .iter()
        .flat_map(|c| c.get_constant_checkers())
        .collect::<Vec<_>>();
    let struct_checkers = external_checks
        .iter()
        .flat_map(|c| c.get_struct_checkers())
        .collect::<Vec<_>>();
    let function_checkers = external_checks
        .iter()
        .flat_map(|c| c.get_function_checkers())
        .collect::<Vec<_>>();
    for module in env.get_modules() {
        if module.is_primary_target() {
            let module_lint_skips =
                lint_skips_from_attributes(env, module.get_attributes(), &known_checker_names);
            for const_env in module.get_named_constants() {
                check_constant(
                    &const_env,
                    &constant_checkers,
                    &module_lint_skips,
                    &known_checker_names,
                );
            }
            for struct_env in module.get_structs() {
                check_struct(
                    &struct_env,
                    &struct_checkers,
                    &module_lint_skips,
                    &known_checker_names,
                );
            }
            for function in module.get_functions() {
                check_function(
                    &function,
                    &function_checkers,
                    &module_lint_skips,
                    &known_checker_names,
                );
                if function.is_native() {
                    continue;
                }
                check_exp(
                    &function,
                    external_checks,
                    &module_lint_skips,
                    &known_checker_names,
                );
            }
        }
    }
}

/// Run constant-level lint checks on a constant.
fn check_constant(
    const_env: &move_model::model::NamedConstantEnv,
    checkers: &[Box<dyn ConstantChecker>],
    module_lint_skips: &BTreeSet<String>,
    known_checker_names: &BTreeSet<String>,
) {
    let env = const_env.module_env.env;
    let lint_skips =
        lint_skips_from_attributes(env, const_env.get_attributes(), known_checker_names);
    for checker in checkers {
        if !is_lint_skipped(&checker.get_name(), module_lint_skips, &lint_skips) {
            checker.check_constant(const_env);
        }
    }
}

/// Run struct-level lint checks on a struct.
fn check_struct(
    struct_env: &move_model::model::StructEnv,
    checkers: &[Box<dyn StructChecker>],
    module_lint_skips: &BTreeSet<String>,
    known_checker_names: &BTreeSet<String>,
) {
    let env = struct_env.module_env.env;
    let lint_skips =
        lint_skips_from_attributes(env, struct_env.get_attributes(), known_checker_names);
    for checker in checkers {
        if !is_lint_skipped(&checker.get_name(), module_lint_skips, &lint_skips) {
            checker.check_struct(struct_env);
        }
    }
}

/// Run function-level lint checks on a function.
fn check_function(
    func_env: &FunctionEnv,
    checkers: &[Box<dyn FunctionChecker>],
    module_lint_skips: &BTreeSet<String>,
    known_checker_names: &BTreeSet<String>,
) {
    let env = func_env.env();
    let lint_skips =
        lint_skips_from_attributes(env, func_env.get_attributes(), known_checker_names);
    for checker in checkers {
        if !is_lint_skipped(&checker.get_name(), module_lint_skips, &lint_skips) {
            checker.check_function(func_env);
        }
    }
}

/// Perform expression-level lint checks on the code in `function`.
fn check_exp(
    function: &FunctionEnv,
    external_checks: &[Arc<dyn ExternalChecks>],
    module_lint_skips: &BTreeSet<String>,
    known_checker_names: &BTreeSet<String>,
) {
    let env = function.env();
    let function_lint_skips =
        lint_skips_from_attributes(env, function.get_attributes(), known_checker_names);
    // Unlike other checkers, exp checkers are recreated per function because
    // some implementations are stateful (`&mut self` in visit methods).
    let mut checkers = external_checks
        .iter()
        .flat_map(|c| c.get_exp_checkers())
        .filter(|c| !is_lint_skipped(&c.get_name(), module_lint_skips, &function_lint_skips))
        .collect::<Vec<_>>();
    if let Some(def) = function.get_def() {
        let mut visitor = |post: bool, e: &ExpData| {
            for exp_lint in checkers.iter_mut() {
                if !post {
                    exp_lint.visit_expr_pre(function, e);
                } else {
                    exp_lint.visit_expr_post(function, e);
                }
            }
            true
        };
        def.visit_pre_post(&mut visitor);
    }
}

fn is_lint_skipped(
    name: &str,
    module_lint_skips: &BTreeSet<String>,
    lint_skips: &BTreeSet<String>,
) -> bool {
    module_lint_skips.contains(name) || lint_skips.contains(name)
}
