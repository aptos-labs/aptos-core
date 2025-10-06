// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module (and its submodules) contain various stackless-bytecode-based lint checks.
//! Live variable analysis is a prerequisite for this lint processor.
//! The lint checks also assume that all the correctness checks have already been performed.

mod contains_in_table;
use move_compiler_v2::external_checks::StacklessBytecodeChecker;
use std::collections::BTreeMap;

/// Get default pipeline of "stackless bytecode linters" to run.
pub fn get_default_linter_pipeline(
    config: &BTreeMap<String, String>,
) -> Vec<Box<dyn StacklessBytecodeChecker>> {
    let mut ret: Vec<Box<dyn StacklessBytecodeChecker>> = vec![];

    let checks_category = config.get("checks").map_or("default", |s| s.as_str());
    if checks_category == "experimental" {
        ret.push(Box::<contains_in_table::ContainsInTable>::default());
    }

    ret
}
