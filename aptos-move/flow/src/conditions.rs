// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-condition verification status and the progress between two attempts.
//!
//! A prover run reports only what failed. Pairing those reports with the
//! conditions the specification actually declares turns a run into a status for
//! every obligation, and comparing two such reports says what an edit changed.
//!
//! Progress is deliberately not a score. A count of proved conditions rises
//! when obligations are deleted, so the comparison names conditions that
//! disappeared as prominently as conditions that started to verify.

use serde::Serialize;
use std::collections::BTreeMap;

/// Status of one declared specification condition after a verification attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConditionStatus {
    pub function: String,
    pub kind: String,
    pub file: String,
    pub line: usize,
    pub verified: bool,
    pub diagnostic: Option<String>,
}

impl ConditionStatus {
    /// Identity of a condition across attempts.
    ///
    /// Keyed by source position and kind rather than by text, so rewording a
    /// condition still counts as the same obligation.
    pub fn key(&self) -> (String, String, String, usize) {
        (
            self.function.clone(),
            self.kind.clone(),
            self.file.clone(),
            self.line,
        )
    }

    pub fn label(&self) -> String {
        format!(
            "{} {} at {}:{}",
            self.function, self.kind, self.file, self.line
        )
    }
}

/// What changed between two verification attempts.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ConditionDelta {
    pub verified_before: usize,
    pub verified_after: usize,
    pub failing_before: usize,
    pub failing_after: usize,
    pub newly_verified: Vec<String>,
    pub newly_failing: Vec<String>,
    pub removed: Vec<String>,
    pub added: Vec<String>,
    pub still_failing: Vec<String>,
}

impl ConditionDelta {
    pub fn between(previous: &[ConditionStatus], current: &[ConditionStatus]) -> Self {
        let before: BTreeMap<_, _> = previous.iter().map(|item| (item.key(), item)).collect();
        let after: BTreeMap<_, _> = current.iter().map(|item| (item.key(), item)).collect();
        let mut delta = ConditionDelta {
            verified_before: previous.iter().filter(|item| item.verified).count(),
            verified_after: current.iter().filter(|item| item.verified).count(),
            failing_before: previous.iter().filter(|item| !item.verified).count(),
            failing_after: current.iter().filter(|item| !item.verified).count(),
            ..Default::default()
        };
        for (key, item) in &after {
            match before.get(key) {
                None => delta.added.push(item.label()),
                Some(old) if !old.verified && item.verified => {
                    delta.newly_verified.push(item.label())
                },
                Some(old) if old.verified && !item.verified => {
                    delta.newly_failing.push(item.label())
                },
                Some(_) if !item.verified => delta.still_failing.push(item.label()),
                Some(_) => {},
            }
        }
        for (key, item) in &before {
            if !after.contains_key(key) {
                delta.removed.push(item.label());
            }
        }
        delta
    }

    /// Compact progress report. Only the lines that carry information appear.
    pub fn render(&self) -> String {
        let mut lines = vec![format!(
            "Progress: verified {} -> {}, failing {} -> {}",
            self.verified_before, self.verified_after, self.failing_before, self.failing_after
        )];
        for (title, items) in [
            ("Now verified", &self.newly_verified),
            ("Now failing", &self.newly_failing),
            ("Added", &self.added),
            // A disappeared obligation is reported as prominently as a proved
            // one: deleting a condition must not read as progress.
            ("No longer declared", &self.removed),
            ("Still failing", &self.still_failing),
        ] {
            for item in items.iter().take(5) {
                lines.push(format!("  {title}: {item}"));
            }
            if items.len() > 5 {
                lines.push(format!("  {title}: and {} more", items.len() - 5));
            }
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(kind: &str, line: usize, verified: bool) -> ConditionStatus {
        ConditionStatus {
            function: "m::f".to_string(),
            kind: kind.to_string(),
            file: "sources/m.move".to_string(),
            line,
            verified,
            diagnostic: None,
        }
    }

    #[test]
    fn a_repaired_condition_is_reported_as_newly_verified() {
        let delta = ConditionDelta::between(
            &[status("Ensures", 10, false), status("AbortsIf", 11, true)],
            &[status("Ensures", 10, true), status("AbortsIf", 11, true)],
        );
        assert_eq!(
            vec!["m::f Ensures at sources/m.move:10"],
            delta.newly_verified
        );
        assert!(delta.removed.is_empty());
        assert_eq!((1, 2), (delta.verified_before, delta.verified_after));
    }

    #[test]
    fn a_deleted_obligation_is_not_progress() {
        let delta = ConditionDelta::between(
            &[status("Ensures", 10, false), status("AbortsIf", 11, true)],
            &[status("AbortsIf", 11, true)],
        );
        assert_eq!(vec!["m::f Ensures at sources/m.move:10"], delta.removed);
        assert!(delta.newly_verified.is_empty());
        assert_eq!(0, delta.failing_after);
        assert!(delta.render().contains("No longer declared"));
    }

    #[test]
    fn a_regression_is_reported_even_while_the_total_improves() {
        let delta = ConditionDelta::between(
            &[status("Ensures", 10, true), status("AbortsIf", 11, false)],
            &[
                status("Ensures", 10, false),
                status("AbortsIf", 11, true),
                status("Modifies", 12, true),
            ],
        );
        assert_eq!(
            vec!["m::f Ensures at sources/m.move:10"],
            delta.newly_failing
        );
        assert_eq!(
            vec!["m::f AbortsIf at sources/m.move:11"],
            delta.newly_verified
        );
        assert_eq!(vec!["m::f Modifies at sources/m.move:12"], delta.added);
    }

    #[test]
    fn rewording_a_condition_keeps_its_identity() {
        let mut reworded = status("Ensures", 10, true);
        reworded.diagnostic = Some("irrelevant".to_string());
        let delta = ConditionDelta::between(&[status("Ensures", 10, false)], &[reworded]);
        assert_eq!(1, delta.newly_verified.len());
        assert!(delta.added.is_empty() && delta.removed.is_empty());
    }
}
