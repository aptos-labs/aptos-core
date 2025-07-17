// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module (and its submodules) contain various stackless-bytecode-based lint checks.
//! Live variable analysis is a prerequisite for this lint processor.
//! The lint checks also assume that all the correctness checks have already been performed.

use move_compiler_v2::external_checks::StacklessBytecodeChecker;

/// Get default pipeline of "stackless bytecode linters" to run.
pub fn get_default_linter_pipeline() -> Vec<Box<dyn StacklessBytecodeChecker>> {
    vec![]
}
