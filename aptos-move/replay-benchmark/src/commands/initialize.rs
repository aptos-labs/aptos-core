// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    commands::{build_debugger, init_logger_and_metrics, RestAPI},
    generator::InputOutputDiffGenerator,
    overrides::OverrideConfig,
    workload::TransactionBlock,
};
use anyhow::{anyhow, bail};
use aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION;
use aptos_logger::Level;
use aptos_types::on_chain_config::FeatureFlag;
use clap::Parser;
use std::path::PathBuf;
use tokio::fs;

#[derive(Parser)]
#[command(about = "Initializes the state for benchmarking, and saves it locally")]
pub struct InitializeCommand {
    #[clap(long, default_value_t = Level::Error)]
    log_level: Level,

    #[clap(flatten)]
    rest_api: RestAPI,

    #[clap(long, help = "Path to the file where the transactions are saved")]
    transactions_file: String,

    #[clap(long, help = "Path to the file where the input states will be saved")]
    inputs_file: String,

    #[clap(
        long,
        num_args = 1..,
        value_delimiter = ' ',
        help = "List of space-separated feature flags to enable, in capital letters. For example, \
                GAS_PAYER_ENABLED or EMIT_FEE_STATEMENT. For the full list of feature flags, see \
                aptos-core/types/src/on_chain_config/aptos_features.rs"
    )]
    enable_features: Vec<FeatureFlag>,

    #[clap(
        long,
        num_args = 1..,
        value_delimiter = ' ',
        help = "List of space-separated feature flags to disable, in capital letters. For \
                example, GAS_PAYER_ENABLED or EMIT_FEE_STATEMENT. For the full list of feature \
                flags, see aptos-core/types/src/on_chain_config/aptos_features.rs"
    )]
    disable_features: Vec<FeatureFlag>,

    #[clap(
        long,
        help = "If set, overrides the gas feature version used by the gas schedule"
    )]
    gas_feature_version: Option<u64>,
}

impl InitializeCommand {
    pub async fn initialize_inputs(self) -> anyhow::Result<()> {
        init_logger_and_metrics(self.log_level);

        if !self
            .enable_features
            .iter()
            .all(|f| !self.disable_features.contains(f))
        {
            bail!("Enabled and disabled feature flags cannot overlap")
        }
        if matches!(self.gas_feature_version, Some(v) if v > LATEST_GAS_FEATURE_VERSION) {
            bail!(
                "Gas feature version must be at most the latest one: {}",
                LATEST_GAS_FEATURE_VERSION
            );
        }

        let bytes = fs::read(PathBuf::from(&self.transactions_file)).await?;
        let txn_blocks: Vec<TransactionBlock> = bcs::from_bytes(&bytes).map_err(|err| {
            anyhow!(
                "Error when deserializing a block of transactions: {:?}",
                err
            )
        })?;

        // TODO:
        //  Right now, only features can be overridden. In the future, we may want to support:
        //      1. Framework code, e.g., to test performance of new natives or compiler,
        //      2. Gas schedule, to track the costs of charging gas or tracking limits.
        //      3. BlockExecutorConfigFromOnchain to experiment with different block cutting based
        //         on gas limits.
        let override_config = OverrideConfig::new(
            self.enable_features,
            self.disable_features,
            self.gas_feature_version,
        );

        let debugger = build_debugger(self.rest_api.rest_endpoint, self.rest_api.api_key)?;
        let inputs =
            InputOutputDiffGenerator::generate(debugger, override_config, txn_blocks).await?;

        let bytes = bcs::to_bytes(&inputs).map_err(|err| {
            anyhow!(
                "Error when serializing inputs for transaction blocks: {:?}",
                err
            )
        })?;
        fs::write(PathBuf::from(&self.inputs_file), &bytes).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_tool() {
        use clap::CommandFactory;
        InitializeCommand::command().debug_assert();
    }
}
