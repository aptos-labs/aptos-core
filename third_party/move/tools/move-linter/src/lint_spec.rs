// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! User-facing specification of which lint checks to run.

use anyhow::{bail, ensure, Result};
use std::{
    collections::BTreeSet,
    fmt::{self, Display, Formatter},
    str::FromStr,
};

/// Tier a lint belongs to.
///
/// Tiers are currently linearly ordered: `Default < Strict < Experimental`.
/// Selecting tier `T` transitively enables every tier `<= T`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LintTier {
    Default,
    Strict,
    Experimental,
}

impl Display for LintTier {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            LintTier::Default => "default",
            LintTier::Strict => "strict",
            LintTier::Experimental => "experimental",
        };
        // Use `pad` (not `write_str`) so callers can apply `{:<14}` etc.
        f.pad(s)
    }
}

impl FromStr for LintTier {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "default" => Ok(LintTier::Default),
            "strict" => Ok(LintTier::Strict),
            "experimental" => Ok(LintTier::Experimental),
            other => bail!(
                "unknown lint tier `{other}` (expected `default`, `strict`, or `experimental`)"
            ),
        }
    }
}

/// User-supplied selection of which lints to run.
///
/// `LintSpec::default()` selects only the `Default` tier — same behavior as
/// `aptos move lint` invoked without `--checks`.
#[derive(Clone, Debug)]
pub struct LintSpec {
    /// Highest tier selected. Tiers are cumulative: `Some(Strict)` enables
    /// Default+Strict; `None` enables no tier.
    pub max_tier: Option<LintTier>,
    /// Lints (by name) to additionally enable beyond `max_tier`.
    pub include: BTreeSet<String>,
    /// Lints (by name) to disable. Exclusion always wins over both `max_tier`
    /// and `include`.
    pub exclude: BTreeSet<String>,
}

impl Default for LintSpec {
    fn default() -> Self {
        Self {
            max_tier: Some(LintTier::Default),
            include: BTreeSet::new(),
            exclude: BTreeSet::new(),
        }
    }
}

impl LintSpec {
    /// Spec that enables nothing. Useful as a neutral starting point for a builder.
    pub fn empty() -> Self {
        Self {
            max_tier: None,
            include: BTreeSet::new(),
            exclude: BTreeSet::new(),
        }
    }

    /// True iff a lint of `tier` named `name` should run under this spec.
    pub fn enables(&self, tier: LintTier, name: &str) -> bool {
        if self.exclude.contains(name) {
            return false;
        }
        if self.include.contains(name) {
            return true;
        }
        self.max_tier.is_some_and(|max| tier <= max)
    }

    /// Parse a comma-separated spec like `"strict,-needless_visibility,cyclomatic_complexity"`.
    ///
    /// Each token may be:
    /// - a category: `default`, `strict`, or `experimental`,
    /// - a lint name to include,
    /// - or `-<name>` to exclude a lint.
    ///
    /// Whitespace around commas is tolerated. When the spec contains no
    /// positive selection (no tier and no included lint name), the default
    /// tier is implied — so empty input, `--checks=""`, and exclude-only
    /// inputs like `--checks=-needless_visibility` all behave as "default
    /// tier (minus any exclusions)".
    pub fn parse(s: &str) -> Result<Self> {
        let mut spec = Self::empty();
        for raw in s.split(',') {
            let tok = raw.trim();
            if tok.is_empty() {
                continue;
            }
            if let Some(rest) = tok.strip_prefix('-') {
                let name = rest.trim();
                ensure!(
                    !name.is_empty(),
                    "expected lint name after '-' in lint spec"
                );
                spec.exclude.insert(name.to_string());
            } else if tok.eq_ignore_ascii_case("all") {
                // `all` resolves to whatever the most-permissive tier currently
                // is. If a tier is added above `Experimental` later, update
                // this branch to point at the new top.
                spec.max_tier = spec.max_tier.max(Some(LintTier::Experimental));
            } else if let Ok(tier) = tok.parse::<LintTier>() {
                spec.max_tier = spec.max_tier.max(Some(tier));
            } else {
                spec.include.insert(tok.to_string());
            }
        }
        if spec.max_tier.is_none() && spec.include.is_empty() {
            spec.max_tier = Some(LintTier::Default);
        }
        Ok(spec)
    }
}

impl FromStr for LintSpec {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_yields_default_tier() {
        let s = LintSpec::parse("").unwrap();
        assert_eq!(s.max_tier, Some(LintTier::Default));
        assert!(s.include.is_empty());
        assert!(s.exclude.is_empty());
    }

    #[test]
    fn parse_categories() {
        assert_eq!(
            LintSpec::parse("strict").unwrap().max_tier,
            Some(LintTier::Strict)
        );
        assert_eq!(
            LintSpec::parse("experimental").unwrap().max_tier,
            Some(LintTier::Experimental)
        );
    }

