// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Error, Result};
use async_trait::async_trait;
use thiserror::Error as ThisError;

use crate::{
    metric_collector::{MetricCollector, MetricCollectorError},
    metric_evaluator::{EvaluationSummary, MetricsEvaluatorError},
};

// TODO: Consider using thiserror.
// todo: Rename MetricsEvaluator to MetricEvaluator

#[derive(Debug, ThisError)]
pub enum RunnerError {
    /// We failed to collect metrics for some reason.
    #[error("Failed to collect metrics")]
    MetricCollectorError(MetricCollectorError),

    /// We couldn't parse the metrics.
    #[error("Failed to parse metrics")]
    ParseMetricsError(Error),

    /// One of the evaluators failed. This is not the same as a poor score from
    /// an evaluator, this is an actual failure in the evaluation process.
    #[error("Failed to evaluate metrics")]
    MetricEvaluatorError(MetricsEvaluatorError),

    #[error("Encountered an unknown error")]
    UnknownError(Error),
}

// This runner doesn't block in the multithreading sense, but from the user
// perspective. To run the health check, we pull metrics once, wait, and then
// pull the metrics again. It does not support continually running beyond this
// point. You can imagine smarter versions of this where you store the last seen
// set of metrics, then compare against that, or perhaps even multiple previously
// seen sets of metrics and do more complex analysis. Additionally we could leverage
// things like long polling +/ sticky routing to make it that the client request
// doesn't just hang waiting for the run to complete.

/// todo describe the trait
/// todo assert these trait constraints are necessary
/// todo consider whether we need Clone if we need to spawn multiple handlers ourselves.
///
/// Note:
///  - Sync + Send is required because this will be a member of the todo which needs
///      to be used across async boundaries
///
///  - 'static is required because this will be stored on the todo which needs to be 'static
#[async_trait]
pub trait Runner: Sync + Send + 'static {
    // TODO: add proper result type.
    async fn run<M: MetricCollector>(
        &self,
        target_collector: &M,
    ) -> Result<EvaluationSummary, RunnerError>;
}
