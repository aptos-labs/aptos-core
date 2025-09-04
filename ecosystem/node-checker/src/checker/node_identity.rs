// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{CheckResult, Checker, CheckerError, CommonCheckerConfig};
use crate::{
    get_provider,
    provider::{api_index::ApiIndexProvider, Provider, ProviderCollection},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodeIdentityCheckerConfig {
    #[serde(flatten)]
    pub common: CommonCheckerConfig,
}

#[derive(Debug)]
pub struct NodeIdentityChecker {
    config: NodeIdentityCheckerConfig,
}

impl NodeIdentityChecker {
    pub fn new(config: NodeIdentityCheckerConfig) -> Self {
        Self { config }
    }

    fn help_build_check_result<T: Display + PartialEq>(
        &self,
        baseline_value: T,
        target_value: T,
        attribute_str: &str,
    ) -> CheckResult {
        let (headline, score, explanation) = if baseline_value == target_value {
            (
                format!("{} reported by baseline and target match", attribute_str),
                100,
                format!(
                    "The node under investigation reported the same {} {} \
                as is reported by the baseline node.",
                    attribute_str, target_value
                ),
            )
        } else {
            (
                format!(
                    "{} reported by the target does not match the baseline",
                    attribute_str
                ),
                0,
                format!(
                    "The node under investigation reported the {} {} while the \
                baseline reported {}. These values should match. Confirm that \
                the baseline you're using is appropriate for the node you're testing.",
                    attribute_str, target_value, baseline_value
                ),
            )
        };
        Self::build_result(headline, score, explanation)
    }
}

#[async_trait::async_trait]
impl Checker for NodeIdentityChecker {
    /// Assert that the node identity (role type and chain ID) of the two nodes match.
    async fn check(
        &self,
        providers: &ProviderCollection,
    ) -> Result<Vec<CheckResult>, CheckerError> {
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

        // We just let this error turn into a CheckerError since we want to
        // return actual errors in the case of a failure in querying the baseline.
        let baseline_response = baseline_api_index_provider.provide().await?;

        // As for the target, we return a CheckResult if something fails here.
        let target_response = match target_api_index_provider.provide().await {
            Ok(response) => response,
            Err(err) => {
                return Ok(vec![Self::build_result(
                    "Failed to check identity of your node".to_string(),
                    0,
                    format!("There was an error querying your node's API: {:#}", err),
                )]);
            },
        };

        let check_results = vec![
            self.help_build_check_result(
                baseline_response.chain_id,
                target_response.chain_id,
                "Chain ID",
            ),
            self.help_build_check_result(
                baseline_response.node_role,
                target_response.node_role,
                "Role Type",
            ),
        ];

        Ok(check_results)
    }
}
