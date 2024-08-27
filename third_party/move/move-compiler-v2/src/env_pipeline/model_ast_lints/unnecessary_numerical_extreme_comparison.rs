// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for unnecessary
//! numerical comparisons with extreme values (min and max value that fits
//! within the numerical type).
//!
//! The recommendation depends on the actual comparison, e.g., for `x: u64`:
//!   `x < 0` => always false, rewrite code to remove this comparison
//!   `x >= 0` ==> always true, rewrite code to remove this comparison
//!   `x <= 0` ==> can be simplified to `x == 0`
//!   `x > 0` ==> can be clarified to `x != 0`
//! and similarly for comparing `x` with u64::MAX.

use crate::{env_pipeline::model_ast_lints::ExpressionLinter, lint_common::LintChecker};
use move_model::{
    ast::{ExpData, Operation, Value},
    model::GlobalEnv,
    ty::Type,
};
use num::BigInt;
use std::fmt;

#[derive(Default)]
pub struct UnnecessaryNumericalExtremeComparison;

impl ExpressionLinter for UnnecessaryNumericalExtremeComparison {
    fn get_lint_checker(&self) -> LintChecker {
        LintChecker::UnnecessaryNumericalExtremeComparison
    }

    fn visit_expr_pre(&mut self, env: &GlobalEnv, expr: &ExpData) {
        use ExpData::Call;
        use Operation::*;
        // Let's narrow down to the comparison operators we are interested in.
        if let Call(id, cmp @ (Le | Ge | Lt | Gt), args) = expr {
            debug_assert!(
                args.len() == 2,
                "there should be exactly two arguments for comparison operators"
            );
            let (lhs, rhs) = (args[0].as_ref(), args[1].as_ref());
            // Types on both sides of the comparison must be the same, so let's just
            // get the type of the left-hand side.
            let ty = env.get_node_type(lhs.node_id());
            if let Some(result) = Self::check_comparisons_with_extremes(lhs, cmp, rhs, &ty) {
                self.warning(env, &env.get_node_loc(*id), &result.to_string());
            }
        }
    }
}

/// Recommendation based on looking at the comparison expression.
enum ComparisonResult {
    AlwaysTrue,
    AlwaysFalse,
    UseEqInstead,
    UseNeqInstead,
}

impl fmt::Display for ComparisonResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::AlwaysTrue => write!(f, "Comparison is always true, consider rewriting the code to remove the redundant comparison"),
            Self::AlwaysFalse => write!(f, "Comparison is always false, consider rewriting the code to remove the redundant comparison"),
            Self::UseEqInstead => write!(f, "Comparison can be simplified to use `==` instead"),
            Self::UseNeqInstead => write!(f, "Comparison can be clarified to use `!=` instead"),
        }
    }
}

impl UnnecessaryNumericalExtremeComparison {
    /// Check if the expression `lhs cmp rhs` is a comparison with an extreme value
    /// for the numerical type `ty`.
    /// Returns `None` if the comparison is not one of the patterns being checked.
    /// Else returns the recommendation based on the comparison.
    fn check_comparisons_with_extremes(
        lhs: &ExpData,
        cmp: &Operation,
        rhs: &ExpData,
        ty: &Type,
    ) -> Option<ComparisonResult> {
        use ComparisonResult::*;
        use ExpData::Value as ExpValue;
        use Operation::*;
        use Value::Number;
        if !ty.is_number() {
            return None;
        }
        let Type::Primitive(ty) = ty else {
            unreachable!("number must be primitive")
        };
        let max = ty.get_max_value()?;
        let zero = BigInt::from(0);
        match (lhs, cmp, rhs) {
            (_, Lt, ExpValue(_, Number(n))) | (ExpValue(_, Number(n)), Gt, _) if n == &zero => {
                // exp < 0 || 0 > exp
                Some(AlwaysFalse)
            },
            (_, Ge, ExpValue(_, Number(n))) | (ExpValue(_, Number(n)), Le, _) if n == &zero => {
                // exp >= 0 || 0 <= exp
                Some(AlwaysTrue)
            },
            (_, Le, ExpValue(_, Number(n))) | (ExpValue(_, Number(n)), Ge, _) if n == &zero => {
                // exp <= 0 || 0 >= exp
                Some(UseEqInstead)
            },
            (_, Gt, ExpValue(_, Number(n))) | (ExpValue(_, Number(n)), Lt, _) if n == &zero => {
                // exp > 0 || 0 < exp
                Some(UseNeqInstead)
            },
            (_, Gt, ExpValue(_, Number(n))) | (ExpValue(_, Number(n)), Lt, _) if *n == max => {
                // exp > max || max < exp
                Some(AlwaysFalse)
            },
            (_, Le, ExpValue(_, Number(n))) | (ExpValue(_, Number(n)), Ge, _) if *n == max => {
                // exp <= max || max >= exp
                Some(AlwaysTrue)
            },
            (_, Ge, ExpValue(_, Number(n))) | (ExpValue(_, Number(n)), Le, _) if *n == max => {
                // exp >= max || max <= exp
                Some(UseEqInstead)
            },
            (_, Lt, ExpValue(_, Number(n))) | (ExpValue(_, Number(n)), Gt, _) if *n == max => {
                // exp < max || max > exp
                Some(UseNeqInstead)
            },
            _ => None,
        }
    }
}
