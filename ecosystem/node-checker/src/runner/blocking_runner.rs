// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::{Runner, RunnerError};
use crate::{
    configuration::NodeAddress,
    evaluator::{EvaluationResult, EvaluationSummary, Evaluator},
    evaluators::{
        direct::{DirectEvaluatorInput, NodeIdentityEvaluator},
        metrics::{parse_metrics, MetricsEvaluatorInput},
        system_information::SystemInformationEvaluatorInput,
        EvaluatorSet, EvaluatorType,
    },
    metric_collector::{MetricCollector, SystemInformation},
    server::NodeInformation,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::Parser;
use futures::future::{try_join_all, BoxFuture, TryFutureExt};
use log::{debug, info};
use poem_openapi::Object as PoemObject;
use prometheus_parse::Scrape as PrometheusScrape;
use serde::{Deserialize, Serialize};
use tokio::{time::Duration, try_join};

#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
pub struct BlockingRunnerArgs {
    #[clap(long, default_value = "5")]
    pub metrics_fetch_delay_secs: u64,

    #[clap(long, default_value_t = 4)]
    pub api_client_timeout_secs: u64,
}

#[derive(Debug)]
pub struct BlockingRunner<M: MetricCollector> {
    args: BlockingRunnerArgs,
    baseline_node_information: NodeInformation,
    baseline_metric_collector: M,
    node_identity_evaluator: NodeIdentityEvaluator,
    evaluator_set: EvaluatorSet,
}

impl<M: MetricCollector> BlockingRunner<M> {
    pub fn new(
        args: BlockingRunnerArgs,
        baseline_node_information: NodeInformation,
        baseline_metric_collector: M,
        node_identity_evaluator: NodeIdentityEvaluator,
        evaluator_set: EvaluatorSet,
    ) -> Self {
        Self {
            args,
            baseline_node_information,
            baseline_metric_collector,
            node_identity_evaluator,
            evaluator_set,
        }
    }

    fn collect_metrics_failed(address: &NodeAddress, error: RunnerError) -> EvaluationResult {
        EvaluationResult {
            headline: "Failed to collect metrics from target node".to_string(),
            score: 0,
            explanation: format!("Failed to collect metrics from your node, make sure your metrics port ({}) is publicly accessible: {}", address.get_metrics_port(), error),
            category: "metrics".to_string(),
            evaluator_name: "metrics_port".to_string(),
            links: vec![],
        }
    }

    async fn collect_metrics<MC: MetricCollector>(
        metric_collector: &MC,
    ) -> Result<PrometheusScrape, RunnerError> {
        let lines = metric_collector.collect_metrics().await?;
        parse_metrics(lines)
            .context("Failed to parse metrics response")
            .map_err(RunnerError::ParseMetricsError)
    }

    async fn collect_system_information<MC: MetricCollector>(
        metric_collector: &MC,
    ) -> Result<SystemInformation, RunnerError> {
        Ok(metric_collector.collect_system_information().await?)
    }

    async fn run_metrics_evaluators<T: MetricCollector>(
        &self,
        target_metric_collector: &T,
        target_node_address: &NodeAddress,
    ) -> Result<Vec<EvaluationResult>, RunnerError> {
        let evaluators = self.evaluator_set.get_metrics_evaluators();

        if evaluators.is_empty() {
            return Ok(vec![]);
        }

        let first_target_metrics = match Self::collect_metrics(target_metric_collector).await {
            Ok(scrape) => scrape,
            Err(e) => return Ok(vec![Self::collect_metrics_failed(target_node_address, e)]),
        };
        let first_baseline_metrics = Self::collect_metrics(&self.baseline_metric_collector).await?;

        tokio::time::sleep(Duration::from_secs(self.args.metrics_fetch_delay_secs)).await;

        let second_target_metrics = match Self::collect_metrics(target_metric_collector).await {
            Ok(scrape) => scrape,
            Err(e) => return Ok(vec![Self::collect_metrics_failed(target_node_address, e)]),
        };
        let second_baseline_metrics =
            Self::collect_metrics(&self.baseline_metric_collector).await?;

        let input = MetricsEvaluatorInput {
            previous_baseline_metrics: first_baseline_metrics,
            previous_target_metrics: first_target_metrics,
            latest_baseline_metrics: second_baseline_metrics,
            latest_target_metrics: second_target_metrics,
        };

        let futures: Vec<BoxFuture<_>> = evaluators
            .iter()
            .map(|evaluator| evaluator.evaluate(&input))
            .collect();

        Ok(try_join_all(futures).await?.into_iter().flatten().collect())
    }

    async fn run_system_information_evaluators<T: MetricCollector>(
        &self,
        target_metric_collector: &T,
        target_node_address: &NodeAddress,
    ) -> Result<Vec<EvaluationResult>, RunnerError> {
        let evaluators = self.evaluator_set.get_system_information_evaluators();

        if evaluators.is_empty() {
            return Ok(vec![]);
        }

        let target_system_information =
            match Self::collect_system_information(target_metric_collector).await {
                Ok(info) => info,
                Err(e) => return Ok(vec![Self::collect_metrics_failed(target_node_address, e)]),
            };
        let baseline_system_information =
            Self::collect_system_information(&self.baseline_metric_collector).await?;

        let input = SystemInformationEvaluatorInput {
            baseline_system_information,
            target_system_information,
        };

        let futures: Vec<BoxFuture<_>> = evaluators
            .iter()
            .map(|evaluator| evaluator.evaluate(&input))
            .collect();

        Ok(try_join_all(futures).await?.into_iter().flatten().collect())
    }

    async fn run_direct_evaluators(
        &self,
        direct_evaluator_input: &DirectEvaluatorInput,
    ) -> Result<Vec<EvaluationResult>, RunnerError> {
        let evaluators = self.evaluator_set.get_direct_evaluators();

        let mut futures: Vec<BoxFuture<_>> = vec![];
        for evaluator in &evaluators {
            futures.push(match evaluator {
                EvaluatorType::Tps(evaluator) => Box::pin(
                    evaluator
                        .evaluate(direct_evaluator_input)
                        .err_into::<RunnerError>(),
                ),
                EvaluatorType::Api(evaluator) => Box::pin(
                    evaluator
                        .evaluate(direct_evaluator_input)
                        .err_into::<RunnerError>(),
                ),
                EvaluatorType::Noise(evaluator) => Box::pin(
                    evaluator
                        .evaluate(direct_evaluator_input)
                        .err_into::<RunnerError>(),
                ),
                _ => continue,
            });
        }

        Ok(try_join_all(futures).await?.into_iter().flatten().collect())
    }
}

/// This runner doesn't block in the multithreading sense, but from the user
/// perspective. To run the health check, we pull metrics once, wait, and then
/// pull the metrics again. It does not support continually running beyond this
/// point. You can imagine smarter versions of this where you store the last seen
/// set of metrics, then compare against that, or perhaps even multiple previously
/// seen sets of metrics and do more complex analysis. Additionally we could leverage
/// things like long polling +/ sticky routing to make it that the client request
/// doesn't just hang waiting for the run to complete.
#[async_trait]
impl<M: MetricCollector> Runner for BlockingRunner<M> {
    async fn run<T: MetricCollector>(
        &self,
        target_node_address: &NodeAddress,
        target_metric_collector: &T,
    ) -> Result<EvaluationSummary, RunnerError> {
        info!("Running evaluation for {}", target_node_address.url);

        let api_client_timeout = Duration::from_secs(self.args.api_client_timeout_secs);

        let baseline_index_response = self
            .baseline_node_information
            .node_address
            .get_index_response(api_client_timeout)
            .await
            .context(format!(
            "Failed to read index response from baseline node. Make sure its API is open (port {})",
            self.baseline_node_information.node_address.get_api_port()
        ))
            .map_err(RunnerError::BaselineMissingDataError)?;

        let target_index_response = match target_node_address
            .get_index_response_or_evaluation_result(api_client_timeout)
            .await
        {
            Ok(response) => response,
            Err(evaluation_result) => return Ok(EvaluationSummary::from(vec![evaluation_result])),
        };

        let direct_evaluator_input = DirectEvaluatorInput {
            baseline_node_information: self.baseline_node_information.clone(),
            target_node_address: target_node_address.clone(),
            baseline_index_response,
            target_index_response,
        };

        debug!("Confirming node identity matches");
        let node_identity_evaluations = self
            .node_identity_evaluator
            .evaluate(&direct_evaluator_input)
            .await
            .map_err(RunnerError::NodeIdentityEvaluatorError)?;

        // Exit early if a node identity evaluation returned a non-passing result.
        for evaluation in &node_identity_evaluations {
            if evaluation.score != 100 {
                return Ok(EvaluationSummary::from(node_identity_evaluations));
            }
        }
        let mut evaluation_results = node_identity_evaluations;

        // Run these different classes of evaluator wrappers simultaneously.
        // By evaluator wrapper I mean, these are functions that collect all
        // the information necessary, e.g. fetching metrics, and then run all
        // the evaluators that depend on that information.
        let (mut metrics_results, mut system_information_results, mut direct_results) = try_join!(
            self.run_metrics_evaluators(target_metric_collector, target_node_address),
            self.run_system_information_evaluators(target_metric_collector, target_node_address),
            self.run_direct_evaluators(&direct_evaluator_input)
        )?;
        evaluation_results.append(&mut metrics_results);
        evaluation_results.append(&mut system_information_results);
        evaluation_results.append(&mut direct_results);

        let complete_evaluation = EvaluationSummary::from(evaluation_results);

        Ok(complete_evaluation)
    }

    fn get_evaluator_set(&self) -> &EvaluatorSet {
        &self.evaluator_set
    }
}
