// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

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
const METRIC: &str = "velor_consensus_last_committed_round";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConsensusRoundCheckerConfig {
    #[serde(flatten)]
    pub common: CommonCheckerConfig,
}

#[derive(Debug)]
pub struct ConsensusRoundChecker {
    config: ConsensusRoundCheckerConfig,
}

impl ConsensusRoundChecker {
    pub fn new(config: ConsensusRoundCheckerConfig) -> Self {
        Self { config }
    }

    fn get_consensus_round(&self, metrics: &Scrape, metrics_round: &str) -> GetMetricResult {
        let check_on_missing_fn = || {
            Self::build_result(
                "Consensus round metric missing".to_string(),
                0,
                format!(
                    "The {} set of metrics from the target node is missing the metric: {}",
                    metrics_round, METRIC
                ),
            )
        };
        get_metric(metrics, METRIC, None, check_on_missing_fn)
    }

    #[allow(clippy::comparison_chain)]
    fn build_check_result(&self, previous_round: u64, latest_round: u64) -> CheckResult {
        if latest_round < previous_round {
            Self::build_result(
                "Consensus round went backwards!".to_string(),
                0,
                format!("Successfully pulled metrics from target node twice, but the second time the consensus round went backwards (from {} to {}", previous_round, latest_round),
            )
        } else if latest_round == previous_round {
            Self::build_result(
                "Consensus round is not progressing".to_string(),
                50,
                "Successfully pulled metrics from target node twice, but the consensus round isn't progressing.".to_string(),
            )
        } else {
            Self::build_result(
                "Consensus round is increasing".to_string(),
                100,
                format!("Successfully pulled metrics from target node twice and saw that consensus round increased (from {} to {})", previous_round, latest_round),
            )
        }
    }
}

#[async_trait::async_trait]
impl Checker for ConsensusRoundChecker {
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
                    "Failed to check consensus round".to_string(),
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
                    "Failed to check consensus round".to_string(),
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
            .get_consensus_round(&first_scrape, "first")
            .unwrap(&mut check_results);

        let latest_round = self
            .get_consensus_round(&second_scrape, "second")
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
