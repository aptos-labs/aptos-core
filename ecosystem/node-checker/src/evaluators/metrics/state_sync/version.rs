// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::{
    super::{
        common::{get_metric, GetMetricResult, Label},
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
use once_cell::sync::Lazy;
use poem_openapi::Object as PoemObject;
use prometheus_parse::Scrape as PrometheusScrape;
use serde::{Deserialize, Serialize};

// TODO: When we have it, switch to using a crate that unifies metric names.
// As it is now, this metric name could change and we'd never catch it here
// at compile time.
const STATE_SYNC_METRIC: &str = "aptos_state_sync_version";

pub static SYNC_VERSION_METRIC_LABEL: Lazy<Label> = Lazy::new(|| Label {
    key: "type",
    value: "synced",
});

#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
pub struct StateSyncVersionMetricsEvaluatorArgs {
    #[clap(long, default_value_t = 10000)]
    pub metrics_version_delta_tolerance: u64,
}

#[derive(Debug)]
pub struct StateSyncVersionMetricsEvaluator {
    args: StateSyncVersionMetricsEvaluatorArgs,
}

impl StateSyncVersionMetricsEvaluator {
    pub fn new(args: StateSyncVersionMetricsEvaluatorArgs) -> Self {
        Self { args }
    }

    fn get_sync_version(&self, metrics: &PrometheusScrape, metrics_round: &str) -> GetMetricResult {
        let evaluation_on_missing_fn = || {
            self.build_evaluation_result(
                "State sync version metric missing".to_string(),
                0,
                format!(
                "The {} set of metrics from the target node is missing the state sync metric: {}",
                metrics_round, STATE_SYNC_METRIC
            ),
            )
        };
        get_metric(
            metrics,
            STATE_SYNC_METRIC,
            Some(&SYNC_VERSION_METRIC_LABEL),
            evaluation_on_missing_fn,
        )
    }

    fn build_state_sync_version_evaluation(
        &self,
        previous_target_version: u64,
        latest_target_version: u64,
        latest_baseline_version: u64,
    ) -> EvaluationResult {
        // We convert to i64 to avoid potential overflow if somehow the state sync version went backwards.
        let target_progress = latest_target_version as i64 - previous_target_version as i64;
        match target_progress {
            target_progress if (target_progress == 0) => {
                self.build_evaluation_result(
                    "State sync version is not progressing".to_string(),
                     50,
                    "Successfully pulled metrics from target node twice, but the metrics aren't progressing.".to_string(),
                )
            }
            target_progress if (target_progress < 0) => {
                self.build_evaluation_result(
                    "State sync version went backwards!".to_string(),
                    0,
                    "Successfully pulled metrics from target node twice, but the second time the state sync version went backwards!".to_string(),
                )
            }
            _wildcard => {
                // We convert to i64 to avoid potential overflow if the target is ahead of the baseline.
                let delta_from_baseline = latest_baseline_version as i64 - latest_target_version as i64;
                if delta_from_baseline > self.args.metrics_version_delta_tolerance as i64 {
                    self.build_evaluation_result(
                        "State sync version is lagging".to_string(),
                        70,
                        format!(
                            "Successfully pulled metrics from target node twice and saw the \
                            version was progressing, but it is lagging {} versions behind the baseline node. \
                            Target version: {}. Baseline version: {}. Tolerance: {}.",
                            delta_from_baseline, latest_target_version, latest_baseline_version, self.args.metrics_version_delta_tolerance
                        )
                    )
                } else {
                    self.build_evaluation_result(
                        "State sync version is within tolerance".to_string(),
                        100,
                        format!(
                            "Successfully pulled metrics from target node twice, saw the \
                            version was progressing, and saw that it is within tolerance \
                            of the baseline node. \
                            Target version: {}. Baseline version: {}. Tolerance: {}.",
                            latest_target_version,
                            latest_baseline_version,
                            self.args.metrics_version_delta_tolerance
                        )
                    )
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl Evaluator for StateSyncVersionMetricsEvaluator {
    type Input = MetricsEvaluatorInput;
    type Error = MetricsEvaluatorError;

    /// Assert that the state sync version is increasing on the target node
    /// and that we're within tolerance of the baseline node's latest version.
    async fn evaluate(&self, input: &Self::Input) -> Result<Vec<EvaluationResult>, Self::Error> {
        let mut evaluation_results = vec![];

        // Get previous version from the target node.
        let previous_target_version = self
            .get_sync_version(&input.previous_target_metrics, "first")
            .unwrap(&mut evaluation_results);

        // Get the latest version from the target node.
        let latest_target_version = self
            .get_sync_version(&input.latest_target_metrics, "second")
            .unwrap(&mut evaluation_results);

        // Get the latest version from the baseline node. In this case, if we
        // cannot find the value, we return an error instead of a negative evalution,
        // since this implies some issue with the baseline node / this code.
        let latest_baseline_version = match self
            .get_sync_version(&input.latest_baseline_metrics, "second")
        {
            GetMetricResult::Present(metric) => metric,
            GetMetricResult::Missing(_) => {
                return
                    Err(MetricsEvaluatorError::MissingBaselineMetric(
                        STATE_SYNC_METRIC.to_string(),
                        "The latest set of metrics from the baseline node did not contain the necessary key"
                            .to_string(),
                    ));
            }
        };

        match (previous_target_version, latest_target_version) {
            (Some(previous), Some(latest)) => {
                evaluation_results.push(self.build_state_sync_version_evaluation(
                    previous,
                    latest,
                    latest_baseline_version,
                ));
            }
            _ => {
                debug!("Not evaluating state sync version because we're missing metrics from the target");
            }
        };

        Ok(evaluation_results)
    }

    fn get_category_name() -> String {
        CATEGORY.to_string()
    }

    fn get_evaluator_name() -> String {
        "version_metrics".to_string()
    }

    fn from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<Self> {
        Ok(Self::new(
            evaluator_args.state_sync_version_metrics_args.clone(),
        ))
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
    use std::fmt::Write;
    fn get_metric_string(value: u64) -> String {
        let mut metric_string = r#"aptos_state_sync_version{type="synced"} "#.to_string();
        write!(metric_string, "{}", value).unwrap();
        metric_string
    }

    async fn test_state_sync_metrics_evaluator(
        previous_target_version: u64,
        latest_baseline_version: u64,
        latest_target_version: u64,
        expected_score: u8,
        omit_previous_target_metric: bool,
        omit_latest_target_metric: bool,
    ) {
        let previous_baseline_metrics = vec![get_metric_string(0)]; // This one doesn't matter right now.
        let latest_baseline_metrics = vec![get_metric_string(latest_baseline_version)];

        let previous_target_metrics = match omit_previous_target_metric {
            true => vec![],
            false => vec![get_metric_string(previous_target_version)],
        };

        let latest_target_metrics = match omit_latest_target_metric {
            true => vec![],
            false => vec![get_metric_string(latest_target_version)],
        };

        let state_sync_metrics_evaluator =
            StateSyncVersionMetricsEvaluator::new(StateSyncVersionMetricsEvaluatorArgs {
                metrics_version_delta_tolerance: 1000,
            });

        let metrics_evaluator_input = MetricsEvaluatorInput {
            previous_baseline_metrics: parse_metrics(previous_baseline_metrics).unwrap(),
            previous_target_metrics: parse_metrics(previous_target_metrics).unwrap(),
            latest_baseline_metrics: parse_metrics(latest_baseline_metrics).unwrap(),
            latest_target_metrics: parse_metrics(latest_target_metrics).unwrap(),
        };

        let evaluations = state_sync_metrics_evaluator
            .evaluate(&metrics_evaluator_input)
            .await
            .expect("Failed to evaluate metrics");

        let expected_evaluations_len =
            match omit_previous_target_metric && omit_latest_target_metric {
                true => 2,
                false => 1,
            };

        assert_eq!(evaluations.len(), expected_evaluations_len);
        assert_eq!(evaluations[0].score, expected_score);
    }

    #[tokio::test]
    async fn test_in_sync_and_progressing() {
        test_state_sync_metrics_evaluator(1000, 2000, 1700, 100, false, false).await;
    }

    #[tokio::test]
    async fn test_progressing_but_lagging() {
        test_state_sync_metrics_evaluator(1000, 5000, 3000, 70, false, false).await;
    }

    #[tokio::test]
    async fn test_not_progressing() {
        test_state_sync_metrics_evaluator(1000, 5000, 1000, 50, false, false).await;
    }

    #[tokio::test]
    async fn test_missing_metric() {
        test_state_sync_metrics_evaluator(1000, 5000, 1000, 0, true, false).await;
    }

    #[tokio::test]
    async fn test_both_missing_metrics() {
        test_state_sync_metrics_evaluator(1000, 5000, 1000, 0, true, true).await;
    }
}
