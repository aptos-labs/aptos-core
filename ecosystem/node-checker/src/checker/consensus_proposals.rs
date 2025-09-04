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
const PROPOSALS_METRIC: &str = "velor_consensus_proposals_count";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConsensusProposalsCheckerConfig {
    #[serde(flatten)]
    pub common: CommonCheckerConfig,
}

#[derive(Debug)]
pub struct ConsensusProposalsChecker {
    config: ConsensusProposalsCheckerConfig,
}

impl ConsensusProposalsChecker {
    pub fn new(config: ConsensusProposalsCheckerConfig) -> Self {
        Self { config }
    }

    fn get_proposals_count(&self, metrics: &Scrape, metrics_round: &str) -> GetMetricResult {
        let evaluation_on_missing_fn = || {
            Self::build_result(
                "Consensus proposals metric missing".to_string(),
                0,
                format!(
                    "The {} set of metrics from the target node is missing the proposals metric: {}",
                    metrics_round, PROPOSALS_METRIC
                )
            )
        };
        get_metric(metrics, PROPOSALS_METRIC, None, evaluation_on_missing_fn)
    }

    #[allow(clippy::comparison_chain)]
    fn build_check_result(
        &self,
        previous_proposals_count: u64,
        latest_proposals_count: u64,
    ) -> CheckResult {
        if latest_proposals_count < previous_proposals_count {
            Self::build_result(
                "Proposals count went backwards!".to_string(),
                0,
                format!("Successfully pulled metrics from target node twice, but the second time the consensus proposals count went backwards (from {} to {})", previous_proposals_count, latest_proposals_count),
            )
        } else if latest_proposals_count == previous_proposals_count {
            Self::build_result(
                "Proposals count is not progressing".to_string(),
                50,
                "Successfully pulled metrics from target node twice, but the proposal count isn't progressing.".to_string(),
            )
        } else {
            Self::build_result(
                "Proposals count is increasing".to_string(),
                100,
                format!("Successfully pulled metrics from target node twice and saw that proposals count is increasing (from {} to {})", previous_proposals_count, latest_proposals_count),
            )
        }
    }
}

// See https://github.com/velor-chain/velor-core/pull/1450 for a discussion on
// how this Checker can be improved.
#[async_trait::async_trait]
impl Checker for ConsensusProposalsChecker {
    /// Assert that the proposals count is increasing on the target node.
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
                    "Failed to check consensus proposals".to_string(),
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
                    "Failed to check consensus proposals".to_string(),
                    0,
                    format!(
                        "Failed to scrape metrics from your node (2nd time): {:#}",
                        e
                    ),
                )])
            },
        };

        let mut check_results = vec![];

        // Get previous proposals count from the target node.
        let previous_proposals_count = self
            .get_proposals_count(&first_scrape, "first")
            .unwrap(&mut check_results);

        // Get the latest proposals count from the target node.
        let latest_proposals_count = self
            .get_proposals_count(&second_scrape, "second")
            .unwrap(&mut check_results);

        if !check_results.is_empty() {
            return Ok(check_results);
        }

        Ok(vec![self.build_check_result(
            previous_proposals_count.unwrap(),
            latest_proposals_count.unwrap(),
        )])
    }
}