    #[test]
    fn parse_compound_spec() {
        let s = LintSpec::parse("strict,-needless_visibility,cyclomatic_complexity").unwrap();
        assert_eq!(s.max_tier, Some(LintTier::Strict));
        assert_eq!(
            s.include,
            BTreeSet::from(["cyclomatic_complexity".to_string()])
        );
        assert_eq!(
            s.exclude,
            BTreeSet::from(["needless_visibility".to_string()])
        );
    }

    #[test]
    fn parse_only_a_single_lint() {
        let s = LintSpec::parse("use_receiver_style").unwrap();
        assert_eq!(s.max_tier, None);
        assert_eq!(
            s.include,
            BTreeSet::from(["use_receiver_style".to_string()])
        );
    }

    #[test]
    fn parse_tolerates_whitespace_and_dedupes() {
        let s = LintSpec::parse(" strict , -needless_visibility , strict ").unwrap();
        assert_eq!(s.max_tier, Some(LintTier::Strict));
        assert_eq!(
            s.exclude,
            BTreeSet::from(["needless_visibility".to_string()])
        );
    }

    #[test]
    fn parse_picks_highest_tier() {
        let s = LintSpec::parse("default,strict").unwrap();
        assert_eq!(s.max_tier, Some(LintTier::Strict));
    }

    #[test]
    fn parse_dash_without_name_errors() {
        assert!(LintSpec::parse("-").is_err());
        assert!(LintSpec::parse("strict,-").is_err());
    }

    #[test]
    fn enables_excluded_lint_never_runs() {
        let mut s = LintSpec::parse("strict").unwrap();
        s.exclude.insert("needless_visibility".to_string());
        assert!(!s.enables(LintTier::Default, "needless_visibility"));
        assert!(s.enables(LintTier::Default, "needless_bool"));
    }

    #[test]
    fn enables_include_overrides_missing_tier() {
        let s = LintSpec::parse("use_receiver_style").unwrap();
        assert!(s.enables(LintTier::Strict, "use_receiver_style"));
        assert!(!s.enables(LintTier::Strict, "deprecated_usage"));
    }

    #[test]
    fn enables_tiers_are_cumulative() {
        let s = LintSpec::parse("experimental").unwrap();
        assert!(s.enables(LintTier::Default, "needless_bool"));
        assert!(s.enables(LintTier::Strict, "use_receiver_style"));
        assert!(s.enables(LintTier::Experimental, "cyclomatic_complexity"));

        let s = LintSpec::parse("strict").unwrap();
        assert!(s.enables(LintTier::Default, "needless_bool"));
        assert!(s.enables(LintTier::Strict, "use_receiver_style"));
        assert!(!s.enables(LintTier::Experimental, "cyclomatic_complexity"));
    }

    #[test]
    fn enables_empty_spec_disables_everything() {
        let s = LintSpec::empty();
        assert!(!s.enables(LintTier::Default, "needless_bool"));
        assert!(!s.enables(LintTier::Strict, "use_receiver_style"));
    }

    #[test]
    fn tier_display_roundtrips_with_fromstr() {
        for tier in [LintTier::Default, LintTier::Strict, LintTier::Experimental] {
            assert_eq!(tier.to_string().parse::<LintTier>().unwrap(), tier);
        }
    }

    #[test]
    fn parse_exclude_only_falls_back_to_default_tier() {
        // Without the fallback, "-needless_visibility" would silently disable
        // every lint — see the regression note on `parse`'s doc comment.
        let s = LintSpec::parse("-needless_visibility").unwrap();
        assert_eq!(s.max_tier, Some(LintTier::Default));
        assert_eq!(
            s.exclude,
            BTreeSet::from(["needless_visibility".to_string()])
        );
        assert!(s.include.is_empty());
        assert!(s.enables(LintTier::Default, "needless_bool"));
        assert!(!s.enables(LintTier::Default, "needless_visibility"));
    }

    #[test]
    fn parse_explicit_include_does_not_pull_in_default_tier() {
        // Bare lint name with no tier still means "only that lint".
        let s = LintSpec::parse("use_receiver_style").unwrap();
        assert_eq!(s.max_tier, None);
        let s = LintSpec::parse("use_receiver_style,-needless_visibility").unwrap();
        assert_eq!(s.max_tier, None);
    }

    #[test]
    fn parse_all_aliases_to_top_tier() {
        let s = LintSpec::parse("all").unwrap();
        assert_eq!(s.max_tier, Some(LintTier::Experimental));
        // `all` and `experimental` are interchangeable on input.
        assert_eq!(
            LintSpec::parse("all").unwrap().max_tier,
            LintSpec::parse("experimental").unwrap().max_tier
        );
        // `all` is sugar for the user-input layer; it is NOT a tier name.
        assert!("all".parse::<LintTier>().is_err());
    }
}
