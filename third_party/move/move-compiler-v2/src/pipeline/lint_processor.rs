// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module exercises externally provided stackless-bytecode-based lint checks.
//! Live variable analysis is a prerequisite for this lint processor.
//! The lint checks also assume that all the correctness checks have already been performed.

use crate::{
    external_checks::{known_checker_names, StacklessBytecodeChecker},
    lint_common::lint_skips_from_attributes,
    Options,
};
use move_model::model::FunctionEnv;
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
};
use std::collections::BTreeSet;

/// The top-level processor for the stackless bytecode lint pipeline.
pub struct LintProcessor {}

impl FunctionTargetProcessor for LintProcessor {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if func_env.is_native() {
            return data;
        }
        let target = FunctionTarget::new(func_env, &data);
        if !target.module_env().is_primary_target() {
            return data;
        }
        let linters = Self::get_applicable_linter_pipeline(&target);
        for lint in linters {
            lint.check(&target);
        }
        data
    }

    fn name(&self) -> String {
        "LintProcessor".to_string()
    }
}

impl LintProcessor {
    /// Returns a filtered pipeline of stackless bytecode linters to run.
    /// The filtering is based on attributes attached to the function and module.
    pub fn get_applicable_linter_pipeline(
        target: &FunctionTarget,
    ) -> Vec<Box<dyn StacklessBytecodeChecker>> {
        let options = target
            .global_env()
            .get_extension::<Options>()
            .expect("Options is available");
        if options.external_checks.is_empty() {
            return vec![];
        }
        let known_checker_names = known_checker_names(&options.external_checks);
        let lint_skips = Self::get_lint_skips(target, &known_checker_names);
        options
            .external_checks
            .iter()
            .flat_map(|checks| {
                checks
                    .get_stackless_bytecode_checkers()
                    .into_iter()
                    .filter(|lint| !lint_skips.contains(&lint.get_name()))
            })
            .collect()
    }

    /// Get the set of lint checks to skip based on attributes attached to the function and module.
    fn get_lint_skips(
        target: &FunctionTarget,
        known_checker_names: &BTreeSet<String>,
    ) -> BTreeSet<String> {
        let module_lint_skips = lint_skips_from_attributes(
            target.global_env(),
            target.module_env().get_attributes(),
            known_checker_names,
        );
        let function_lint_skips = lint_skips_from_attributes(
            target.global_env(),
            target.func_env.get_attributes(),
            known_checker_names,
        );
        BTreeSet::from_iter(module_lint_skips.into_iter().chain(function_lint_skips))
    }
}
