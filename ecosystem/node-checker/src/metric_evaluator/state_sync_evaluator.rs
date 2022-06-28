// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::{
    common::get_metric_value, types::EvaluationResult, MetricsEvaluator, MetricsEvaluatorError,
};
use anyhow::Result;
use clap::Parser;
use log::debug;
use poem_openapi::Object as PoemObject;
use prometheus_parse::Scrape as PrometheusScrape;
use serde::{Deserialize, Serialize};

pub const NAME: &str = "state_sync";

// TODO: When we have it, switch to using a crate that unifies metric names.
// As it is now, this metric name could change and we'd never catch it here
// at compile time.
const STATE_SYNC_METRIC: &str = "aptos_state_sync_version";

#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
pub struct StateSyncMetricsEvaluatorArgs {
    #[clap(long, default_value = "5000")]
    pub version_delta_tolerance: u64,
}

#[derive(Debug)]
pub struct StateSyncMetricsEvaluator {
    args: StateSyncMetricsEvaluatorArgs,
}

impl StateSyncMetricsEvaluator {
    pub fn new(args: StateSyncMetricsEvaluatorArgs) -> Self {
        Self { args }
    }

    fn get_sync_version(&self, metrics: &PrometheusScrape) -> Option<u64> {
        get_metric_value(metrics, STATE_SYNC_METRIC, "type", "synced")
    }

    fn evaluate_version_presence(
        &self,
        version: &Option<u64>,
        metrics_round: &str,
    ) -> Option<EvaluationResult> {
        match version {
            Some(_v) => None,
            None => Some(EvaluationResult {
                headline: "State sync version metric missing".to_string(),
                score: 0,
                explanation: format!("The {} set of metrics from the target node is missing the state sync metric: {}", metrics_round, STATE_SYNC_METRIC),
                source: self.get_name(),
                links: vec![],
            }),
        }
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
                EvaluationResult {
                    headline: "State sync version is not progressing".to_string(),
                    score: 50,
                    explanation: "Successfully pulled metrics from target node twice, but the metrics aren't progressing.".to_string(),
                    source: self.get_name(),
                    links: vec![],
              }
            }
            target_progress if (target_progress < 0) => {
                EvaluationResult {
                    headline: "State sync version went backwards!".to_string(),
                    score: 0,
                    explanation: "Successfully pulled metrics from target node twice, but the second time the state sync version went backwards!".to_string(),
                    source: self.get_name(),
                    links: vec![],
                }
            }
            _wildcard => {
                // We convert to i64 to avoid potential overflow if the target is ahead of the baseline.
                let delta_from_baseline = latest_baseline_version as i64 - latest_target_version as i64;
                if delta_from_baseline > self.args.version_delta_tolerance as i64 {
                    EvaluationResult {
                        headline: "State sync version is lagging".to_string(),
                        score: 70,
                        explanation: format!(
                            "Successfully pulled metrics from target node twice and saw the \
                            version was progressing, but it is lagging {} versions behind the baseline node. \
                            Target version: {}. Baseline version: {}. Tolerance: {}.",
                            delta_from_baseline, latest_target_version, latest_baseline_version, self.args.version_delta_tolerance
                        ),
                        source: self.get_name(),
                        links: vec![],
                    }
                } else {
                    EvaluationResult {
                        headline: "State sync version is within tolerance".to_string(),
                        score: 100,
                        explanation: format!(
                            "Successfully pulled metrics from target node twice, saw the \
                            version was progressing, and saw that it is within tolerance \
                            of the baseline node. \
                            Target version: {}. Baseline version: {}. Tolerance: {}.",
                            latest_target_version,
                            latest_baseline_version,
                            self.args.version_delta_tolerance
                        ),
                        source: self.get_name(),
                        links: vec![],
                    }
                }
            }
        }
    }
}

impl MetricsEvaluator for StateSyncMetricsEvaluator {
    /// Assert that the state sync version is increasing on the target node
    /// and that we're within tolerance of the baseline node's latest version.
    fn evaluate_metrics(
        &self,
        _previous_baseline_metrics: &PrometheusScrape,
        previous_target_metrics: &PrometheusScrape,
        latest_baseline_metrics: &PrometheusScrape,
        latest_target_metrics: &PrometheusScrape,
    ) -> Result<Vec<EvaluationResult>, MetricsEvaluatorError> {
        let mut evaluations = vec![];

        // Get previous version from the target node.
        let previous_target_version = self.get_sync_version(previous_target_metrics);

        if let Some(evaluation) = self.evaluate_version_presence(&previous_target_version, "first")
        {
            evaluations.push(evaluation);
        }

        // Get the latest version from the target node.
        let latest_target_version = self.get_sync_version(latest_target_metrics);

        if let Some(evaluation) = self.evaluate_version_presence(&latest_target_version, "second") {
            evaluations.push(evaluation);
        }

        // Get the latest state sync version from the baseline node.
        let latest_baseline_version =
            self.get_sync_version(latest_baseline_metrics)
                .ok_or_else(|| {
                    MetricsEvaluatorError::MissingBaselineMetric(
                        STATE_SYNC_METRIC.to_string(),
                        "The latest set of metrics from the baseline node did not contain the necessary key"
                            .to_string(),
                    )
                })?;

        match (previous_target_version, latest_target_version) {
            (Some(previous), Some(latest)) => {
                evaluations.push(self.build_state_sync_version_evaluation(
                    previous,
                    latest,
                    latest_baseline_version,
                ));
            }
            _ => {
                debug!("Not evaluating state sync version because we're missing metrics from the target");
            }
        };

        Ok(evaluations)
    }

    fn get_name(&self) -> String {
        NAME.to_string()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::metric_evaluator::common::parse_metrics;

    fn get_metric_string(value: u64) -> String {
        let mut metric_string = r#"aptos_state_sync_version{type="synced"} "#.to_string();
        metric_string.push_str(&format!("{}", value));
        metric_string
    }

    fn test_state_sync_metrics_evaluator(
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
            StateSyncMetricsEvaluator::new(StateSyncMetricsEvaluatorArgs {
                version_delta_tolerance: 1000,
            });
        let evaluations = state_sync_metrics_evaluator
            .evaluate_metrics(
                &parse_metrics(previous_baseline_metrics).unwrap(),
                &parse_metrics(previous_target_metrics).unwrap(),
                &parse_metrics(latest_baseline_metrics).unwrap(),
                &parse_metrics(latest_target_metrics).unwrap(),
            )
            .expect("Failed to evaluate metrics");

        let expected_evaluations_len =
            match omit_previous_target_metric && omit_latest_target_metric {
                true => 2,
                false => 1,
            };

        assert_eq!(evaluations.len(), expected_evaluations_len);
        assert_eq!(evaluations[0].score, expected_score);
    }

    #[test]
    fn test_in_sync_and_progressing() {
        test_state_sync_metrics_evaluator(1000, 2000, 1700, 100, false, false);
    }

    #[test]
    fn test_progressing_but_lagging() {
        test_state_sync_metrics_evaluator(1000, 5000, 3000, 70, false, false);
    }

    #[test]
    fn test_not_progressing() {
        test_state_sync_metrics_evaluator(1000, 5000, 1000, 50, false, false);
    }

    #[test]
    fn test_missing_metric() {
        test_state_sync_metrics_evaluator(1000, 5000, 1000, 0, true, false);
    }

    #[test]
    fn test_both_missing_metrics() {
        test_state_sync_metrics_evaluator(1000, 5000, 1000, 0, true, true);
    }
}
