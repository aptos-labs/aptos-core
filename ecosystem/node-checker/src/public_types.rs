// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::args::{DEFAULT_API_PORT, DEFAULT_METRICS_PORT, DEFAULT_NOISE_PORT};
use poem_openapi::{types::Example, Object as PoemObject};

#[derive(Clone, Debug, PoemObject)]
#[oai(example)]
pub struct NodeUrl {
    /// Target URL. This should include a scheme (e.g. http://). If there is
    /// no scheme, we will prepend http://.
    pub url: String,

    /// Metrics port.
    #[oai(default = "Self::default_metrics_port")]
    pub metrics_port: u16,

    /// API port.
    #[oai(default = "Self::default_api_port")]
    pub api_port: u16,

    /// Validator communication port.
    #[oai(default = "Self::default_noise_port")]
    pub noise_port: u16,
}

impl NodeUrl {
    fn default_metrics_port() -> u16 {
        DEFAULT_METRICS_PORT
    }

    fn default_api_port() -> u16 {
        DEFAULT_API_PORT
    }

    fn default_noise_port() -> u16 {
        DEFAULT_NOISE_PORT
    }
}

impl Example for NodeUrl {
    fn example() -> Self {
        Self {
            url: "mynode.mysite.com".to_string(),
            metrics_port: Self::default_metrics_port(),
            api_port: Self::default_api_port(),
            noise_port: Self::default_noise_port(),
        }
    }
}

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
