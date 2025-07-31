// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module (and its submodules) contain various model-AST-based lint checks.

mod almost_swapped;

use move_compiler_v2::external_checks::ExpChecker;

/// Returns a default pipeline of "expression linters" to run.
pub fn get_default_linter_pipeline() -> Vec<Box<dyn ExpChecker>> {
    vec![
        Box::<almost_swapped::AlmostSwapped>::default(),
    ]
}
