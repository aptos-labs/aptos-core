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
mod mutable_view_function;
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
mod unsafe_friend_package_entry;
pub(crate) mod unused_common;
mod unused_constant;
mod unused_function;
mod unused_struct;
mod use_index_syntax;
mod use_receiver_style;
mod while_true;

use crate::{select_lints, LintSpec, LintTier};
use aborting_overflow_checks::AbortingOverflowChecks;
use almost_swapped::AlmostSwapped;
use assert_const::AssertConst;
use blocks_in_conditions::BlocksInConditions;
use collapsible_if::CollapsibleIf;
use cyclomatic_complexity::CyclomaticComplexity;
use deprecated_usage::{
    DeprecatedUsage, DeprecatedUsageInFields, DeprecatedUsageInSignatures,
    DeprecatedUsageOfConstants,
};
use empty_if::EmptyIf;
use equal_operands_in_bin_op::EqualOperandsInBinOp;
use known_to_abort::KnownToAbort;
use move_compiler_v2::external_checks::{
    ConstantChecker, ExpChecker, FunctionChecker, StructChecker,
};
use mutable_view_function::MutableViewFunction;
use needless_bool::NeedlessBool;
use needless_deref_ref::NeedlessDerefRef;
use needless_ref_deref::NeedlessRefDeref;
use needless_ref_in_field_access::NeedlessRefInFieldAccess;
use needless_return::NeedlessReturn;
use needless_visibility::NeedlessVisibility;
use nonminimal_bool::NonminimalBool;
use self_assignment::SelfAssignment;
use simpler_bool_expression::SimplerBoolExpression;
use simpler_numeric_expression::SimplerNumericExpression;
use unnecessary_boolean_identity_comparison::UnnecessaryBooleanIdentityComparison;
use unnecessary_cast::FindUnnecessaryCast;
use unnecessary_numerical_extreme_comparison::UnnecessaryNumericalExtremeComparison;
use unsafe_friend_package_entry::UnsafeFriendPackageEntry;
use unused_constant::UnusedConstant;
use unused_function::UnusedFunction;
use unused_struct::UnusedStruct;
use use_index_syntax::UseIndexSyntax;
use use_receiver_style::UseReceiverStyle;
use while_true::WhileTrue;

/// Registry of every expression-AST lint with its tier, unfiltered.
/// `select_lints` narrows this list according to a [`LintSpec`].
pub(crate) fn all_exp_lints() -> Vec<(LintTier, Box<dyn ExpChecker>)> {
    use LintTier::{Default, Experimental, Strict};
    vec![
        // ── default tier ──────────────────────────────────────────────
        (Default, Box::<AbortingOverflowChecks>::default()),
        (Default, Box::<AlmostSwapped>::default()),
        (Default, Box::<AssertConst>::default()),
        (Default, Box::<BlocksInConditions>::default()),
        (Default, Box::<CollapsibleIf>::default()),
        (Default, Box::<EmptyIf>::default()),
        (Default, Box::<EqualOperandsInBinOp>::default()),
        (Default, Box::<FindUnnecessaryCast>::default()),
        (Default, Box::<KnownToAbort>::default()),
        (Default, Box::<NeedlessBool>::default()),
        (Default, Box::<NeedlessDerefRef>::default()),
        (Default, Box::<NeedlessRefDeref>::default()),
        (Default, Box::<NeedlessRefInFieldAccess>::default()),
        (Default, Box::<NeedlessReturn>::default()),
        (Default, Box::<NonminimalBool>::default()),
        (Default, Box::<SelfAssignment>::default()),
        (Default, Box::<SimplerBoolExpression>::default()),
        (Default, Box::<SimplerNumericExpression>::default()),
        (
            Default,
            Box::<UnnecessaryBooleanIdentityComparison>::default(),
        ),
        (
            Default,
            Box::<UnnecessaryNumericalExtremeComparison>::default(),
        ),
        (Default, Box::<WhileTrue>::default()),
        // ── strict tier ───────────────────────────────────────────────
        (Strict, Box::<DeprecatedUsage>::default()),
        (Strict, Box::<UseIndexSyntax>::default()),
        (Strict, Box::<UseReceiverStyle>::default()),
        // ── experimental tier ─────────────────────────────────────────
        (Experimental, Box::<CyclomaticComplexity>::default()),
    ]
}

/// Registry of every constant-level lint with its tier, unfiltered.
pub(crate) fn all_constant_lints() -> Vec<(LintTier, Box<dyn ConstantChecker>)> {
    use LintTier::{Default, Strict};
    vec![
        // ── default tier ──────────────────────────────────────────────
        (Default, Box::<UnusedConstant>::default()),
        // ── strict tier ───────────────────────────────────────────────
        (Strict, Box::<DeprecatedUsageOfConstants>::default()),
    ]
}

/// Registry of every struct-level lint with its tier, unfiltered.
pub(crate) fn all_struct_lints() -> Vec<(LintTier, Box<dyn StructChecker>)> {
    use LintTier::{Default, Strict};
    vec![
        // ── default tier ──────────────────────────────────────────────
        (Default, Box::<UnusedStruct>::default()),
        // ── strict tier ───────────────────────────────────────────────
        (Strict, Box::<DeprecatedUsageInFields>::default()),
    ]
}

/// Registry of every function-level lint with its tier, unfiltered.
pub(crate) fn all_function_lints() -> Vec<(LintTier, Box<dyn FunctionChecker>)> {
    use LintTier::{Default, Strict};
    vec![
        // ── default tier ──────────────────────────────────────────────
        (Default, Box::new(MutableViewFunction::new())),
        (Default, Box::<UnusedFunction>::default()),
        (Default, Box::<NeedlessVisibility>::default()),
        (Default, Box::<UnsafeFriendPackageEntry>::default()),
        // ── strict tier ───────────────────────────────────────────────
        (Strict, Box::<DeprecatedUsageInSignatures>::default()),
    ]
}

/// Expression-AST checkers enabled by `spec`.
pub fn get_default_exp_linter_pipeline(spec: &LintSpec) -> Vec<Box<dyn ExpChecker>> {
    select_lints(spec, all_exp_lints(), |c| c.get_name())
}

/// Constant-level checkers enabled by `spec`.
pub fn get_default_constant_linter_pipeline(spec: &LintSpec) -> Vec<Box<dyn ConstantChecker>> {
    select_lints(spec, all_constant_lints(), |c| c.get_name())
}

/// Struct-level checkers enabled by `spec`.
pub fn get_default_struct_linter_pipeline(spec: &LintSpec) -> Vec<Box<dyn StructChecker>> {
    select_lints(spec, all_struct_lints(), |c| c.get_name())
}

/// Function-level checkers enabled by `spec`.
pub fn get_default_function_linter_pipeline(spec: &LintSpec) -> Vec<Box<dyn FunctionChecker>> {
    select_lints(spec, all_function_lints(), |c| c.get_name())
}
