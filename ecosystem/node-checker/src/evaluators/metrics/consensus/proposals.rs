// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::{
    super::{
        common::{get_metric, GetMetricResult},
        types::{MetricsEvaluatorError, MetricsEvaluatorInput},
    },
    CATEGORY,
};
use crate::{
    configuration::EvaluatorArgs,
    evaluator::{EvaluationResult, Evaluator},
    evaluators::EvaluatorType,
};
use anyhow::Result;
use clap::Parser;
use log::debug;
use poem_openapi::Object as PoemObject;
use prometheus_parse::Scrape as PrometheusScrape;
use serde::{Deserialize, Serialize};

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
        let evaluation_on_missing_fn = || {
            self.build_evaluation_result(
                "Consensus proposals metric missing".to_string(),
                0,
                format!(
                    "The {} set of metrics from the target node is missing the proposals metric: {}",
                    metrics_round, PROPOSALS_METRIC
                )
            )
        };
        get_metric(metrics, PROPOSALS_METRIC, None, evaluation_on_missing_fn)
    }

    #[allow(clippy::comparison_chain)]
    fn build_evaluation(
        &self,
        previous_proposals_count: u64,
        latest_proposals_count: u64,
    ) -> EvaluationResult {
        if latest_proposals_count < previous_proposals_count {
            self.build_evaluation_result(
                "Proposals count went backwards!".to_string(),
                0,
                format!("Successfully pulled metrics from target node twice, but the second time the consensus proposals count went backwards (from {} to {})", previous_proposals_count, latest_proposals_count),
            )
        } else if latest_proposals_count == previous_proposals_count {
            self.build_evaluation_result(
                "Proposals count is not progressing".to_string(),
                50,
                "Successfully pulled metrics from target node twice, but the proposal count isn't progressing.".to_string(),
            )
        } else {
            self.build_evaluation_result(
                "Proposals count is increasing".to_string(),
                100,
                format!("Successfully pulled metrics from target node twice and saw that proposals count is increasing (from {} to {})", previous_proposals_count, latest_proposals_count),
            )
        }
    }
}

// See https://github.com/aptos-labs/aptos-core/pull/1450 for a discussion on
// how this evaluator can be improved.
#[async_trait::async_trait]
impl Evaluator for ConsensusProposalsEvaluator {
    type Input = MetricsEvaluatorInput;
    type Error = MetricsEvaluatorError;

    /// Assert that the proposals count is increasing on the target node.
    async fn evaluate(&self, input: &Self::Input) -> Result<Vec<EvaluationResult>, Self::Error> {
        let mut evaluation_results = vec![];

        // Get previous proposals count from the target node.
        let previous_proposals_count = self
            .get_proposals_count(&input.previous_target_metrics, "first")
            .unwrap(&mut evaluation_results);

        // Get the latest proposals count from the target node.
        let latest_proposals_count = self
            .get_proposals_count(&input.latest_target_metrics, "second")
            .unwrap(&mut evaluation_results);

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

    fn get_category_name() -> String {
        CATEGORY.to_string()
    }

    fn get_evaluator_name() -> String {
        "proposals".to_string()
    }

    fn from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<Self> {
        Ok(Self::new(evaluator_args.consensus_proposals_args.clone()))
    }

    fn evaluator_type_from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<EvaluatorType> {
        Ok(EvaluatorType::Metrics(Box::new(Self::from_evaluator_args(
            evaluator_args,
        )?)))
    }
}

#[cfg(test)]
mod test {
    use super::{super::super::parse_metrics, *};

    fn get_metric_strings(value: u64) -> Vec<String> {
        vec![
            format!("# TYPE {} counter", PROPOSALS_METRIC),
            format!("{} {}", PROPOSALS_METRIC, value),
        ]
    }

    async fn test_proposals_evaluator(
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

        let input = MetricsEvaluatorInput {
            previous_baseline_metrics: parse_metrics(vec![]).unwrap(),
            previous_target_metrics: parse_metrics(previous_target_metrics).unwrap(),
            latest_baseline_metrics: parse_metrics(vec![]).unwrap(),
            latest_target_metrics: parse_metrics(latest_target_metrics).unwrap(),
        };

        let evaluations = evaluator
            .evaluate(&input)
            .await
            .expect("Failed to evaluate metrics");

        let expected_evaluations_len =
            match previous_target_proposals.is_none() && latest_target_proposals.is_none() {
                true => 2,
                false => 1,
            };

        assert_eq!(evaluations.len(), expected_evaluations_len);
        assert_eq!(evaluations[0].score, expected_score);
    }

    #[tokio::test]
    async fn test_progressing() {
        test_proposals_evaluator(Some(500), Some(600), 100).await;
    }

    #[tokio::test]
    async fn test_not_progressing() {
        test_proposals_evaluator(Some(500), Some(500), 50).await;
    }

    #[tokio::test]
    async fn test_missing_metric() {
        test_proposals_evaluator(Some(500), None, 0).await;
    }

    #[tokio::test]
    async fn test_both_missing_metrics() {
        test_proposals_evaluator(None, None, 0).await;
    }
}
