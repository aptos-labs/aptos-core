// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module (and its submodules) contain various module-level lint checks.
//! These check declarations (modules, functions, constants, structs) rather than
//! expressions within function bodies.

mod error_constant_naming;
mod error_message_too_long;
mod missing_doc_constant;
mod missing_doc_error_constant;
mod missing_doc_module;
mod missing_doc_public_function;
mod missing_doc_struct;
mod prefer_doc_comment;

use move_compiler_v2::external_checks::ModuleChecker;
use std::collections::BTreeMap;

/// Returns a pipeline of module-level linters to run.
pub fn get_default_linter_pipeline(
    config: &BTreeMap<String, String>,
) -> Vec<Box<dyn ModuleChecker>> {
    // Start with the default set of checks.
    let mut checks: Vec<Box<dyn ModuleChecker>> = vec![];
    let checks_category = config.get("checks").map_or("default", |s| s.as_str());
    if checks_category == "strict" || checks_category == "experimental" {
        // Push strict checks to `checks`.
    }
    if checks_category == "experimental" {
        checks.push(Box::new(
            missing_doc_public_function::MissingDocPublicFunction,
        ));
        checks.push(Box::new(missing_doc_constant::MissingDocConstant));
        checks.push(Box::new(
            missing_doc_error_constant::MissingDocErrorConstant,
        ));
        checks.push(Box::new(error_constant_naming::ErrorConstantNaming));
        checks.push(Box::new(error_message_too_long::ErrorMessageTooLong));
        checks.push(Box::new(missing_doc_module::MissingDocModule));
        checks.push(Box::new(missing_doc_struct::MissingDocStruct));
        checks.push(Box::new(prefer_doc_comment::PreferDocComment));
    }
    checks
}

/// Returns true if the constant name matches an error constant pattern:
/// starts with `E_` or starts with `E` followed by an uppercase letter.
pub(crate) fn is_error_constant(name: &str) -> bool {
    name.starts_with("E_")
        || (name.starts_with('E') && name.len() > 1 && name.as_bytes()[1].is_ascii_uppercase())
}
