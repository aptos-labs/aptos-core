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

use move_compiler_v2::external_checks::StacklessBytecodeChecker;
use std::collections::BTreeMap;

/// Get default pipeline of "stackless bytecode linters" to run.
pub fn get_default_linter_pipeline(
    config: &BTreeMap<String, String>,
) -> Vec<Box<dyn StacklessBytecodeChecker>> {
    // Start with the default set of checks.
    let checks: Vec<Box<dyn StacklessBytecodeChecker>> = vec![
        Box::new(avoid_copy_on_identity_comparison::AvoidCopyOnIdentityComparison {}),
        Box::new(needless_mutable_reference::NeedlessMutableReference {}),
        Box::new(unreachable_code::UnreachableCode {}),
    ];
    let checks_category = config.get("checks").map_or("default", |s| s.as_str());
    if checks_category == "strict" || checks_category == "experimental" {
        // Push strict checks to `checks`.
    }
    if checks_category == "experimental" {
        // Push experimental checks to `checks`.
    }
    checks
}
