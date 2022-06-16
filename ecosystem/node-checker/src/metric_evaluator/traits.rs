// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::evaluator::EvaluationResult;
use anyhow::Result;
use prometheus_parse::Scrape as PrometheusScrape;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum MetricsEvaluatorError {
    /// The metric we're evaluating is missing from the baseline. Args:
    ///   - The metric name.
    ///   - Explanation.
    /// When the target node is missing a metric, we return an Evaluation
    /// indiating that something is wrong with the target node, but if the
    /// baseline node is missing a metric, it implies that something is wrong
    /// without our node checker configuration, so we return an error here.
    #[error("A baseline metric was missing. Metric name: {0}, Explanation: {1}")]
    MissingBaselineMetric(String, String),
}

/// todo describe the trait
/// todo assert these trait constraints are necessary
/// todo consider whether we need Clone if we need to spawn multiple handlers ourselves.
///
/// This is only for metrics evaluation, we will need a different
/// more permissive trait for other evaluation types. ideally we will still be able
/// to return Evaluation from those too (in which case we lift that type up), but
/// if not, we can use a trait instead.
///
/// Note:
///  - Sync + Send is required because this will be a member of the todo which needs
///      to be used across async boundaries
///
///  - 'static is required because this will be stored on the todo which needs to be 'static
pub trait MetricsEvaluator: Sync + Send {
    fn evaluate_metrics(
        &self,
        previous_baseline_metrics: &PrometheusScrape,
        previous_target_metrics: &PrometheusScrape,
        latest_baseline_metrics: &PrometheusScrape,
        latest_target_metrics: &PrometheusScrape,
    ) -> Result<Vec<EvaluationResult>, MetricsEvaluatorError>;

    /// todo
    fn get_name(&self) -> String;
}

impl std::fmt::Debug for dyn MetricsEvaluator {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "MetricsEvaluator {{ name: {:?} }}", self.get_name())
    }
}
