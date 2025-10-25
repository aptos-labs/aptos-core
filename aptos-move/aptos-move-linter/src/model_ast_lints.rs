// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module (and its submodules) contain various model-AST-based lint checks.

use move_compiler_v2::external_checks::ExpChecker;
use std::collections::BTreeMap;
mod random_modulo;

/// Returns a default pipeline of "expression linters" to run.
pub fn get_default_linter_pipeline(config: &BTreeMap<String, String>) -> Vec<Box<dyn ExpChecker>> {
    #[allow(unused_mut)]
    let mut checks: Vec<Box<dyn ExpChecker>> = vec![Box::<random_modulo::RandomModulo>::default()];
    let checks_category = config.get("checks").map_or("default", |s| s.as_str());
    if checks_category == "strict" || checks_category == "experimental" {
        // Push strict checks to `checks`.
    }
    if checks_category == "experimental" {
        // Push experimental checks to `checks`.
    }
    checks
}
