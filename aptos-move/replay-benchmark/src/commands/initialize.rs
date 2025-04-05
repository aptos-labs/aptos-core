// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    commands::{build_debugger, init_logger_and_metrics, RestAPI},
    generator::InputOutputDiffGenerator,
    overrides::OverrideConfig,
    workload::{BlockIndex, APTOS_COMMONS, APTOS_PACKAGES_DIR_NAMES},
};
use anyhow::anyhow;
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

    #[clap(long, help = "Name of the file where the transactions are saved")]
    transactions_file: String,

    #[clap(long, help = "Name of the file where the input states will be saved")]
    inputs_file: String,

    #[clap(
        long,
        help = "Path to the folder where transactions_file and inputs_file are saved"
    )]
    input_dir: Option<PathBuf>,

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

    #[clap(
        long,
        num_args = 1..,
        value_delimiter = ' ',
        help = "List of space-separated paths to compiled / built packages with Move code"
    )]
    override_packages: Vec<PathBuf>,

    #[clap(
        long,
        help = "List of space-separated experimental features to enable for compiler"
    )]
    experimental_features: Vec<String>,

    #[clap(
        long,
        help = "Whether transaction data contains source code information"
    )]
    with_source_code: bool,

    #[clap(long, help = "Whether recompile aptos-framework code")]
    recompile_aptos_framework: bool,
}

impl InitializeCommand {
    pub async fn initialize_inputs(mut self) -> anyhow::Result<()> {
        init_logger_and_metrics(self.log_level);

        let input = if let Some(path) = self.input_dir {
            path
        } else {
            PathBuf::from(".")
        };
        let bytes = fs::read(input.join(self.transactions_file)).await?;

        let mut txn_blocks = vec![];

        if self.with_source_code {
            let block_index: Vec<BlockIndex> = bcs::from_bytes(&bytes).map_err(|err| {
                anyhow!(
                    "Error when deserializing a block of transactions: {:?}",
                    err
                )
            })?;

            let mut txn_blocks = Vec::new();
            for block_index in block_index {
                for (_, package_info) in block_index.package_info {
                    self.override_packages
                        .push(input.join(format!("{}", package_info)));
                }
                txn_blocks.push(block_index.transaction_block);
            }
            if self.recompile_aptos_framework {
                for name in APTOS_PACKAGES_DIR_NAMES {
                    self.override_packages
                        .push(input.join(APTOS_COMMONS).join(name));
                }
            }
        } else {
            txn_blocks = bcs::from_bytes(&bytes).map_err(|err| {
                anyhow!(
                    "Error when deserializing a block of transactions: {:?}",
                    err
                )
            })?;
        }

        // TODO:
        //   1. Override gas schedule, to track the costs of charging gas or tracking limits.
        //   2. BlockExecutorConfigFromOnchain to experiment with different block cutting based
        //      on gas limits?.
        //   3. Build options for package overrides.
        let override_config = OverrideConfig::new(
            self.enable_features,
            self.disable_features,
            self.gas_feature_version,
            self.override_packages,
            self.experimental_features,
        )?;

        let debugger = build_debugger(self.rest_api.rest_endpoint, self.rest_api.api_key)?;
        let inputs =
            InputOutputDiffGenerator::generate(debugger, override_config, txn_blocks).await?;

        let bytes = bcs::to_bytes(&inputs).map_err(|err| {
            anyhow!(
                "Error when serializing inputs for transaction blocks: {:?}",
                err
            )
        })?;
        fs::write(input.join(&self.inputs_file), &bytes).await?;
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
