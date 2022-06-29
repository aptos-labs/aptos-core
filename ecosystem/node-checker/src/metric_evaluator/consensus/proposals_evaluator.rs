// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::{
    super::{
        common::{get_metric, GetMetricResult},
        MetricsEvaluator, MetricsEvaluatorError,
    },
    CONSENSUS_EVALUATOR_SOURCE,
};
use crate::evaluator::EvaluationResult;
use anyhow::Result;
use clap::Parser;
use log::debug;
use poem_openapi::Object as PoemObject;
use prometheus_parse::Scrape as PrometheusScrape;
use serde::{Deserialize, Serialize};

pub const CONSENSUS_PROPOSALS_EVALUATOR_NAME: &str =
    const_format::concatcp!(CONSENSUS_EVALUATOR_SOURCE, "_", "proposals");

// TODO: When we have it, switch to using a crate that unifies metric names.
// As it is now, this metric name could change and we'd never catch it here
// at compile time.
const PROPOSALS_METRIC: &str = "aptos_consensus_proposals_count";

#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
pub struct ConsensusProposalsEvaluatorArgs {}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ConsensusProposalsEvaluator {
    args: ConsensusProposalsEvaluatorArgs,
}

impl ConsensusProposalsEvaluator {
    pub fn new(args: ConsensusProposalsEvaluatorArgs) -> Self {
        Self { args }
    }

    fn get_proposals_count(
        &self,
        metrics: &PrometheusScrape,
        metrics_round: &str,
    ) -> GetMetricResult {
        let evaluation_on_missing_fn = || EvaluationResult {
            headline: "Consensus proposals metric missing".to_string(),
            score: 0,
            explanation: format!(
                "The {} set of metrics from the target node is missing the proposals metric: {}",
                metrics_round, PROPOSALS_METRIC
            ),
            source: CONSENSUS_EVALUATOR_SOURCE.to_string(),
            links: vec![],
        };
        get_metric(metrics, PROPOSALS_METRIC, None, evaluation_on_missing_fn)
    }

    fn build_evaluation(
        &self,
        previous_proposals_count: u64,
        latest_proposals_count: u64,
    ) -> EvaluationResult {
        // We convert to i64 to avoid potential overflow if somehow the count somehow went backwards.
        let progress = latest_proposals_count as i64 - previous_proposals_count as i64;
        match progress {
            progress if (progress == 0) => {
                EvaluationResult {
                    headline: "Proposals count is not progressing".to_string(),
                    score: 50,
                    explanation: "Successfully pulled metrics from target node twice, but the proposal count isn't progressing.".to_string(),
                    source: CONSENSUS_EVALUATOR_SOURCE.to_string(),
                    links: vec![],
              }
            }
            progress if (progress < 0) => {
                EvaluationResult {
                    headline: "Proposals count went backwards!".to_string(),
                    score: 0,
                    explanation: "Successfully pulled metrics from target node twice, but the second time the proposals count went backwards!".to_string(),
                    source: CONSENSUS_EVALUATOR_SOURCE.to_string(),
                    links: vec![],
                }
            }
            _wildcard => {
                EvaluationResult {
                    headline: "Proposals count is increasing".to_string(),
                    score: 100,
                    explanation: "Successfully pulled metrics from target node twice and saw that proposals count is increasing".to_string(),
                    source: CONSENSUS_EVALUATOR_SOURCE.to_string(),
                    links: vec![],
                }
            }
        }
    }
}

impl MetricsEvaluator for ConsensusProposalsEvaluator {
    /// Assert that the proposals count is increasing on the target node.
    fn evaluate_metrics(
        &self,
        _previous_baseline_metrics: &PrometheusScrape,
        previous_target_metrics: &PrometheusScrape,
        _latest_baseline_metrics: &PrometheusScrape,
        latest_target_metrics: &PrometheusScrape,
    ) -> Result<Vec<EvaluationResult>, MetricsEvaluatorError> {
        let mut evaluation_results = vec![];

        // Get previous proposals count from the target node.
        let previous_proposals_count =
            match self.get_proposals_count(previous_target_metrics, "first") {
                GetMetricResult::Present(metric) => Some(metric),
                GetMetricResult::Missing(evaluation_result) => {
                    evaluation_results.push(evaluation_result);
                    None
                }
            };

        // Get the latest proposals count from the target node.
        let latest_proposals_count = match self.get_proposals_count(latest_target_metrics, "second")
        {
            GetMetricResult::Present(metric) => Some(metric),
            GetMetricResult::Missing(evaluation_result) => {
                evaluation_results.push(evaluation_result);
                None
            }
        };

        match (previous_proposals_count, latest_proposals_count) {
            (Some(previous), Some(latest)) => {
                evaluation_results.push(self.build_evaluation(previous, latest));
            }
            _ => {
                debug!(
                    "Not evaluating proposals count because we're missing metrics from the target"
                );
            }
        };

        Ok(evaluation_results)
    }

    fn get_name(&self) -> String {
        CONSENSUS_PROPOSALS_EVALUATOR_NAME.to_string()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::metric_evaluator::common::parse_metrics;

    fn get_metric_strings(value: u64) -> Vec<String> {
        vec![
            format!("# TYPE {} counter", PROPOSALS_METRIC),
            format!("{} {}", PROPOSALS_METRIC, value),
        ]
    }

    fn test_proposals_evaluator(
        previous_target_proposals: Option<u64>,
        latest_target_proposals: Option<u64>,
        expected_score: u8,
    ) {
        let previous_target_metrics = match previous_target_proposals {
            Some(v) => get_metric_strings(v),
            None => vec![],
        };

        let latest_target_metrics = match latest_target_proposals {
            Some(v) => get_metric_strings(v),
            None => vec![],
        };

        let evaluator = ConsensusProposalsEvaluator::new(ConsensusProposalsEvaluatorArgs {});
        let evaluations = evaluator
            .evaluate_metrics(
                &parse_metrics(vec![]).unwrap(),
                &parse_metrics(previous_target_metrics).unwrap(),
                &parse_metrics(vec![]).unwrap(),
                &parse_metrics(latest_target_metrics).unwrap(),
            )
            .expect("Failed to evaluate metrics");

        let expected_evaluations_len =
            match previous_target_proposals.is_none() && latest_target_proposals.is_none() {
                true => 2,
                false => 1,
            };

        assert_eq!(evaluations.len(), expected_evaluations_len);
        assert_eq!(evaluations[0].score, expected_score);
    }

    #[test]
    fn test_progressing() {
        test_proposals_evaluator(Some(500), Some(600), 100);
    }

    #[test]
    fn test_not_progressing() {
        test_proposals_evaluator(Some(500), Some(500), 50);
    }

    #[test]
    fn test_missing_metric() {
        test_proposals_evaluator(Some(500), None, 0);
    }

    #[test]
    fn test_both_missing_metrics() {
        test_proposals_evaluator(None, None, 0);
    }
}
