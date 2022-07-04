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
const METRIC: &str = "aptos_consensus_last_committed_round";

#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
pub struct ConsensusRoundEvaluatorArgs {}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ConsensusRoundEvaluator {
    args: ConsensusRoundEvaluatorArgs,
}

impl ConsensusRoundEvaluator {
    pub fn new(args: ConsensusRoundEvaluatorArgs) -> Self {
        Self { args }
    }

    fn get_consensus_round(
        &self,
        metrics: &PrometheusScrape,
        metrics_round: &str,
    ) -> GetMetricResult {
        let evaluation_on_missing_fn = || {
            self.build_evaluation_result(
                "Consensus round metric missing".to_string(),
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
    fn build_evaluation(&self, previous_round: u64, latest_round: u64) -> EvaluationResult {
        if latest_round < previous_round {
            self.build_evaluation_result(
                "Consensus round went backwards!".to_string(),
                0,
                format!("Successfully pulled metrics from target node twice, but the second time the consensus round went backwards (from {} to {}", previous_round, latest_round),
            )
        } else if latest_round == previous_round {
            self.build_evaluation_result(
                "Consensus round is not progressing".to_string(),
                50,
                "Successfully pulled metrics from target node twice, but the consensus round isn't progressing.".to_string(),
            )
        } else {
            self.build_evaluation_result(
                "Consensus round is increasing".to_string(),
                100,
                format!("Successfully pulled metrics from target node twice and saw that consensus round increased (from {} to {})", previous_round, latest_round),
            )
        }
    }
}

#[async_trait::async_trait]
impl Evaluator for ConsensusRoundEvaluator {
    type Input = MetricsEvaluatorInput;
    type Error = MetricsEvaluatorError;

    /// Assert that the consensus round is increasing on the target node.
    async fn evaluate(&self, input: &Self::Input) -> Result<Vec<EvaluationResult>, Self::Error> {
        let mut evaluation_results = vec![];

        let previous_round = self
            .get_consensus_round(&input.previous_target_metrics, "first")
            .unwrap(&mut evaluation_results);

        let latest_round = self
            .get_consensus_round(&input.latest_target_metrics, "second")
            .unwrap(&mut evaluation_results);

        match (previous_round, latest_round) {
            (Some(previous), Some(latest)) => {
                evaluation_results.push(self.build_evaluation(previous, latest));
            }
            _ => {
                debug!(
                    "Not evaluating consensus round because we're missing metrics from the target"
                );
            }
        };

        Ok(evaluation_results)
    }

    fn get_category_name() -> String {
        CATEGORY.to_string()
    }

    fn get_evaluator_name() -> String {
        "round".to_string()
    }

    fn from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<Self> {
        Ok(Self::new(evaluator_args.consensus_round_args.clone()))
    }

    fn evaluator_type_from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<EvaluatorType> {
        Ok(EvaluatorType::Metrics(Box::new(Self::from_evaluator_args(
            evaluator_args,
        )?)))
    }
}
