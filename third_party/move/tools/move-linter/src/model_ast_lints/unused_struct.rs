// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lint check for unused private structs/enums.

use super::unused_common::SHARED_SUPPRESSION_ATTRS;
use move_binary_format::file_format::Visibility;
use move_compiler_v2::external_checks::StructChecker;
use move_model::{
    ast::Attribute,
    model::{GlobalEnv, Loc, StructEnv},
};

const CHECKER_NAME: &str = "unused_struct";

/// Additional attribute names that suppress unused warnings for structs only.
/// - `resource_group`: Empty marker structs used by VM for storage optimization.
/// - `resource_group_member`: Structs belonging to a resource group, used by VM verifier.
const STRUCT_ONLY_SUPPRESSION_ATTRS: &[&str] = &["resource_group", "resource_group_member"];

#[derive(Default)]
pub struct UnusedStruct;

impl StructChecker for UnusedStruct {
    fn get_name(&self) -> String {
        CHECKER_NAME.to_string()
    }

    fn check_struct(&self, struct_env: &StructEnv) {
        if should_warn_unused_struct(struct_env) {
            let entity_type = if struct_env.has_variants() {
                "enum"
            } else {
                "struct"
            };
            let msg = format!("{} `{}` is unused", entity_type, struct_env.get_name_str());
            self.report(struct_env.module_env.env, &struct_env.get_loc(), &msg);
        }
    }

    fn report(&self, env: &GlobalEnv, loc: &Loc, msg: &str) {
        env.lint_diag_with_notes(loc, msg, vec![format!(
            "Remove it (if not published), or suppress this warning with `#[test_only]` (if for test-only) \
             or `#[lint::skip({})]` (if for spec-only or otherwise needed).",
            CHECKER_NAME
        )]);
    }
}

/// Returns true if struct should be warned as unused.
fn should_warn_unused_struct(struct_env: &StructEnv) -> bool {
    let env = struct_env.module_env.env;

    let is_suppression_attr = |attr: &Attribute| {
        SHARED_SUPPRESSION_ATTRS
            .iter()
            .chain(STRUCT_ONLY_SUPPRESSION_ATTRS.iter())
            .any(|&s| attr.name() == env.symbol_pool().make(s))
    };

    if struct_env.get_visibility() != Visibility::Private
        || struct_env.is_ghost_memory()
        || struct_env.is_test_or_verify_only()
        || struct_env.has_attribute(is_suppression_attr)
        || !struct_env.get_users().is_empty()
    {
        return false;
    }

    true
}
