// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Lint: error constant doc comments must be at most 128 bytes long.

use crate::module_lints::is_error_constant;
use move_compiler_v2::external_checks::ModuleChecker;
use move_model::model::{GlobalEnv, NamedConstantEnv};

const MAX_ERROR_MESSAGE_BYTES: usize = 128;

pub struct ErrorMessageTooLong;

impl ModuleChecker for ErrorMessageTooLong {
    fn get_name(&self) -> String {
        "error_message_too_long".to_string()
    }

    fn visit_named_constant(&self, env: &GlobalEnv, constant: &NamedConstantEnv) {
        let name = env.symbol_pool().string(constant.get_name());
        if !is_error_constant(name.as_str()) {
            return;
        }
        let doc = constant.get_doc();
        if doc.len() > MAX_ERROR_MESSAGE_BYTES {
            self.report(
                env,
                &constant.get_loc(),
                &format!(
                    "Error constant doc comment is {} bytes, which exceeds the {} byte limit.",
                    doc.len(),
                    MAX_ERROR_MESSAGE_BYTES
                ),
            );
        }
    }
}
