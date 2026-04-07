// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module (and its submodules) contain various model-AST-based lint checks.

mod aborting_overflow_checks;
mod almost_swapped;
mod assert_const;
mod blocks_in_conditions;
mod collapsible_if;
mod cyclomatic_complexity;
mod deprecated_usage;
mod empty_if;
mod equal_operands_in_bin_op;
mod known_to_abort;
mod needless_bool;
mod needless_deref_ref;
mod needless_ref_deref;
mod needless_ref_in_field_access;
mod needless_return;
mod needless_visibility;
mod nonminimal_bool;
mod self_assignment;
mod simpler_bool_expression;
mod simpler_numeric_expression;
mod unnecessary_boolean_identity_comparison;
mod unnecessary_cast;
mod unnecessary_numerical_extreme_comparison;
pub(crate) mod unused_common;
mod unused_constant;
mod unused_function;
mod unused_struct;
mod use_index_syntax;
mod use_receiver_style;
mod while_true;

use move_compiler_v2::external_checks::{
    ConstantChecker, ExpChecker, FunctionChecker, StructChecker,
};
use std::collections::BTreeMap;

/// Returns a default pipeline of "expression linters" to run.
/// The `config` parameter gates checkers by category. The `"checks"` key selects which
/// tier of lints to enable:
/// - `"default"`: curated checks that minimize false positives.
/// - `"strict"`: stricter checks (may produce more false positives); includes default.
/// - `"experimental"`: unstable checks; includes strict and default.
pub fn get_default_exp_linter_pipeline(
    config: &BTreeMap<String, String>,
) -> Vec<Box<dyn ExpChecker>> {
    // Start with the default set of checks.
    let mut checks: Vec<Box<dyn ExpChecker>> = vec![
        Box::<aborting_overflow_checks::AbortingOverflowChecks>::default(),
        Box::<almost_swapped::AlmostSwapped>::default(),
        Box::<assert_const::AssertConst>::default(),
        Box::<blocks_in_conditions::BlocksInConditions>::default(),
        Box::<collapsible_if::CollapsibleIf>::default(),
        Box::<empty_if::EmptyIf>::default(),
        Box::<equal_operands_in_bin_op::EqualOperandsInBinOp>::default(),
        Box::<unnecessary_cast::FindUnnecessaryCast>::default(),
        Box::<known_to_abort::KnownToAbort>::default(),
        Box::<needless_bool::NeedlessBool>::default(),
        Box::<needless_deref_ref::NeedlessDerefRef>::default(),
        Box::<needless_ref_deref::NeedlessRefDeref>::default(),
        Box::<needless_ref_in_field_access::NeedlessRefInFieldAccess>::default(),
        Box::<needless_return::NeedlessReturn>::default(),
        Box::<nonminimal_bool::NonminimalBool>::default(),
        Box::<self_assignment::SelfAssignment>::default(),
        Box::<simpler_bool_expression::SimplerBoolExpression>::default(),
        Box::<simpler_numeric_expression::SimplerNumericExpression>::default(),
        Box::<unnecessary_boolean_identity_comparison::UnnecessaryBooleanIdentityComparison>::default(),
        Box::<unnecessary_numerical_extreme_comparison::UnnecessaryNumericalExtremeComparison>::default(),
        Box::<while_true::WhileTrue>::default(),
    ];
    let checks_category = config.get("checks").map_or("default", |s| s.as_str());
    if checks_category == "strict" || checks_category == "experimental" {
        checks.push(Box::<deprecated_usage::DeprecatedUsage>::default());
        checks.push(Box::<use_index_syntax::UseIndexSyntax>::default());
        checks.push(Box::<use_receiver_style::UseReceiverStyle>::default());
    }
    if checks_category == "experimental" {
        checks.push(Box::<cyclomatic_complexity::CyclomaticComplexity>::default());
    }
    checks
}

/// Returns a default pipeline of constant linters.
/// The `config` parameter follows the same convention as in [`get_default_exp_linter_pipeline`].
pub fn get_default_constant_linter_pipeline(
    config: &BTreeMap<String, String>,
) -> Vec<Box<dyn ConstantChecker>> {
    let mut checks: Vec<Box<dyn ConstantChecker>> =
        vec![Box::<unused_constant::UnusedConstant>::default()];
    let checks_category = config.get("checks").map_or("default", |s| s.as_str());
    if checks_category == "strict" || checks_category == "experimental" {
        checks.push(Box::<deprecated_usage::DeprecatedUsageOfConstants>::default());
    }
    checks
}

/// Returns a default pipeline of struct linters.
/// The `config` parameter follows the same convention as in [`get_default_exp_linter_pipeline`].
pub fn get_default_struct_linter_pipeline(
    config: &BTreeMap<String, String>,
) -> Vec<Box<dyn StructChecker>> {
    let mut checks: Vec<Box<dyn StructChecker>> =
        vec![Box::<unused_struct::UnusedStruct>::default()];
    let checks_category = config.get("checks").map_or("default", |s| s.as_str());
    if checks_category == "strict" || checks_category == "experimental" {
        checks.push(Box::<deprecated_usage::DeprecatedUsageInFields>::default());
    }
    checks
}

/// Returns a default pipeline of function linters.
/// The `config` parameter follows the same convention as in [`get_default_exp_linter_pipeline`].
pub fn get_default_function_linter_pipeline(
    config: &BTreeMap<String, String>,
) -> Vec<Box<dyn FunctionChecker>> {
    let mut checks: Vec<Box<dyn FunctionChecker>> = vec![
        Box::<unused_function::UnusedFunction>::default(),
        Box::<needless_visibility::NeedlessVisibility>::default(),
    ];
    let checks_category = config.get("checks").map_or("default", |s| s.as_str());
    if checks_category == "strict" || checks_category == "experimental" {
        checks.push(Box::<deprecated_usage::DeprecatedUsageInSignatures>::default());
    }
    checks
}
