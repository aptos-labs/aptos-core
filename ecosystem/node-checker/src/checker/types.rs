// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Object, Serialize)]
pub struct CheckResult {
    /// Name of the Checker that created the result.
    pub checker_name: String,

    /// Headline of the result, e.g. "Healthy!" or "Metrics missing!".
    pub headline: String,

    /// Score out of 100.
    pub score: u8,

    /// Explanation of the result.
    pub explanation: String,

    /// Links that might help the user fix a potential problem.
    pub links: Vec<String>,
}

impl CheckResult {
    pub fn new(checker_name: String, headline: String, score: u8, explanation: String) -> Self {
        Self {
            checker_name,
            headline,
            score,
            explanation,
            links: Vec::new(),
        }
    }

    pub fn links(mut self, links: Vec<String>) -> Self {
        self.links = links;
        self
    }
}

#[derive(Clone, Debug, Deserialize, Object, Serialize)]
pub struct CheckSummary {
    /// Results from all the Checkers NHC ran.
    pub check_results: Vec<CheckResult>,

    /// An aggeregated summary (method TBA).
    pub summary_score: u8,

    /// An overall explanation of the results.
    pub summary_explanation: String,
}

impl From<Vec<CheckResult>> for CheckSummary {
    // Very basic for now, we likely want a trait for this.
    fn from(check_results: Vec<CheckResult>) -> Self {
        let summary_score = match check_results.len() {
            0 => 100,
            len => (check_results.iter().map(|e| e.score as u32).sum::<u32>() / len as u32) as u8,
        };
        let summary_explanation = match summary_score {
            summary_score if summary_score > 95 => format!("{}: Awesome!", summary_score),
            summary_score if summary_score > 80 => format!("{}: Good!", summary_score),
            summary_score if summary_score > 50 => format!("{}: Getting there!", summary_score),
            wildcard => format!("{}: Improvement necessary", wildcard),
        };
        CheckSummary {
            check_results,
            summary_score,
            summary_explanation,
        }
    }
}
