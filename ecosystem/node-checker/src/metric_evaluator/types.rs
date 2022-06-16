// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use poem_openapi::Object as PoemObject;

// TODO: Should I find a way to have typed actual + expected fields?
#[derive(Clone, Debug, PoemObject)]
pub struct EvaluationResult {
    /// Headline of the evaluation, e.g. "Healthy!" or "Metrics missing!".
    pub headline: String,

    /// Score out of 100.
    pub score: u8,

    /// Explanation of the evaluation.
    pub explanation: String,

    /// Name of the evaluator where the evaluation came from, e.g. state_sync.
    pub source: String,

    /// Links that might help the user fix a potential problem.
    pub links: Vec<String>,
}

#[derive(Clone, Debug, PoemObject)]
pub struct EvaluationSummary {
    /// All the evaluations we ran.
    pub evaluations: Vec<EvaluationResult>,

    /// An aggeregated summary (method TBA).
    pub summary_score: u8,

    /// An overall explanation of the results.
    pub summary_explanation: String,
}

impl From<Vec<EvaluationResult>> for EvaluationSummary {
    // Very basic for now, we likely want a trait for this.
    fn from(evaluations: Vec<EvaluationResult>) -> Self {
        let summary_score = match evaluations.len() {
            0 => 0,
            _ => evaluations.iter().map(|e| e.score).sum::<u8>() / evaluations.len() as u8,
        };
        let summary_explanation = match summary_score {
            summary_score if summary_score > 95 => format!("{}: Awesome!", summary_score),
            summary_score if summary_score > 80 => format!("{}: Good!", summary_score),
            summary_score if summary_score > 50 => format!("{}: Getting there!", summary_score),
            wildcard => format!("{}: Not good enough :(", wildcard),
        };
        EvaluationSummary {
            evaluations,
            summary_score,
            summary_explanation,
        }
    }
}
