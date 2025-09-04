// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This Checker is only valuable in certain contexts. For example, this is
//! not a useful Checker for node registration for the AITs, since each node
//! is running in their own isolated network, where no consensus is occurring.
//! This is useful for the AIT itself though, where the nodes are participating
//! in a real network.

use super::{CheckResult, Checker, CheckerError, CommonCheckerConfig};
use crate::{
    get_provider,
    provider::{
        metrics::{get_metric, GetMetricResult, MetricsProvider},
        Provider, ProviderCollection,
    },
};
use anyhow::Result;
use prometheus_parse::Scrape;
use serde::{Deserialize, Serialize};

// TODO: When we have it, switch to using a crate that unifies metric names.
// As it is now, this metric name could change and we'd never catch it here
// at compile time.
const METRIC: &str = "velor_consensus_timeout_count";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConsensusTimeoutsCheckerConfig {
    #[serde(flatten)]
    pub common: CommonCheckerConfig,

    /// The amount by which timeouts are allowed to increase between each
    /// round of metrics collection.
    #[serde(default)]
    pub allowed_consensus_timeouts: u64,
}

#[derive(Debug)]
pub struct ConsensusTimeoutsChecker {
    config: ConsensusTimeoutsCheckerConfig,
}

impl ConsensusTimeoutsChecker {
    pub fn new(config: ConsensusTimeoutsCheckerConfig) -> Self {
        Self { config }
    }

    fn get_consensus_timeouts(&self, metrics: &Scrape, metrics_round: &str) -> GetMetricResult {
        let result_on_missing_fn = || {
            Self::build_result(
                "Consensus timeouts metric missing".to_string(),
                0,
                format!(
                    "The {} set of metrics from the target node is missing the metric: {}",
                    metrics_round, METRIC
                ),
            )
        };
        get_metric(metrics, METRIC, None, result_on_missing_fn)
    }

    #[allow(clippy::comparison_chain)]
    fn build_check_result(
        &self,
        previous_timeouts_count: u64,
        latest_timeouts_count: u64,
    ) -> CheckResult {
        if latest_timeouts_count > previous_timeouts_count + self.config.allowed_consensus_timeouts
        {
            Self::build_result(
                "Consensus timeouts metric increased".to_string(),
                50,
                format!(
                    "The consensus timeouts count increased from {} to {} between metrics rounds more than the allowed amount ({})",
                    previous_timeouts_count, latest_timeouts_count, self.config.allowed_consensus_timeouts
                ),
            )
        } else {
            Self::build_result(
                "Consensus timeouts metric okay".to_string(),
                100,
                format!(
                    "The consensus timeouts count was {} in the first round and {} in the second round of metrics collection, which is within tolerance of the allowed increase ({})",
                    previous_timeouts_count, latest_timeouts_count, self.config.allowed_consensus_timeouts
                ),
            )
        }
    }
}

#[async_trait::async_trait]
impl Checker for ConsensusTimeoutsChecker {
    /// Assert that the consensus round is increasing on the target node.
    async fn check(
        &self,
        providers: &ProviderCollection,
    ) -> Result<Vec<CheckResult>, CheckerError> {
        let target_metrics_provider = get_provider!(
            providers.target_metrics_provider,
            self.config.common.required,
            MetricsProvider
        );

        let first_scrape = match target_metrics_provider.provide().await {
            Ok(scrape) => scrape,
            Err(e) => {
                return Ok(vec![Self::build_result(
                    "Failed to check consensus timeouts".to_string(),
                    0,
                    format!(
                        "Failed to scrape metrics from your node (1st time): {:#}",
                        e
                    ),
                )])
            },
        };

        tokio::time::sleep(target_metrics_provider.config.common.check_delay()).await;

        let second_scrape = match target_metrics_provider.provide().await {
            Ok(scrape) => scrape,
            Err(e) => {
                return Ok(vec![Self::build_result(
                    "Failed to check consensus timeouts".to_string(),
                    0,
                    format!(
                        "Failed to scrape metrics from your node (2nd time): {:#}",
                        e
                    ),
                )])
            },
        };

        let mut check_results = vec![];

        let previous_round = self
            .get_consensus_timeouts(&first_scrape, "first")
            .unwrap(&mut check_results);

        let latest_round = self
            .get_consensus_timeouts(&second_scrape, "second")
            .unwrap(&mut check_results);

        if !check_results.is_empty() {
            return Ok(check_results);
        }

        Ok(vec![self.build_check_result(
            previous_round.unwrap(),
            latest_round.unwrap(),
        )])
    }
}
