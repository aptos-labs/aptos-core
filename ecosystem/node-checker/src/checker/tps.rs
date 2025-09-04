// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{CheckResult, Checker, CheckerError, CommonCheckerConfig};
use crate::{
    get_provider,
    provider::{api_index::ApiIndexProvider, Provider, ProviderCollection},
};
use anyhow::{Context, Result};
use velor_sdk::types::chain_id::ChainId;
use velor_transaction_emitter_lib::{
    emit_transactions_with_cluster, Cluster, ClusterArgs, CoinSourceArgs, EmitArgs,
};
use velor_transaction_workloads_lib::args::EmitWorkloadArgs;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

const NODE_REQUIREMENTS_LINK: &str =
    "https://velor.dev/nodes/validator-node/operator/node-requirements";

#[derive(Debug, ThisError)]
pub enum TpsCheckerError {
    /// Failed to build the cluster for the transaction emitter. This
    /// represents an internal logic error.
    #[error("Error building the transaction emitter cluster: {0}")]
    BuildClusterError(anyhow::Error),

    /// There was an error from the transaction emitter that we suspect
    /// was our own fault, not the fault of the target node.
    #[error("Error from within the transaction emitter: {0}")]
    TransactionEmitterError(anyhow::Error),

    /// We return this error if the transaction emitter failed to emit
    /// more transactions than the configured min TPS. This implies
    /// a configuration error.
    #[error("The transaction emitter only submitted {0} TPS but the minimum TPS requirement is {1}, this implies a configuration problem with the NHC instance")]
    InsufficientSubmittedTransactionsError(u64, u64),
}

impl TpsCheckerError {
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::BuildClusterError(_) => false,
            Self::TransactionEmitterError(_) => true,
            Self::InsufficientSubmittedTransactionsError(_, _) => false,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TpsCheckerConfig {
    #[serde(flatten)]
    pub common: CommonCheckerConfig,

    #[serde(flatten)]
    pub emit_config: EmitArgs,

    #[serde(flatten)]
    pub emit_workload_configs: EmitWorkloadArgs,

    // Ed25519PrivateKey, either on the CLI or from a file, for minting coins.
    // We choose to take this in in the baseline config because we can't
    // securely transmit this at request time over the wire.
    #[serde(flatten)]
    pub coin_source_args: CoinSourceArgs,

    /// The minimum TPS required to pass the test.
    pub minimum_tps: u64,

    /// The number of times to repeat the target. This influences thread
    /// count and rest client count.
    #[serde(default = "TpsCheckerConfig::default_repeat_target_count")]
    pub repeat_target_count: usize,
}

impl TpsCheckerConfig {
    fn default_repeat_target_count() -> usize {
        1
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct TpsChecker {
    config: TpsCheckerConfig,
}

impl TpsChecker {
    pub fn new(config: TpsCheckerConfig) -> Result<Self> {
        // Confirm that we actually have a mint key.
        config
            .coin_source_args
            .get_private_key()
            .context("TpsChecker: No mint key provided")?;
        Ok(Self { config })
    }
}

#[async_trait::async_trait]
impl Checker for TpsChecker {
    // You'll see that we're using the baseline chain ID here. This is okay
    // because at this point we've already asserted the baseline and target
    // have the same chain id.

    /// This test runs a TPS (transactions per second) evaluation on the target
    /// node, in which it passes if it meets some preconfigured minimum.
    async fn check(
        &self,
        providers: &ProviderCollection,
    ) -> Result<Vec<CheckResult>, CheckerError> {
        let target_api_index_provider = get_provider!(
            providers.target_api_index_provider,
            self.config.common.required,
            ApiIndexProvider
        );

        let target_url = target_api_index_provider.client.build_path("/").unwrap();
        let chain_id = match target_api_index_provider.provide().await {
            Ok(response) => ChainId::new(response.chain_id),
            Err(err) => {
                return Ok(vec![Self::build_result(
                    "Failed to get chain ID of your node".to_string(),
                    0,
                    format!("There was an error querying your node's API: {:#}", err),
                )]);
            },
        };

        let cluster_config = ClusterArgs {
            targets: Some(vec![target_url; self.config.repeat_target_count]),
            targets_file: None,
            coin_source_args: self.config.coin_source_args.clone(),
            chain_id: Some(chain_id),
            node_api_key: None,
        };
        let cluster = Cluster::try_from_cluster_args(&cluster_config)
            .await
            .map_err(TpsCheckerError::BuildClusterError)?;

        let stats = emit_transactions_with_cluster(
            &cluster,
            &self.config.emit_config,
            self.config
                .emit_workload_configs
                .args_to_transaction_mix_per_phase(),
        )
        .await
        .map_err(TpsCheckerError::TransactionEmitterError)?;

        // AKA stats per second.
        let rate = stats.rate();

        if rate.submitted < (self.config.minimum_tps as f64) {
            return Err(TpsCheckerError::InsufficientSubmittedTransactionsError(
                rate.submitted as u64,
                self.config.minimum_tps,
            )
            .into());
        }

        let mut description = format!("The minimum TPS (transactions per second) \
            required of nodes is {}, your node hit: {} (out of {} transactions submitted per second).", self.config.minimum_tps, rate.committed, rate.submitted);
        let evaluation_result = if rate.committed >= (self.config.minimum_tps as f64) {
            if stats.committed == stats.submitted {
                description.push_str(
                    " Your node could theoretically hit \
                even higher TPS, the evaluation suite only tests to check \
                your node meets the minimum requirements.",
                );
            }
            Self::build_result(
                "Transaction processing speed is sufficient".to_string(),
                100,
                description,
            )
        } else {
            description.push_str(
                " This implies that the hardware you're \
            using to run your node isn't powerful enough, please see the attached link",
            );
            Self::build_result(
                "Transaction processing speed is too low".to_string(),
                0,
                description,
            )
            .links(vec![NODE_REQUIREMENTS_LINK.to_string()])
        };

        Ok(vec![evaluation_result])
    }
}
