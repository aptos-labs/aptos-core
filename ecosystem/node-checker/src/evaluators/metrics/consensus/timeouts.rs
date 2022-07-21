// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/// This evaluator is only valuable in certain contexts. For example, this is
/// not a useful evaluator for node registration for the AITs, since each node
/// is running in their own isolated network, where no consensus is occurring.
/// This is useful for the AIT itself though, where the nodes are participating
/// in a real network.
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
const METRIC: &str = "aptos_consensus_timeout_count";

#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
pub struct ConsensusTimeoutsEvaluatorArgs {
    /// The amount by which timeouts are allowed to increase between each
    /// round of metrics collection.
    #[clap(long, default_value_t = 0)]
    pub allowed_consensus_timeouts: u64,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ConsensusTimeoutsEvaluator {
    args: ConsensusTimeoutsEvaluatorArgs,
}

impl ConsensusTimeoutsEvaluator {
    pub fn new(args: ConsensusTimeoutsEvaluatorArgs) -> Self {
        Self { args }
    }

    fn get_consensus_timeouts(
        &self,
        metrics: &PrometheusScrape,
        metrics_round: &str,
    ) -> GetMetricResult {
        let evaluation_on_missing_fn = || {
            self.build_evaluation_result(
                "Consensus timeouts metric missing".to_string(),
                0,
                format!(
                    "The {} set of metrics from the target node is missing the metric: {}",
                    metrics_round, METRIC
                ),
            )
        };
        get_metric(metrics, METRIC, None, evaluation_on_missing_fn)
    }

    #[allow(clippy::comparison_chain)]
    fn build_evaluation(
        &self,
        previous_timeouts_count: u64,
        latest_timeouts_count: u64,
    ) -> EvaluationResult {
        if latest_timeouts_count > previous_timeouts_count + self.args.allowed_consensus_timeouts {
            self.build_evaluation_result(
                "Consensus timeouts metric increased".to_string(),
                50,
                format!(
                    "The consensus timeouts count increased from {} to {} between metrics rounds more than the allowed amount ({})",
                    previous_timeouts_count, latest_timeouts_count, self.args.allowed_consensus_timeouts
                ),
            )
        } else {
            self.build_evaluation_result(
                "Consensus timeouts metric okay".to_string(),
                100,
                format!(
                    "The consensus timeouts count was {} in the first round and {} in the second round of metrics collection, which is within tolerance of the allowed increase ({})",
                    previous_timeouts_count, latest_timeouts_count, self.args.allowed_consensus_timeouts
                ),
            )
        }
    }
}

#[async_trait::async_trait]
impl Evaluator for ConsensusTimeoutsEvaluator {
    type Input = MetricsEvaluatorInput;
    type Error = MetricsEvaluatorError;

    /// Assert that the consensus timeouts are not increasing too much.
    async fn evaluate(&self, input: &Self::Input) -> Result<Vec<EvaluationResult>, Self::Error> {
        let mut evaluation_results = vec![];

        let previous_timeouts_count = self
            .get_consensus_timeouts(&input.previous_target_metrics, "first")
            .unwrap(&mut evaluation_results);

        let latest_timeouts_count = self
            .get_consensus_timeouts(&input.latest_target_metrics, "second")
            .unwrap(&mut evaluation_results);

        match (previous_timeouts_count, latest_timeouts_count) {
            (Some(previous), Some(latest)) => {
                evaluation_results.push(self.build_evaluation(previous, latest));
            }
            _ => {
                debug!(
                    "Not evaluating timeouts count because we're missing metrics from the target"
                );
            }
        };

        Ok(evaluation_results)
    }

    fn get_category_name() -> String {
        CATEGORY.to_string()
    }

    fn get_evaluator_name() -> String {
        "timeouts".to_string()
    }

    fn from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<Self> {
        Ok(Self::new(evaluator_args.consensus_timeouts_args.clone()))
    }

    fn evaluator_type_from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<EvaluatorType> {
        Ok(EvaluatorType::Metrics(Box::new(Self::from_evaluator_args(
            evaluator_args,
        )?)))
    }
}
