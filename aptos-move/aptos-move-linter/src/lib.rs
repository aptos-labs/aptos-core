// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod model_ast_lints;
mod stackless_bytecode_lints;
mod utils;

use move_compiler_v2::external_checks::{ExpChecker, ExternalChecks, StacklessBytecodeChecker};
use std::sync::Arc;

/// Holds collection of lint checks for Move.
pub struct SecurityChecks {}

impl ExternalChecks for SecurityChecks {
    fn get_exp_checkers(&self) -> Vec<Box<dyn ExpChecker>> {
        model_ast_lints::get_default_linter_pipeline()
    }

    fn get_stackless_bytecode_checkers(&self) -> Vec<Box<dyn StacklessBytecodeChecker>> {
        stackless_bytecode_lints::get_default_linter_pipeline()
    }
}

impl SecurityChecks {
    /// Make an instance of lint checks for Move, provided as `ExternalChecks`.
    pub fn make() -> Arc<dyn ExternalChecks> {
        Arc::new(SecurityChecks {})
    }
}
