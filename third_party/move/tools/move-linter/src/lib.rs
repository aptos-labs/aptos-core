// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

mod model_ast_lints;
mod stackless_bytecode_lints;
mod utils;

use move_compiler_v2::external_checks::{ExpChecker, ExternalChecks, StacklessBytecodeChecker};
use std::{collections::BTreeMap, sync::Arc};

/// Holds collection of lint checks for Move.
pub struct MoveLintChecks {
    config: BTreeMap<String, String>,
}

impl ExternalChecks for MoveLintChecks {
    fn get_exp_checkers(&self) -> Vec<Box<dyn ExpChecker>> {
        model_ast_lints::get_default_linter_pipeline(&self.config)
    }

    fn get_stackless_bytecode_checkers(&self) -> Vec<Box<dyn StacklessBytecodeChecker>> {
        stackless_bytecode_lints::get_default_linter_pipeline(&self.config)
    }
}

impl MoveLintChecks {
    /// Make an instance of lint checks for Move, provided as `ExternalChecks`.
    /// Will panic if the configuration is not valid.
    pub fn make(config: BTreeMap<String, String>) -> Arc<dyn ExternalChecks> {
        // Check whether the config map is valid.
        // Currently, we expect it to contain a single key "checks" that can map to
        // one of the three string values: "default", "strict", or "experimental".
        if config.len() != 1 {
            panic!("config should have a single key `checks`");
        }
        let checks_value = config
            .get("checks")
            .expect("config is missing the `checks` key");
        if !matches!(checks_value.as_str(), "default" | "strict" | "experimental") {
            panic!("Invalid value for `checks` key in the config, expected one of: `default`, `strict`, or `experimental`");
        }
        Arc::new(MoveLintChecks { config })
    }
}
