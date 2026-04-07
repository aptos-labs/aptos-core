// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lint check for unused constants.

use super::unused_common::SHARED_SUPPRESSION_ATTRS;
use move_compiler_v2::external_checks::ConstantChecker;
use move_model::{
    ast::Attribute,
    model::{GlobalEnv, Loc, NamedConstantEnv},
    ty::Type,
};

const CHECKER_NAME: &str = "unused_constant";

#[derive(Default)]
pub struct UnusedConstant;

impl ConstantChecker for UnusedConstant {
    fn get_name(&self) -> String {
        CHECKER_NAME.to_string()
    }

    fn check_constant(&self, const_env: &NamedConstantEnv) {
        if should_warn_unused_constant(const_env) {
            let env = const_env.module_env.env;
            let msg = format!(
                "constant `{}` is unused",
                const_env.get_name().display(env.symbol_pool()),
            );
            self.report(env, &const_env.get_loc(), &msg);
        }
    }

    fn report(&self, env: &GlobalEnv, loc: &Loc, msg: &str) {
        env.lint_diag_with_notes(loc, msg, vec![format!(
            "Remove it, or suppress this warning with `#[test_only]` (if for test-only) \
             or `#[lint::skip({})]` (if for spec-only or otherwise needed).",
            CHECKER_NAME
        )]);
    }
}

/// Returns true if constant should be warned as unused.
fn should_warn_unused_constant(const_env: &NamedConstantEnv) -> bool {
    let env = const_env.module_env.env;

    let is_suppression_attr = |attr: &Attribute| {
        SHARED_SUPPRESSION_ATTRS
            .iter()
            .any(|&s| attr.name() == env.symbol_pool().make(s))
    };

    if const_env.is_test_or_verify_only()
        || const_env.has_attribute(is_suppression_attr)
        || !const_env.get_users().is_empty()
        || is_error_code(const_env)
    {
        return false;
    }

    true
}

/// Check if a constant is an error code. Error code constants are named with an
/// `E` prefix and must be `u64`.
fn is_error_code(const_env: &NamedConstantEnv) -> bool {
    let env = const_env.module_env.env;
    let name = env.symbol_pool().string(const_env.get_name());
    name.starts_with("E")
        && matches!(
            const_env.get_type(),
            Type::Primitive(move_model::ty::PrimitiveType::U64)
        )
}
