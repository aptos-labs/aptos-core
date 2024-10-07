// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module (and its submodules) contain various stackless-bytecode-based lint checks.
//! Live variable analysis is a prerequisite for this lint processor.
//! The lint checks also assume that all the correctness checks have already been performed.

mod avoid_copy_on_identity_comparison;
mod needless_mutable_reference;

use crate::{
    lint_common::{lint_skips_from_attributes, LintChecker},
    pipeline::lint_processor::{
        avoid_copy_on_identity_comparison::AvoidCopyOnIdentityComparison,
        needless_mutable_reference::NeedlessMutableReference,
    },
};
use move_compiler::shared::known_attributes::LintAttribute;
use move_model::model::{FunctionEnv, GlobalEnv, Loc};
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
};
use std::collections::BTreeSet;

/// Perform various lint checks on the stackless bytecode.
pub trait StacklessBytecodeLinter {
    /// The corresponding lint checker enumerated value.
    fn get_lint_checker(&self) -> LintChecker;

    /// Examine the `target` and potentially emit lint warnings via `self.warning()`.
    fn check(&self, target: &FunctionTarget);

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

/// The top-level processor for the lint pipeline.
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
    /// Returns the default pipeline of stackless bytecode linters to run.
    fn get_default_linter_pipeline() -> Vec<Box<dyn StacklessBytecodeLinter>> {
        vec![
            Box::new(AvoidCopyOnIdentityComparison {}),
            Box::new(NeedlessMutableReference {}),
        ]
    }

    /// Returns a filtered pipeline of stackless bytecode linters to run.
    /// The filtering is based on attributes attached to the function and module.
    pub fn get_applicable_linter_pipeline(
        target: &FunctionTarget,
    ) -> Vec<Box<dyn StacklessBytecodeLinter>> {
        let lint_skips = Self::get_lint_skips(target);
        Self::get_default_linter_pipeline()
            .into_iter()
            .filter(|lint| !lint_skips.contains(&lint.get_lint_checker()))
            .collect()
    }

    /// Get the set of lint checks to skip based on attributes attached to the function and module.
    fn get_lint_skips(target: &FunctionTarget) -> BTreeSet<LintChecker> {
        let module_lint_skips =
            lint_skips_from_attributes(target.global_env(), target.module_env().get_attributes());
        let function_lint_skips =
            lint_skips_from_attributes(target.global_env(), target.func_env.get_attributes());
        BTreeSet::from_iter(module_lint_skips.into_iter().chain(function_lint_skips))
    }
}
