// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{CheckResult, Checker, CheckerError, CommonCheckerConfig};
use crate::{
    get_provider,
    provider::{api_index::ApiIndexProvider, Provider, ProviderCollection},
};
use anyhow::Result;
use velor_rest_client::{velor_api_types::TransactionData, Client as VelorRestClient};
use serde::{Deserialize, Serialize};
use std::cmp::{max, min};

const TRANSACTIONS_ENDPOINT: &str = "/transactions/by_version";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionCorrectnessCheckerConfig {
    #[serde(flatten)]
    pub common: CommonCheckerConfig,
}

#[derive(Clone, Debug)]
pub struct TransactionCorrectnessChecker {
    config: TransactionCorrectnessCheckerConfig,
}

impl TransactionCorrectnessChecker {
    pub fn new(config: TransactionCorrectnessCheckerConfig) -> Self {
        Self { config }
    }

    /// Fetch a transaction by version and return it.
    async fn get_transaction_by_version(
        client: &VelorRestClient,
        version: u64,
        node_name: &str,
    ) -> Result<TransactionData, CheckerError> {
        Ok(client
            .get_transaction_by_version_bcs(version)
            .await
            .map_err(|e| {
                CheckerError::NonRetryableEndpointError(
                    TRANSACTIONS_ENDPOINT,
                    anyhow::Error::from(e).context(format!(
                        "The {} node API failed to return the requested transaction at version: {}",
                        node_name, version
                    )),
                )
            })?
            .into_inner())
    }

    /// Helper to get the accumulator root hash from an on chain transaction
    /// as returned by the API.
    fn unwrap_accumulator_root_hash(
        transaction_data: &TransactionData,
    ) -> Result<&velor_crypto::HashValue, CheckerError> {
        match transaction_data {
            TransactionData::OnChain(on_chain) => Ok(&on_chain.accumulator_root_hash),
            wildcard => Err(CheckerError::NonRetryableEndpointError(
                TRANSACTIONS_ENDPOINT,
                anyhow::anyhow!(
                    "The API unexpectedly returned a transaction that was not an on-chain transaction: {:?}",
                    wildcard
                ),
            ))
        }
    }
}

#[async_trait::async_trait]
impl Checker for TransactionCorrectnessChecker {
    /// Assert that the target node can produce the same transaction that the
    /// baseline produced after a delay. We confirm that the transactions are
    /// same by looking at the version.
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

        let oldest_baseline_version = baseline_api_index_provider
            .provide()
            .await?
            .oldest_ledger_version
            .0;
        let oldest_target_version = match target_api_index_provider.provide().await {
            Ok(response) => response.oldest_ledger_version.0,
            Err(err) => {
                return Ok(vec![Self::build_result(
                    "Failed to determine oldest ledger version of your node".to_string(),
                    0,
                    format!(
                        "There was an error querying your node's API (1st time): {:#}",
                        err
                    ),
                )]);
            },
        };

        tokio::time::sleep(target_api_index_provider.config.common.check_delay()).await;

        let latest_baseline_version = baseline_api_index_provider
            .provide()
            .await?
            .ledger_version
            .0;
        let latest_target_version = match target_api_index_provider.provide().await {
            Ok(response) => response.ledger_version.0,
            Err(err) => {
                return Ok(vec![Self::build_result(
                    "Failed to determine latest ledger version of your node".to_string(),
                    0,
                    format!(
                        "There was an error querying your node's API (2nd time): {:#}",
                        err
                    ),
                )]);
            },
        };

        // Get the oldest ledger version between the two nodes.
        let oldest_shared_version = max(oldest_baseline_version, oldest_target_version);

        // Get the least up to date latest ledger version between the two nodes.
        let latest_shared_version = min(latest_baseline_version, latest_target_version);

        // Ensure that there is a window between the oldest shared version and
        // latest shared version. If there is not, it will not be possible to
        // pull a transaction that both nodes have.
        if oldest_shared_version > latest_shared_version {
            return Ok(vec![Self::build_result(
                "Unable to pull transaction from both nodes".to_string(),
                0,
                format!(
                    "We were unable to find a ledger version window between \
                        the baseline and target nodes. The oldest and latest \
                        ledger versions on the baseline node are {} and {}. \
                        The oldest and latest ledger versions on the target \
                        node are {} and {}. This means your API cannot return \
                        a transaction that the baseline has for us to verify. \
                        Likely this means your node is too out of sync with \
                        the network, but it could also indicate an \
                        over-aggressive pruner.",
                    oldest_baseline_version,
                    latest_baseline_version,
                    oldest_target_version,
                    latest_target_version,
                ),
            )]);
        }

        // Select a version in the middle of shared oldest and latest version.
        let middle_shared_version =
            (oldest_shared_version.saturating_add(latest_shared_version)) / 2;

        // We've asserted that both nodes are sufficiently up to date relative
        // to each other, we should be able to pull the same transaction from
        // both nodes.

        let middle_baseline_transaction = Self::get_transaction_by_version(
            &baseline_api_index_provider.client,
            middle_shared_version,
            "baseline",
        )
        .await?;
        let middle_baseline_accumulator_root_hash =
            Self::unwrap_accumulator_root_hash(&middle_baseline_transaction)?;

        let evaluation = match Self::get_transaction_by_version(
            &target_api_index_provider.client,
            middle_shared_version,
            "latest",
        )
        .await
        {
            Ok(middle_target_transaction) => {
                match Self::unwrap_accumulator_root_hash(&middle_target_transaction) {
                    Ok(middle_target_accumulator_root_hash) => {
                        if middle_baseline_accumulator_root_hash
                            == middle_target_accumulator_root_hash
                        {
                            Self::build_result(
                                "Target node produced valid recent transaction".to_string(),
                                100,
                                format!(
                                    "We were able to pull the same transaction (version: {}) \
                                    from both your node and the baseline node. Great! This \
                                    implies that your node is returning valid transaction data.",
                                    middle_shared_version,
                                ),
                            )
                        } else {
                            Self::build_result(
                                "Target node produced recent transaction, but it was invalid"
                                    .to_string(),
                                0,
                                format!(
                                    "We were able to pull the same transaction (version: {}) \
                                    from both your node and the baseline node. However, the \
                                    transaction was invalid compared to the baseline as the \
                                    accumulator root hash of the transaction ({}) was different \
                                    compared to the baseline ({}).",
                                    middle_shared_version,
                                    middle_target_accumulator_root_hash,
                                    middle_baseline_accumulator_root_hash,
                                ),
                            )
                        }
                    },
                    Err(error) => Self::build_result(
                        "Target node produced recent transaction, but it was missing metadata"
                            .to_string(),
                        10,
                        format!(
                            "We were able to pull the same transaction (version: {}) \
                            from both your node and the baseline node. However, the \
                            the transaction was missing metadata such as the version, \
                            accumulator root hash, etc. Error: {}",
                            middle_shared_version, error,
                        ),
                    ),
                }
            },
            Err(error) => Self::build_result(
                "Target node failed to produce transaction".to_string(),
                25,
                format!(
                    "The target node claims it has transactions between versions {} and {}, \
                    but it was unable to return the transaction with version {}. This implies \
                    something is wrong with your node's API. Error: {}",
                    oldest_target_version, latest_target_version, middle_shared_version, error,
                ),
            ),
        };

        Ok(vec![evaluation])
    }
}
