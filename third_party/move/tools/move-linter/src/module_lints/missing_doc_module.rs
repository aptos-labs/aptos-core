// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Lint (strict): modules must have a doc comment.

use move_compiler_v2::external_checks::ModuleChecker;
use move_model::model::{GlobalEnv, ModuleEnv};

pub struct MissingDocModule;

impl ModuleChecker for MissingDocModule {
    fn get_name(&self) -> String {
        "missing_doc_module".to_string()
    }

    fn visit_module(&self, env: &GlobalEnv, module: &ModuleEnv) {
        if module.get_doc().is_empty() {
            self.report(env, &module.get_loc(), "Module is missing a doc comment.");
        }
    }
}
