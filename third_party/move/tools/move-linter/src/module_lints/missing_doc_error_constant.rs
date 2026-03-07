// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Lint: error constants (E-prefix) must have a doc comment.
//! The doc comment serves as a human-readable error message.

use crate::module_lints::is_error_constant;
use move_compiler_v2::external_checks::ModuleChecker;
use move_model::model::{GlobalEnv, NamedConstantEnv};

pub struct MissingDocErrorConstant;

impl ModuleChecker for MissingDocErrorConstant {
    fn get_name(&self) -> String {
        "missing_doc_error_constant".to_string()
    }

    fn visit_named_constant(&self, env: &GlobalEnv, constant: &NamedConstantEnv) {
        let name = env.symbol_pool().string(constant.get_name());
        if !is_error_constant(name.as_str()) {
            return;
        }
        if constant.get_doc().is_empty() {
            self.report(
                env,
                &constant.get_loc(),
                "Error constant is missing a doc comment. The doc comment is used as a human-readable error message.",
            );
        }
    }
}
