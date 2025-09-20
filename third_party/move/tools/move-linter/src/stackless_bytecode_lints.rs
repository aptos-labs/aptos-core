// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module (and its submodules) contain various stackless-bytecode-based lint checks.
//! Live variable analysis is a prerequisite for this lint processor.
//! The lint checks also assume that all the correctness checks have already been performed.

mod avoid_copy_on_identity_comparison;
mod needless_loops;
mod needless_mutable_reference;

use move_compiler_v2::external_checks::StacklessBytecodeChecker;
use std::collections::BTreeMap;

/// Get default pipeline of "stackless bytecode linters" to run.
pub fn get_default_linter_pipeline(
    config: &BTreeMap<String, String>,
) -> Vec<Box<dyn StacklessBytecodeChecker>> {
    // Start with the default set of checks.
    let checks: Vec<Box<dyn StacklessBytecodeChecker>> = vec![
        Box::new(avoid_copy_on_identity_comparison::AvoidCopyOnIdentityComparison {}),
        Box::new(needless_loops::NeedlessLoops {}),
        Box::new(needless_mutable_reference::NeedlessMutableReference {}),
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
