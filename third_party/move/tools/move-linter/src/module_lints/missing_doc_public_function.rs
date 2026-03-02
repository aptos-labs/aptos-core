// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Lint: public, entry, or `#[view]` functions must have doc comments.

use move_compiler_v2::external_checks::ModuleChecker;
use move_model::model::{FunctionEnv, GlobalEnv};

pub struct MissingDocPublicFunction;

impl ModuleChecker for MissingDocPublicFunction {
    fn get_name(&self) -> String {
        "missing_doc_public_function".to_string()
    }

    fn visit_function(&self, env: &GlobalEnv, func: &FunctionEnv) {
        let is_public = func.visibility() == move_model::model::Visibility::Public;
        let is_entry = func.is_entry();
        let is_view =
            func.has_attribute(|attr| env.symbol_pool().string(attr.name()).as_str() == "view");
        if (is_public || is_entry || is_view) && func.get_doc().is_empty() {
            let kind = if is_entry {
                "Entry"
            } else if is_view {
                "View"
            } else {
                "Public"
            };
            self.report(
                env,
                &func.get_id_loc(),
                &format!("{} function is missing a doc comment.", kind),
            );
        }
    }
}
