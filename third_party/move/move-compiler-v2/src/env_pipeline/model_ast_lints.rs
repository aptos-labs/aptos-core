// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module exercises externally provided model-AST-based lint checks.

use crate::{
    external_checks::{known_checker_names, ExpChecker, ModuleChecker},
    lint_common::lint_skips_from_attributes,
    Options,
};
use move_model::{
    ast::ExpData,
    model::{FunctionEnv, GlobalEnv, ModuleEnv},
};
use std::collections::BTreeSet;

/// Perform various external lint checks on the model AST.
pub fn checker(env: &mut GlobalEnv) {
    let options = env
        .get_extension::<Options>()
        .expect("Options is available");
    if options.external_checks.is_empty() {
        return;
    }
    let known_checker_names = known_checker_names(&options.external_checks);
    for module in env.get_modules() {
        if module.is_primary_target() {
            let module_lint_skips =
                lint_skips_from_attributes(env, module.get_attributes(), &known_checker_names);
            for function in module.get_functions() {
                if function.is_native() {
                    continue;
                }
                check_function(&function, &module_lint_skips, &known_checker_names);
            }
            run_module_checkers(env, &module, &module_lint_skips, &known_checker_names);
        }
    }
}

/// Perform the lint checks on the code in `function`.
fn check_function(
    function: &FunctionEnv,
    module_lint_skips: &BTreeSet<String>,
    known_checker_names: &BTreeSet<String>,
) {
    let env = function.env();
    let function_lint_skips =
        lint_skips_from_attributes(env, function.get_attributes(), known_checker_names);
    let mut lint_skips = BTreeSet::from_iter(function_lint_skips);
    lint_skips.extend(module_lint_skips.clone());
    let mut expression_linters = get_applicable_lints(function, lint_skips);
    if let Some(def) = function.get_def() {
        let mut visitor = |post: bool, e: &ExpData| {
            if !post {
                for exp_lint in expression_linters.iter_mut() {
                    exp_lint.visit_expr_pre(function, e);
                }
            } else {
                for exp_lint in expression_linters.iter_mut() {
                    exp_lint.visit_expr_post(function, e);
                }
            }
            true
        };
        def.visit_pre_post(&mut visitor);
    }
}

/// Returns a pipeline of "expression linters" to run, skipping the ones in `lint_skips`.
fn get_applicable_lints(
    function_env: &FunctionEnv,
    lint_skips: BTreeSet<String>,
) -> Vec<Box<dyn ExpChecker>> {
    let options = function_env
        .module_env
        .env
        .get_extension::<Options>()
        .expect("Options is available");
    options
        .external_checks
        .iter()
        .flat_map(|checks| {
            checks
                .get_exp_checkers()
                .into_iter()
                .filter(|lint| !lint_skips.contains(&lint.get_name()))
        })
        .collect()
}

/// Run module-level checkers on the given module and its declarations.
fn run_module_checkers(
    env: &GlobalEnv,
    module: &ModuleEnv,
    module_lint_skips: &BTreeSet<String>,
    known_checker_names: &BTreeSet<String>,
) {
    let options = env
        .get_extension::<Options>()
        .expect("Options is available");
    let all_module_checkers: Vec<Box<dyn ModuleChecker>> = options
        .external_checks
        .iter()
        .flat_map(|checks| checks.get_module_checkers())
        .collect();
    if all_module_checkers.is_empty() {
        return;
    }

    // Visit the module itself with checkers not skipped at module level.
    for checker in &all_module_checkers {
        if !module_lint_skips.contains(&checker.get_name()) {
            checker.visit_module(env, module);
        }
    }

    // Visit functions (non-native, non-test).
    for function in module.get_functions() {
        if function.is_native() || function.is_test_or_verify_only() {
            continue;
        }
        let function_skips =
            lint_skips_from_attributes(env, function.get_attributes(), known_checker_names);
        for checker in &all_module_checkers {
            let name = checker.get_name();
            if !module_lint_skips.contains(&name) && !function_skips.contains(&name) {
                checker.visit_function(env, &function);
            }
        }
    }

    // Visit named constants (non-test).
    for constant in module.get_named_constants() {
        if constant.is_test_or_verify_only() {
            continue;
        }
        let constant_skips =
            lint_skips_from_attributes(env, constant.get_attributes(), known_checker_names);
        for checker in &all_module_checkers {
            let name = checker.get_name();
            if !module_lint_skips.contains(&name) && !constant_skips.contains(&name) {
                checker.visit_named_constant(env, &constant);
            }
        }
    }

    // Visit structs/enums (non-test, non-ghost).
    for struct_env in module.get_structs() {
        if struct_env.is_test_or_verify_only() || struct_env.is_ghost_memory() {
            continue;
        }
        let struct_skips =
            lint_skips_from_attributes(env, struct_env.get_attributes(), known_checker_names);
        for checker in &all_module_checkers {
            let name = checker.get_name();
            if !module_lint_skips.contains(&name) && !struct_skips.contains(&name) {
                checker.visit_struct(env, &struct_env);
            }
        }
    }
}
