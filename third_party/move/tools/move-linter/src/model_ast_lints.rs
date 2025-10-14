// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module (and its submodules) contain various model-AST-based lint checks.

mod aborting_overflow_checks;
mod almost_swapped;
mod assert_const;
mod blocks_in_conditions;
mod cyclomatic_complexity;
mod empty_if;
mod equal_operands_in_bin_op;
mod find_unnecessary_casts;
mod known_to_abort;
mod needless_bool;
mod needless_deref_ref;
mod needless_ref_deref;
mod needless_ref_in_field_access;
mod needless_return;
mod nonminimal_bool;
mod self_assignment;
mod simpler_bool_expression;
mod simpler_numeric_expression;
mod unnecessary_boolean_identity_comparison;
mod unnecessary_numerical_extreme_comparison;
mod while_true;

use move_compiler_v2::external_checks::ExpChecker;
use std::collections::BTreeMap;

/// Returns a default pipeline of "expression linters" to run.
pub fn get_default_linter_pipeline(config: &BTreeMap<String, String>) -> Vec<Box<dyn ExpChecker>> {
    // Start with the default set of checks.
    let mut checks: Vec<Box<dyn ExpChecker>> = vec![
        Box::<aborting_overflow_checks::AbortingOverflowChecks>::default(),
        Box::<almost_swapped::AlmostSwapped>::default(),
        Box::<assert_const::AssertConst>::default(),
        Box::<blocks_in_conditions::BlocksInConditions>::default(),
        Box::<empty_if::EmptyIf>::default(),
        Box::<equal_operands_in_bin_op::EqualOperandsInBinOp>::default(),
        Box::<find_unnecessary_casts::FindUnnecessaryCasts>::default(),
        Box::<known_to_abort::KnownToAbort>::default(),
        Box::<needless_bool::NeedlessBool>::default(),
        Box::<needless_return::NeedlessReturn>::default(),
        Box::<needless_deref_ref::NeedlessDerefRef>::default(),
        Box::<needless_ref_deref::NeedlessRefDeref>::default(),
        Box::<needless_ref_in_field_access::NeedlessRefInFieldAccess>::default(),
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
        // Push strict checks to `checks`.
    }
    if checks_category == "experimental" {
        // Push experimental checks to `checks`.
        checks.push(Box::<cyclomatic_complexity::CyclomaticComplexity>::default());
    }
    checks
}
