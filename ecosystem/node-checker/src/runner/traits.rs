// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Error, Result};
use async_trait::async_trait;
use thiserror::Error as ThisError;

use crate::{
    configuration::NodeAddress,
    evaluator::EvaluationSummary,
    evaluators::{
        direct::{
            ApiEvaluatorError, NodeIdentityEvaluatorError, NoiseEvaluatorError, TpsEvaluatorError,
        },
        metrics::MetricsEvaluatorError,
        system_information::SystemInformationEvaluatorError,
        EvaluatorSet,
    },
    metric_collector::{MetricCollector, MetricCollectorError},
};

#[derive(Debug, ThisError)]
pub enum RunnerError {
    #[error("Failed to evaluate API: {0}")]
    ApiEvaluatorError(#[from] ApiEvaluatorError),

    #[error("Data was missing from the baseline node: {0}")]
    BaselineMissingDataError(Error),

    /// We failed to collect metrics for some reason.
    #[error("Failed to collect metrics: {0}")]
    MetricCollectorError(#[from] MetricCollectorError),

    /// One of the metrics evaluators failed. This is not the same as a poor score from
    /// an evaluator, this is an actual failure in the evaluation process.
    #[error("Failed to evaluate metrics: {0}")]
    MetricEvaluatorError(#[from] MetricsEvaluatorError),

    /// We failed to get the node identity.
    #[error("Failed to check identity of node: {0}")]
    NodeIdentityEvaluatorError(#[from] NodeIdentityEvaluatorError),

    #[error("Failed to evaluate over noise port: {0}")]
    NoiseEvaluatorError(#[from] NoiseEvaluatorError),

    /// We couldn't parse the metrics.
    #[error("Failed to parse metrics: {0}")]
    ParseMetricsError(Error),

    /// One of the system information evaluators failed. This is not the same
    /// as a poor score from an evaluator, this is an actual failure in the
    /// evaluation process.
    #[error("Failed to evaluate system information: {0}")]
    SystemInformationEvaluatorError(#[from] SystemInformationEvaluatorError),

    /// The TPS evaluator failed. This is not the same as a poor score from an
    /// evaluator, this is an actual failure in the evaluation process.
    #[error("Failed to evaluate TPS: {0}")]
    TpsEvaluatorError(#[from] TpsEvaluatorError),
}

/// This trait describes a Runner, something that can take in instances of other
/// necessary traits, such as a metric collector for the baseline node, and then,
/// upon a `run` call, drive a node evaluation end to end. This is the top level
/// entrypoint to the core functionality of NHC, it should be hooked up fairly
/// directly to the API endpoint handlers.
///
/// Note on trait bounds:
///  - Sync + Send is required because this will be a member of the Api which
///    needs to be used across thread boundaries.
///  - The 'static lifetime is required because this will be stored on the Api
///    which needs to be 'static.
#[async_trait]
pub trait Runner: Sync + Send + 'static {
    async fn run<M: MetricCollector>(
        &self,
        target_node_address: &NodeAddress,
        target_metric_collector: &M,
    ) -> Result<EvaluationSummary, RunnerError>;

    fn get_evaluator_set(&self) -> &EvaluatorSet;
}
