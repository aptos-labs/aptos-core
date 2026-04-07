// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod model_ast_lints;
mod stackless_bytecode_lints;
mod utils;

use move_compiler_v2::external_checks::{
    ConstantChecker, ExpChecker, ExternalChecks, FunctionChecker, StacklessBytecodeChecker,
    StructChecker,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// Holds collection of lint checks for Move.
pub struct MoveLintChecks {
    config: BTreeMap<String, String>,
    all_checker_names: BTreeSet<String>,
}

impl ExternalChecks for MoveLintChecks {
    fn get_exp_checkers(&self) -> Vec<Box<dyn ExpChecker>> {
        model_ast_lints::get_default_exp_linter_pipeline(&self.config)
    }

    fn get_stackless_bytecode_checkers(&self) -> Vec<Box<dyn StacklessBytecodeChecker>> {
        stackless_bytecode_lints::get_default_linter_pipeline(&self.config)
    }

    fn get_constant_checkers(&self) -> Vec<Box<dyn ConstantChecker>> {
        model_ast_lints::get_default_constant_linter_pipeline(&self.config)
    }

    fn get_struct_checkers(&self) -> Vec<Box<dyn StructChecker>> {
        model_ast_lints::get_default_struct_linter_pipeline(&self.config)
    }

    fn get_function_checkers(&self) -> Vec<Box<dyn FunctionChecker>> {
        model_ast_lints::get_default_function_linter_pipeline(&self.config)
    }

    fn get_all_checker_names(&self) -> BTreeSet<String> {
        self.all_checker_names.clone()
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
        // Precompute all checker names using the "experimental" tier config,
        // which is the superset of all tiers, so that all checker names are
        // recognized in `#[lint::skip(...)]` attributes.
        let all_config = BTreeMap::from([("checks".to_string(), "experimental".to_string())]);
        let mut all_checker_names = BTreeSet::new();
        for c in model_ast_lints::get_default_exp_linter_pipeline(&all_config) {
            all_checker_names.insert(c.get_name());
        }
        for c in stackless_bytecode_lints::get_default_linter_pipeline(&all_config) {
            all_checker_names.insert(c.get_name());
        }
        for c in model_ast_lints::get_default_constant_linter_pipeline(&all_config) {
            all_checker_names.insert(c.get_name());
        }
        for c in model_ast_lints::get_default_struct_linter_pipeline(&all_config) {
            all_checker_names.insert(c.get_name());
        }
        for c in model_ast_lints::get_default_function_linter_pipeline(&all_config) {
            all_checker_names.insert(c.get_name());
        }
        // TODO: instead of storing a key-value config map, store a typed representation.
        Arc::new(MoveLintChecks {
            config,
            all_checker_names,
        })
    }
}
