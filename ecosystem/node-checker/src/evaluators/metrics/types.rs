// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use prometheus_parse::Scrape as PrometheusScrape;
use thiserror::Error as ThisError;

// todo talk about how these are not refs, but this gets passed in
// as a ref. talk about how the former is possible but associated
// types with lifetimes isn't supported very well in stable rust
// right now, refer to https://github.com/rust-lang/rust/issues/44265.
#[derive(Debug)]
pub struct MetricsEvaluatorInput {
    pub previous_baseline_metrics: PrometheusScrape,
    pub previous_target_metrics: PrometheusScrape,
    pub latest_baseline_metrics: PrometheusScrape,
    pub latest_target_metrics: PrometheusScrape,
}

#[derive(Debug, ThisError)]
pub enum MetricsEvaluatorError {
    /// The metric we're evaluating is missing from the baseline. Args:
    ///   - The metric name.
    ///   - Explanation.
    /// When the target node is missing a metric, we return an Evaluation
    /// indicating that something is wrong with the target node, but if the
    /// baseline node is missing a metric, it implies that something is wrong
    /// with our node checker configuration, so we return an error here.
    #[error("A baseline metric was missing. Metric name: {0}, Explanation: {1}")]
    MissingBaselineMetric(String, String),
}
