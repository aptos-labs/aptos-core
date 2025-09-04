// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

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
use anyhow::Result;
use serde::{Deserialize, Serialize};

// TODO: Use the keys in crates/velor-telemetry/src/system_information.rs
const CPU_COUNT_KEY: &str = "cpu_count";
const MEMORY_TOTAL_KEY: &str = "memory_total";

const NODE_REQUIREMENTS_DOC_LINK: &str =
    "https://velor.dev/nodes/validator-node/operator/node-requirements";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HardwareCheckerConfig {
    #[serde(flatten)]
    pub common: CommonCheckerConfig,

    /// The minimum number of physical CPU cores the machine must have.
    #[serde(default = "HardwareCheckerConfig::default_min_cpu_cores")]
    pub min_cpu_cores: u64,

    /// The minimum amount of RAM in GB (not GiB) the machine must have.
    #[serde(default = "HardwareCheckerConfig::default_min_ram_gb")]
    pub min_ram_gb: u64,
}

impl HardwareCheckerConfig {
    fn default_min_cpu_cores() -> u64 {
        8
    }

    fn default_min_ram_gb() -> u64 {
        31
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct HardwareChecker {
    config: HardwareCheckerConfig,
}

impl HardwareChecker {
    pub fn new(config: HardwareCheckerConfig) -> Self {
        Self { config }
    }

    // TODO: Make this more general, so we can use it in build_version too.
    fn get_system_information_value(
        &self,
        system_information: &SystemInformation,
        key: &str,
    ) -> GetValueResult {
        let evaluation_on_missing_fn = || {
            Self::build_result(
                format!("Key \"{}\" missing", key),
                0,
                format!(
                    "The key \"{}\" is missing from the system information reported by the node",
                    key
                ),
            )
        };
        get_value(system_information, key, evaluation_on_missing_fn)
    }

    fn check_single_item(
        &self,
        info: &SystemInformation,
        key: &str,
        minimum: u64,
        unit: &str,
    ) -> CheckResult {
        let value_from_target = match self.get_system_information_value(info, key) {
            GetValueResult::Present(value) => match value.parse::<u64>() {
                Ok(value) => value,
                Err(err) => {
                    return Self::build_result(
                        format!("Failed to parse value for key \"{}\" as an int", key),
                        0,
                        format!(
                            "The value ({}) for key \"{}\" could not be parsed as an int: {}",
                            value, key, err
                        ),
                    )
                },
            },
            GetValueResult::Missing(evaluation_result) => {
                return evaluation_result;
            },
        };

        if value_from_target < minimum {
            Self::build_result(
                format!("{} is too low", key),
                25,
                format!(
                    "The value for {} is too small: {} {}. It must be at least {} {}.",
                    key, value_from_target, unit, minimum, unit
                ),
            )
            .links(vec![NODE_REQUIREMENTS_DOC_LINK.to_string()])
        } else {
            Self::build_result(
                format!("{} is large enough", key),
                100,
                format!(
                    "The value for {} is large enough: {} {}. Great! The minimum is {} {}",
                    key, value_from_target, unit, minimum, unit
                ),
            )
            .links(vec![NODE_REQUIREMENTS_DOC_LINK.to_string()])
        }
    }
}

#[async_trait::async_trait]
impl Checker for HardwareChecker {
    /// Assert that the build commit hashes match.
    async fn check(
        &self,
        providers: &ProviderCollection,
    ) -> Result<Vec<CheckResult>, CheckerError> {
        let target_provider = get_provider!(
            providers.target_system_information_provider,
            self.config.common.required,
            SystemInformationProvider
        );

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

        let check_results = vec![
            self.check_single_item(
                &target_information,
                CPU_COUNT_KEY,
                self.config.min_cpu_cores,
                "cores",
            ),
            self.check_single_item(
                &target_information,
                MEMORY_TOTAL_KEY,
                self.config.min_ram_gb * 1_000_000, // Convert from GB to KB
                "KB",
            ),
        ];

        Ok(check_results)
    }
}
