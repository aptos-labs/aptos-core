// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod lint_spec;
mod model_ast_lints;
mod stackless_bytecode_lints;
mod utils;

use anyhow::{ensure, Result};
pub use lint_spec::{LintSpec, LintTier};
use move_compiler_v2::external_checks::{
    ConstantChecker, ExpChecker, ExternalChecks, FunctionChecker, StacklessBytecodeChecker,
    StructChecker,
};
use std::{collections::BTreeSet, sync::Arc};

/// Holds the collection of lint checks for Move that will run on a build.
pub struct MoveLintChecks {
    spec: LintSpec,
}

impl ExternalChecks for MoveLintChecks {
    fn get_exp_checkers(&self) -> Vec<Box<dyn ExpChecker>> {
        model_ast_lints::get_default_exp_linter_pipeline(&self.spec)
    }

    fn get_stackless_bytecode_checkers(&self) -> Vec<Box<dyn StacklessBytecodeChecker>> {
        stackless_bytecode_lints::get_default_linter_pipeline(&self.spec)
    }

    fn get_constant_checkers(&self) -> Vec<Box<dyn ConstantChecker>> {
        model_ast_lints::get_default_constant_linter_pipeline(&self.spec)
    }

    fn get_struct_checkers(&self) -> Vec<Box<dyn StructChecker>> {
        model_ast_lints::get_default_struct_linter_pipeline(&self.spec)
    }

    fn get_function_checkers(&self) -> Vec<Box<dyn FunctionChecker>> {
        model_ast_lints::get_default_function_linter_pipeline(&self.spec)
    }

    fn get_all_checker_names(&self) -> BTreeSet<String> {
        all_lints_by_tier().into_iter().map(|(_, n)| n).collect()
    }
}

impl MoveLintChecks {
    /// Make an instance of lint checks for Move, provided as `ExternalChecks`.
    /// Returns an error if `spec` references an unknown lint name in either
    /// `include` or `exclude`.
    pub fn make(spec: LintSpec) -> Result<Arc<dyn ExternalChecks>> {
        if !spec.include.is_empty() || !spec.exclude.is_empty() {
            let known: BTreeSet<String> = all_lints_by_tier().into_iter().map(|(_, n)| n).collect();
            for name in spec.include.iter().chain(spec.exclude.iter()) {
                ensure!(
                    known.contains(name),
                    "unknown lint check: `{}`. Run `aptos move lint --list-checks` to see all available lints",
                    name
                );
            }
        }
        Ok(Arc::new(MoveLintChecks { spec }))
    }
}

/// All known lints, with their tier, sorted by `(tier, name)` and deduplicated.
/// A single logical lint (e.g. `deprecated_usage`) may have multiple
/// implementations targeting different program elements; the user sees one
/// entry per unique name.
pub fn all_lints_by_tier() -> Vec<(LintTier, String)> {
    let expression_lints = model_ast_lints::all_exp_lints()
        .into_iter()
        .map(|(tier, checker)| (tier, checker.get_name()));
    let bytecode_lints = stackless_bytecode_lints::all_lints()
        .into_iter()
        .map(|(tier, checker)| (tier, checker.get_name()));
    let constant_lints = model_ast_lints::all_constant_lints()
        .into_iter()
        .map(|(tier, checker)| (tier, checker.get_name()));
    let struct_lints = model_ast_lints::all_struct_lints()
        .into_iter()
        .map(|(tier, checker)| (tier, checker.get_name()));
    let function_lints = model_ast_lints::all_function_lints()
        .into_iter()
        .map(|(tier, checker)| (tier, checker.get_name()));
    let mut out: Vec<(LintTier, String)> = expression_lints
        .chain(bytecode_lints)
        .chain(constant_lints)
        .chain(struct_lints)
        .chain(function_lints)
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    out.dedup_by(|a, b| a.1 == b.1);
    out
}

/// Select the boxed checkers from `all` that `spec` enables. Used by every
/// pipeline; `name_of` extracts the lint name from each boxed checker.
fn select_lints<C: ?Sized>(
    spec: &LintSpec,
    all: Vec<(LintTier, Box<C>)>,
    name_of: impl Fn(&C) -> String,
) -> Vec<Box<C>> {
    all.into_iter()
        .filter(|(tier, c)| spec.enables(*tier, &name_of(c)))
        .map(|(_, c)| c)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn make_rejects_unknown_include() {
        let mut spec = LintSpec::default();
        spec.include.insert("not_a_real_lint".to_string());
        let err = MoveLintChecks::make(spec).unwrap_err();
        assert!(
            err.to_string().contains("not_a_real_lint"),
            "error should mention the offending name: {err}"
        );
    }

    #[test]
    fn make_rejects_unknown_exclude() {
        let mut spec = LintSpec::default();
        spec.exclude.insert("misspelled_lint".to_string());
        let err = MoveLintChecks::make(spec).unwrap_err();
        assert!(err.to_string().contains("misspelled_lint"));
    }

    #[test]
    fn make_accepts_known_names() {
        let mut spec = LintSpec::default();
        spec.exclude.insert("needless_visibility".to_string());
        spec.include.insert("cyclomatic_complexity".to_string());
        assert!(MoveLintChecks::make(spec).is_ok());
    }

    #[test]
    fn all_lints_by_tier_is_nonempty_and_sorted() {
        let lints = all_lints_by_tier();
        assert!(!lints.is_empty());
        for win in lints.windows(2) {
            assert!(win[0] <= win[1], "not sorted: {:?} > {:?}", win[0], win[1]);
        }
    }

    /// A logical lint may have multiple implementations (e.g. `deprecated_usage`
    /// targets expressions, constants, struct fields, and function signatures
    /// via four separate impls). All impls sharing a name must be registered
    /// at the same tier — otherwise `all_lints_by_tier()`'s dedup-by-name
    /// silently picks one tier for `--list-checks`, while `spec.enables` runs
    /// each impl according to its own tier, yielding a partially-enabled lint.
    #[test]
    fn lint_names_have_unique_tier() {
        let mut by_name: BTreeMap<String, BTreeSet<LintTier>> = BTreeMap::new();
        for (tier, c) in model_ast_lints::all_exp_lints() {
            by_name.entry(c.get_name()).or_default().insert(tier);
        }
        for (tier, c) in stackless_bytecode_lints::all_lints() {
            by_name.entry(c.get_name()).or_default().insert(tier);
        }
        for (tier, c) in model_ast_lints::all_constant_lints() {
            by_name.entry(c.get_name()).or_default().insert(tier);
        }
        for (tier, c) in model_ast_lints::all_struct_lints() {
            by_name.entry(c.get_name()).or_default().insert(tier);
        }
        for (tier, c) in model_ast_lints::all_function_lints() {
            by_name.entry(c.get_name()).or_default().insert(tier);
        }
        for (name, tiers) in &by_name {
            assert_eq!(
                tiers.len(),
                1,
                "lint `{name}` is registered in multiple tiers: {tiers:?}"
            );
        }
    }
}
