// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};
use url::Url;

const IMPORTED_TRANSACTIONS_FOLDER: &str = "imported_transactions";
const SCRIPTED_TRANSACTIONS_FOLDER: &str = "scripted_transactions";

#[derive(Parser)]
pub struct IndexerCliArgs {
    /// Path to the configuration file with `TransactionGeneratorConfig`.
    #[clap(long)]
    pub config: PathBuf,

    /// Path to the output folder where the generated transactions will be saved.
    #[clap(long)]
    pub output_folder: PathBuf,
}

impl IndexerCliArgs {
    pub async fn run(&self) -> anyhow::Result<()> {
        // Read the configuration file.
        let config_raw = tokio::fs::read_to_string(&self.config)
            .await
            .with_context(|| format!("Failed to read configuration file: {:?}", self.config))?;

        // Parse the configuration.
        let config: TransactionGeneratorConfig = serde_yaml::from_str(&config_raw)
            .with_context(|| format!("Failed to parse configuration file: {:?}", self.config))?;

        // Run the transaction generator.
        config.run(&self.output_folder).await
    }
}

/// Overall configuration for the transaction generator.
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionGeneratorConfig {
    // Configuration for importing transactions from multiple networks.
    pub import_config: TransactionImporterConfig,
    // Configuration for generating transactions from scripts.
    pub script_transaction_generator_config: ScriptTransactionGeneratorConfig,
}

impl TransactionGeneratorConfig {
    pub async fn run(&self, output_path: &Path) -> anyhow::Result<()> {
        let import_config_path = output_path.join(IMPORTED_TRANSACTIONS_FOLDER);
        // Check if the output folder exists.
        if !import_config_path.exists() {
            tokio::fs::create_dir_all(&import_config_path).await?;
        }
        self.import_config
            .run(&import_config_path)
            .await
            .context("Importing transactions failed.")?;

        let script_config_path = output_path.join(SCRIPTED_TRANSACTIONS_FOLDER);
        // Check if the output folder exists.
        if !script_config_path.exists() {
            tokio::fs::create_dir_all(&script_config_path).await?;
        }
        self.script_transaction_generator_config
            .run(&script_config_path)
            .await
            .context("Generating transactions from scripts failed.")
    }
}

/// Configuration for importing transactions from multiple networks.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TransactionImporterConfig {
    // Config is a map from network name to the configuration for that network.
    #[serde(flatten)]
    pub configs: HashMap<String, TransactionImporterPerNetworkConfig>,
}

impl TransactionImporterConfig {
    fn validate(&self) -> anyhow::Result<()> {
        // Validate the configuration. This is to make sure that no output file shares the same name.
        let mut output_files = HashSet::new();
        for (_, network_config) in self.configs.iter() {
            for output_file in network_config.versions_to_import.values() {
                if !output_files.insert(output_file) {
                    return Err(anyhow::anyhow!(
                        "[Transaction Importer] Output file name {} is duplicated",
                        output_file
                    ));
                }
            }
        }
        Ok(())
    }

    pub async fn run(&self, output_path: &Path) -> anyhow::Result<()> {
        // Validate the configuration.
        self.validate()?;

        // Run the transaction importer for each network.
        for (network_name, network_config) in self.configs.iter() {
            network_config.run(output_path).await.context(format!(
                "[Transaction Importer] Failed for network: {}",
                network_name
            ))?;
        }
        Ok(())
    }
}

/// Configuration for importing transactions from a network.
/// This includes the URL of the network, the API key, the version of the transaction to fetch,
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionImporterPerNetworkConfig {
    /// The endpoint of the transaction stream.
    pub transaction_stream_endpoint: Url,
    /// The API key to use for the transaction stream if required.
    pub api_key: Option<String>,
    /// The version of the transaction to fetch and their output file names.
    pub versions_to_import: HashMap<u64, String>,
}

/// Configuration for generating transactions from scripts.
#[derive(Debug, Serialize, Deserialize)]
pub struct ScriptTransactionGeneratorConfig {
    /// List of scripts to run to generate transactions.
    /// Note, each run will be executed in sequence.
    pub runs: Vec<ScriptTransactions>,
}

/// Configuration for generating transactions from a script.
/// `ScriptTransactions` will generate a list of transactions and output if specified.
/// A managed-node will be used to execute the scripts in sequence.
#[derive(Debug, Serialize, Deserialize)]
pub struct ScriptTransactions {
    pub transactions: Vec<ScriptTransaction>,
}

/// A step that can optionally output one transaction.
#[derive(Debug, Serialize, Deserialize)]
pub struct ScriptTransaction {
    pub script_path: PathBuf,
    pub output_name: Option<String>,
    // Optional address to fund the account; if not provided, the default profile address will be used.
    pub fund_address: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_duplicate_output_name() {
        let transaction_generator_config = r#"
            {
                "import_config": {
                    "mainnet": {
                        "transaction_stream_endpoint": "http://mainnet.com",
                        "api_key": "mainnet_api_key",
                        "versions_to_import": {
                            1: "mainnet_v1.json"
                        }
                    },
                    "testnet": {
                        "transaction_stream_endpoint": "http://testnet.com",
                        "api_key": "testnet_api_key",
                        "versions_to_import": {
                            1: "mainnet_v1.json"
                        }
                    }
                },
                "script_transaction_generator_config": {
                    "runs": [
                    ]
                }
            }
        "#;
        let transaction_generator_config: TransactionGeneratorConfig =
            serde_yaml::from_str(transaction_generator_config).unwrap();
        let output_path = PathBuf::from("/tmp");
        let result = transaction_generator_config.run(&output_path).await;
        assert!(result.is_err());
    }
}
