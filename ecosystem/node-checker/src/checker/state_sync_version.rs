// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{CheckResult, Checker, CheckerError, CommonCheckerConfig};
use crate::{
    get_provider,
    provider::{api_index::ApiIndexProvider, Provider, ProviderCollection},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StateSyncVersionCheckerConfig {
    #[serde(flatten)]
    pub common: CommonCheckerConfig,

    #[serde(default = "StateSyncVersionCheckerConfig::default_version_delta_tolerance")]
    pub version_delta_tolerance: u64,
}

impl StateSyncVersionCheckerConfig {
    fn default_version_delta_tolerance() -> u64 {
        5000
    }
}

#[derive(Debug)]
pub struct StateSyncVersionChecker {
    config: StateSyncVersionCheckerConfig,
}

impl StateSyncVersionChecker {
    pub fn new(config: StateSyncVersionCheckerConfig) -> Self {
        Self { config }
    }

    fn build_state_sync_version_check_result(
        &self,
        previous_target_version: u64,
        latest_target_version: u64,
        latest_baseline_version: u64,
        delay_secs: u64,
    ) -> CheckResult {
        // We convert to i64 to avoid potential overflow if somehow the ledger version went backwards.
        let target_progress = latest_target_version as i64 - previous_target_version as i64;
        match target_progress {
            0 => Self::build_result(
                "Ledger version is not increasing".to_string(),
                25,
                format!(
                    "Successfully pulled ledger version from your node \
                        twice, but the ledger version is not increasing ({} both times).",
                    latest_target_version
                ),
            ),
            target_progress if (target_progress < 0) => Self::build_result(
                "Ledger version went backwards!".to_string(),
                0,
                format!(
                    "Successfully pulled ledger version from your node twice, \
                    but the second time the ledger version went backwards! \
                    First datapoint: {}, second datapoint: {}",
                    previous_target_version, latest_target_version
                ),
            ),
            _wildcard => {
                // We convert to i64 to avoid potential overflow if the target is ahead of the baseline.
                let delta_from_baseline =
                    latest_baseline_version as i64 - latest_target_version as i64;
                if delta_from_baseline > self.config.version_delta_tolerance as i64 {
                    Self::build_result(
                        "Ledger version is lagging".to_string(),
                        50,
                        format!(
                            "Successfully pulled ledger version from your node twice \
                            and saw the version was increasing, but it is lagging {} versions \
                            behind the baseline node, more than the allowed lag of {}. \
                            Target version: {}. Baseline version: {}.",
                            delta_from_baseline,
                            self.config.version_delta_tolerance,
                            latest_target_version,
                            latest_baseline_version,
                        ),
                    )
                } else {
                    Self::build_result(
                        "Ledger version is increasing".to_string(),
                        100,
                        format!(
                            "NHC pulled ledger version from your node twice, \
                            saw that the version is increasing (it increased by {} over \
                            {} seconds), and saw that it is within tolerance of the \
                            baseline node. The baseline ledger version is {} and your node's \
                            ledger version is {}, which is within the allowed lag of {} versions.",
                            target_progress,
                            delay_secs,
                            latest_baseline_version,
                            latest_target_version,
                            self.config.version_delta_tolerance
                        ),
                    )
                }
            },
        }
    }
}

#[async_trait::async_trait]
impl Checker for StateSyncVersionChecker {
    /// Assert that the ledger version is increasing on the target node
    /// and that we're within tolerance of the baseline node's latest version.
    async fn check(
        &self,
        providers: &ProviderCollection,
    ) -> Result<Vec<CheckResult>, CheckerError> {
        // Assert we have both a baseline and target API index provider.
        let baseline_api_index_provider = get_provider!(
            providers.baseline_api_index_provider,
            self.config.common.required,
            ApiIndexProvider
        );

        let target_api_index_provider = get_provider!(
            providers.target_api_index_provider,
            self.config.common.required,
            ApiIndexProvider
        );

        // Get one instance of the target node ledger version.
        let previous_target_version = match target_api_index_provider.provide().await {
            Ok(response) => response.ledger_version.0,
            Err(err) => {
                return Ok(vec![Self::build_result(
                    "Failed to determine state sync status".to_string(),
                    0,
                    format!("There was an error querying your node's API: {:#}", err),
                )]);
            },
        };

        // Now wait.
        tokio::time::sleep(target_api_index_provider.config.common.check_delay()).await;

        // Get the target node ledger version x seconds later.
        let latest_target_version = match target_api_index_provider.provide().await {
            Ok(response) => response.ledger_version.0,
            Err(err) => {
                return Ok(vec![Self::build_result(
                    "Failed to determine state sync status".to_string(),
                    0,
                    format!("There was an error querying your node's API: {:#}", err),
                )]);
            },
        };

        // Get the latest version from the baseline node. In this case, if we
        // cannot find the value, we return an error instead of a negative evalution,
        // since this implies some issue with the baseline node / this code.
        let latest_baseline_response = baseline_api_index_provider.provide().await?;
        let latest_baseline_version = latest_baseline_response.ledger_version.0;

        // Evaluate the data, returning a check result.
        Ok(vec![self.build_state_sync_version_check_result(
            previous_target_version,
            latest_target_version,
            latest_baseline_version,
            target_api_index_provider.config.common.check_delay_secs,
        )])
    }
}
