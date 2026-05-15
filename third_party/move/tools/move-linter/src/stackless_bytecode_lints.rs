// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module (and its submodules) contain various stackless-bytecode-based lint checks.
//!
//! Prerequisite analyses (must be registered before the lint processor in the pipeline):
//! - Live variable analysis.
//! - Reachable state analysis.
//!
//! When adding a new lint check that depends on additional analyses, register
//! those analyses as prerequisites and document them here.
//!
//! The lint checks also assume that all the correctness checks have already been performed.

mod avoid_copy_on_identity_comparison;
mod needless_mutable_reference;
mod unreachable_code;

use crate::{select_lints, LintSpec, LintTier};
use avoid_copy_on_identity_comparison::AvoidCopyOnIdentityComparison;
use move_compiler_v2::external_checks::StacklessBytecodeChecker;
use needless_mutable_reference::NeedlessMutableReference;
use unreachable_code::UnreachableCode;

/// Registry of every stackless-bytecode lint with its tier, unfiltered.
pub(crate) fn all_lints() -> Vec<(LintTier, Box<dyn StacklessBytecodeChecker>)> {
    use LintTier::Default;
    vec![
        // ── default tier ──────────────────────────────────────────────
        (Default, Box::new(AvoidCopyOnIdentityComparison {})),
        (Default, Box::new(NeedlessMutableReference {})),
        (Default, Box::new(UnreachableCode {})),
    ]
}

/// Stackless-bytecode checkers enabled by `spec`.
pub fn get_default_linter_pipeline(spec: &LintSpec) -> Vec<Box<dyn StacklessBytecodeChecker>> {
    select_lints(spec, all_lints(), |c| c.get_name())
}
