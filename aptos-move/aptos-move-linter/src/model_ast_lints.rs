// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module (and its submodules) contain various model-AST-based lint checks.

use move_compiler_v2::external_checks::ExpChecker;
use std::collections::BTreeMap;

/// Returns a default pipeline of "expression linters" to run.
pub fn get_default_linter_pipeline(_config: &BTreeMap<String, String>) -> Vec<Box<dyn ExpChecker>> {
    vec![]
}
