// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Lint: every constant must have a doc comment.
//! Error constants (E-prefix) are skipped here to avoid overlap with `missing_doc_error_constant`.

use crate::module_lints::is_error_constant;
use move_compiler_v2::external_checks::ModuleChecker;
use move_model::model::{GlobalEnv, NamedConstantEnv};

pub struct MissingDocConstant;

impl ModuleChecker for MissingDocConstant {
    fn get_name(&self) -> String {
        "missing_doc_constant".to_string()
    }

    fn visit_named_constant(&self, env: &GlobalEnv, constant: &NamedConstantEnv) {
        let name = env.symbol_pool().string(constant.get_name());
        // Skip error constants — handled by missing_doc_error_constant lint.
        if is_error_constant(name.as_str()) {
            return;
        }
        if constant.get_doc().is_empty() {
            self.report(
                env,
                &constant.get_loc(),
                "Constant is missing a doc comment.",
            );
        }
    }
}
