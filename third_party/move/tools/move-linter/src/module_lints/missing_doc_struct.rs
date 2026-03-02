// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Lint (strict): structs and enums must have doc comments.

use move_compiler_v2::external_checks::ModuleChecker;
use move_model::model::{GlobalEnv, StructEnv};

pub struct MissingDocStruct;

impl ModuleChecker for MissingDocStruct {
    fn get_name(&self) -> String {
        "missing_doc_struct".to_string()
    }

    fn visit_struct(&self, env: &GlobalEnv, struct_env: &StructEnv) {
        if struct_env.get_doc().is_empty() {
            let kind = if struct_env.has_variants() {
                "Enum"
            } else {
                "Struct"
            };
            self.report(
                env,
                &struct_env.get_loc(),
                &format!("{} is missing a doc comment.", kind),
            );
        }
    }
}
