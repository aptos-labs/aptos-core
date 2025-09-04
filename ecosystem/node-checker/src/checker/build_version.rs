// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// TODO: Sometimes build_commit_hash is an empty string (so far I've noticed
// this happens when targeting a node running from a container). Figure out
// what to do in this case.

use super::{CheckResult, Checker, CheckerError, CommonCheckerConfig};
use crate::{
    get_provider,
    provider::{
        system_information::{
            get_value, GetValueResult, SystemInformation, SystemInformationProvider,
        },
        Provider, ProviderCollection,
    },
};
use anyhow::{anyhow, Result};
use velor_logger::debug;
use serde::{Deserialize, Serialize};

// TODO: Use the key in crates/velor-telemetry/src/build_information.rs
const BUILD_COMMIT_HASH_KEY: &str = "build_commit_hash";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildVersionCheckerConfig {
    #[serde(flatten)]
    pub common: CommonCheckerConfig,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct BuildVersionChecker {
    config: BuildVersionCheckerConfig,
}

impl BuildVersionChecker {
    pub fn new(config: BuildVersionCheckerConfig) -> Self {
        Self { config }
    }

    fn get_build_commit_hash(&self, system_information: &SystemInformation) -> GetValueResult {
        let evaluation_on_missing_fn = || {
            Self::build_result(
                "Build commit hash value missing".to_string(),
                0,
                format!(
                    "The build information from the node is missing: {}",
                    BUILD_COMMIT_HASH_KEY
                ),
            )
        };
        get_value(
            system_information,
            BUILD_COMMIT_HASH_KEY,
            evaluation_on_missing_fn,
        )
    }
}

#[async_trait::async_trait]
impl Checker for BuildVersionChecker {
    /// Assert that the build commit hashes match.
    async fn check(
        &self,
        providers: &ProviderCollection,
    ) -> Result<Vec<CheckResult>, CheckerError> {
        let baseline_provider = get_provider!(
            providers.baseline_system_information_provider,
            self.config.common.required,
            SystemInformationProvider
        );
        let target_provider = get_provider!(
            providers.target_system_information_provider,
            self.config.common.required,
            SystemInformationProvider
        );

        let baseline_information = baseline_provider.provide().await?;
        let target_information = match target_provider.provide().await {
            Ok(info) => info,
            Err(e) => {
                return Ok(vec![Self::build_result(
                    "Failed to check build version".to_string(),
                    0,
                    format!("Failed to get system information from your node: {:#}", e),
                )])
            },
        };

        let mut check_results = vec![];

        let baseline_build_commit_hash = match self.get_build_commit_hash(&baseline_information) {
            GetValueResult::Present(value) => value,
            GetValueResult::Missing(_evaluation_result) => {
                return
                    Err(CheckerError::MissingDataError(
                        BUILD_COMMIT_HASH_KEY,
                        anyhow!("The latest set of metrics from the baseline node did not contain the necessary key \"{}\"", BUILD_COMMIT_HASH_KEY),
                    ));
            },
        };

        let target_build_commit_hash = match self.get_build_commit_hash(&target_information) {
            GetValueResult::Present(value) => Some(value),
            GetValueResult::Missing(evaluation_result) => {
                check_results.push(evaluation_result);
                None
            },
        };

        match target_build_commit_hash {
            Some(target_build_commit_hash) => {
                check_results.push({
                    if baseline_build_commit_hash == target_build_commit_hash {
                        Self::build_result(
                            "Build commit hashes match".to_string(),
                            100,
                            format!(
                                "The build commit hash from the target node ({}) matches the build commit hash from the baseline node ({}).",
                                target_build_commit_hash, baseline_build_commit_hash
                            ),
                        )
                    } else {
                        Self::build_result(
                            "Build commit hash mismatch".to_string(),
                            50,
                            format!(
                                "The build commit hash from the target node ({}) does not match the build commit hash from the baseline node ({}).",
                                target_build_commit_hash, baseline_build_commit_hash
                            ),
                        )
                    }
                });
            },
            None => debug!(
                "Not evaluating build commit hash because we're missing data from the target"
            ),
        }

        Ok(check_results)
    }
}
