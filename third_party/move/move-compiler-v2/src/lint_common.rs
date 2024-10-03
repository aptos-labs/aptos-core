// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module contains common code useful for lint checkers at various stages
//! of the compilation pipeline.

use move_compiler::shared::known_attributes::LintAttribute;
use move_model::{ast::Attribute, model::GlobalEnv};
use std::str::FromStr;
use strum_macros::{Display, EnumString};

/// Enumeration of all the lint checks that can be performed.
///
/// With the use of `strum_macros::EnumString` and `strum_macros::Display`,
/// each of the variants can be serialized to and deserialized from a string.
/// The serialization follows "snake_case".
#[derive(Copy, Clone, Ord, Eq, PartialEq, PartialOrd, EnumString, Display)]
#[strum(serialize_all = "snake_case")]
pub enum LintChecker {
    AvoidCopyOnIdentityComparison,
    BlocksInConditions,
    NeedlessBool,
    NeedlessDerefRef,
    NeedlessMutableReference,
    NeedlessRefDeref,
    NeedlessRefInFieldAccess,
    SimplerNumericExpression,
    UnnecessaryBooleanIdentityComparison,
    UnnecessaryNumericalExtremeComparison,
    WhileTrue,
}

/// Extract all the lint checks to skip from the given attributes.
/// Also performs error-checking on any `LintAttribute::SKIP` attributes.
pub fn lint_skips_from_attributes(env: &GlobalEnv, attrs: &[Attribute]) -> Vec<LintChecker> {
    let lint_skip = env.symbol_pool().make(LintAttribute::SKIP);
    let skip_attr = attrs.iter().find(|attr| attr.name() == lint_skip);
    if let Some(skip_attr) = skip_attr {
        parse_lint_skip_attribute(env, skip_attr)
    } else {
        vec![]
    }
}

/// Extract all the lint checks to skip from `attr`.
/// Also performs error-checking on the LintAttribute::SKIP `attr`.
fn parse_lint_skip_attribute(env: &GlobalEnv, attr: &Attribute) -> Vec<LintChecker> {
    match attr {
        Attribute::Assign(id, ..) => {
            env.error(
                &env.get_node_loc(*id),
                &format!(
                    "expected `#[{}(...)]`, not an assigned value",
                    LintAttribute::SKIP
                ),
            );
            vec![]
        },
        Attribute::Apply(id, _, attrs) => {
            if attrs.is_empty() {
                env.error(
                    &env.get_node_loc(*id),
                    "no lint checks are specified to be skipped",
                );
            }
            attrs
            .iter()
            .filter_map(|lint_check| match lint_check {
                Attribute::Assign(id, ..) => {
                    env.error(
                        &env.get_node_loc(*id),
                        "did not expect an assigned value, expected only the names of the lint checks to be skipped",
                    );
                    None
                },
                Attribute::Apply(id, name, sub_attrs) => {
                    if !sub_attrs.is_empty() {
                        env.error(&env.get_node_loc(*id), "unexpected nested attributes");
                        None
                    } else {
                        let name = name.display(env.symbol_pool()).to_string();
                        let checker = LintChecker::from_str(&name).ok();
                        if checker.is_none() {
                            env.error(
                                &env.get_node_loc(*id),
                                &format!("unknown lint check: `{}`", name),
                            );
                        }
                        checker
                    }
                },
            })
            .collect()
        },
    }
}
