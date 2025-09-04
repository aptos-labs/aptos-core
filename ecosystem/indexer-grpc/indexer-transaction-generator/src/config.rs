// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    accont_manager::AccountManager,
    managed_node::ManagedNode,
    transaction_code_builder::{
        TransactionCodeBuilder, IMPORTED_DEVNET_TXNS, IMPORTED_MAINNET_TXNS, IMPORTED_TESTNET_TXNS,
        SCRIPTED_TRANSACTIONS_TXNS,
    },
};
use anyhow::Context;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fmt, fs,
    path::{Path, PathBuf},
    str::FromStr,
};
use url::Url;

const SCRIPTED_TRANSACTIONS_FOLDER: &str = "scripted_transactions";
const MOVE_SCRIPTS_FOLDER: &str = "move_fixtures";
const IMPORTED_TRANSACTION_CONFIG_FILE: &str = "imported_transactions.yaml";
const ACCOUNT_MANAGER_FILE_NAME: &str = "testing_accounts.yaml";

#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    Import,
    Script,
}

// Implement Display for Mode
impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self) // Use the debug representation
    }
}

impl FromStr for Mode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "import" => Ok(Mode::Import),
            "script" => Ok(Mode::Script),
            _ => Err(anyhow::anyhow!("Invalid mode: {}", s)),
        }
    }
}

#[derive(Parser)]
pub struct IndexerCliArgs {
    /// Path to the testing folder, which includes:
    /// - The configuration file for importing transactions: `imported_transactions.yaml`.
    /// - The folder containing the Move scripts to generate transactions: `move_fixtures`.
    /// - The file containing the accounts for testing: `testing_accounts.yaml`.
    #[clap(long)]
    pub testing_folder: PathBuf,

    /// Path to the output folder where the generated transactions will be saved.
    #[clap(long)]
    pub output_folder: PathBuf,

    /// Mode of operation for the indexer.
    #[clap(long, default_value_t = Mode::Import, value_parser = Mode::from_str)]
    pub mode: Mode,

    /// Leave this blank if want all the networks
    #[clap(long)]
    pub network: Option<String>,
}

impl IndexerCliArgs {
    pub async fn run(&self) -> anyhow::Result<()> {
        let output_folder =
            convert_relative_path_to_absolute_path(&self.output_folder).join("json_transactions");
        if !output_folder.exists() {
            tokio::fs::create_dir_all(&output_folder).await?;
        }
        let testing_folder = convert_relative_path_to_absolute_path(&self.testing_folder);

        // Determine the behavior based on the flags
        match self.mode {
            Mode::Import => {
                // Run the transaction importer.
                println!("Running the transaction importer.");

                let imported_transactions_config_path =
                    testing_folder.join(IMPORTED_TRANSACTION_CONFIG_FILE);

                if imported_transactions_config_path.exists() {
                    let imported_transactions_config_raw: String =
                        tokio::fs::read_to_string(&imported_transactions_config_path).await?;
                    let imported_transactions_config: TransactionImporterConfig =
                        serde_yaml::from_str(&imported_transactions_config_raw)?;

                    imported_transactions_config
                        .validate_and_run(&output_folder, self.network.clone())
                        .await
                        .context("Importing transactions failed.")?;
                }
            },
            Mode::Script => {
                // Run the script transaction generator.
                let script_transactions_output_folder =
                    output_folder.join(SCRIPTED_TRANSACTIONS_FOLDER);
                let move_folder_path = testing_folder.join(MOVE_SCRIPTS_FOLDER);
                // If the move fixtures folder does not exist, skip the script transaction generator.
                if !move_folder_path.exists() {
                    return Ok(());
                }
                if !script_transactions_output_folder.exists() {
                    tokio::fs::create_dir_all(&script_transactions_output_folder).await?;
                }
                // 1. Validate.
                // Scan all yaml files in the move folder path.
                let mut script_transactions_vec: Vec<(String, ScriptTransactions)> = vec![];
                let move_files = std::fs::read_dir(&move_folder_path)?;
                let mut used_sender_addresses: HashSet<String> = HashSet::new();
                for entry in move_files {
                    let entry = entry?;
                    // entry has to be a file.
                    if !entry.file_type()?.is_file() {
                        continue;
                    }
                    let path = entry.path();
                    if path.extension().unwrap_or_default() == "yaml" {
                        let file_name = path.file_name().unwrap().to_str().unwrap();
                        let script_transactions_raw: String =
                            tokio::fs::read_to_string(&path).await?;
                        let script_transactions: ScriptTransactions =
                            serde_yaml::from_str(&script_transactions_raw)?;

                        let new_senders: HashSet<String> = script_transactions
                            .transactions
                            .iter()
                            .map(|txn| txn.sender_address.clone())
                            .collect();
                        // Check if any new sender is already used
                        if new_senders
                            .iter()
                            .any(|sender| used_sender_addresses.contains(sender))
                        {
                            return Err(anyhow::anyhow!(
                                "[Script Transaction Generator] Sender address in file `{}` is already being used",
                                file_name
                            ));
                        }
                        used_sender_addresses.extend(new_senders);
                        script_transactions_vec.push((file_name.to_string(), script_transactions));
                    }
                }
                // Validate the configuration.
                let mut output_script_transactions_set = HashSet::new();
                for (file_name, script_transactions) in script_transactions_vec.iter() {
                    if script_transactions.transactions.is_empty() {
                        return Err(anyhow::anyhow!(
                            "[Script Transaction Generator] No transactions found in file `{}`",
                            file_name
                        ));
                    }
                    for script_transaction in script_transactions.transactions.iter() {
                        if let Some(output_name) = &script_transaction.output_name {
                            if !output_script_transactions_set.insert(output_name.clone()) {
                                return Err(anyhow::anyhow!(
                                    "[Script Transaction Generator] Output file name `{}` is duplicated in file `{}`",
                                    output_name.clone(),
                                    file_name
                                    ));
                            }
                        }
                    }
                }
                // Run each config.
                let account_manager_file_path = testing_folder.join(ACCOUNT_MANAGER_FILE_NAME);
                let mut account_manager = AccountManager::load(&account_manager_file_path).await?;
                let mut managed_node = ManagedNode::start(None, None).await?;
                for (file_name, script_transactions) in script_transactions_vec {
                    script_transactions
                        .run(
                            &move_folder_path,
                            &script_transactions_output_folder,
                            &mut account_manager,
                        )
                        .await
                        .context(format!(
                            "Failed to generate script transaction for file `{}`",
                            file_name
                        ))?;
                }
                // Stop the localnet.
                managed_node.stop().await?;
            },
        }

        // Using the builder pattern to construct the code
        let code = TransactionCodeBuilder::new()
            .add_license_in_comments()
            .add_directory(
                output_folder.join(IMPORTED_MAINNET_TXNS).as_path(),
                IMPORTED_MAINNET_TXNS,
                false,
            )
            .add_directory(
                output_folder.join(IMPORTED_TESTNET_TXNS).as_path(),
                IMPORTED_TESTNET_TXNS,
                false,
            )
            .add_directory(
                output_folder.join(IMPORTED_DEVNET_TXNS).as_path(),
                IMPORTED_DEVNET_TXNS,
                false,
            )
            .add_directory(
                output_folder.join(SCRIPTED_TRANSACTIONS_TXNS).as_path(),
                SCRIPTED_TRANSACTIONS_TXNS,
                true,
            )
            .add_transaction_name_function()
            .build();

        let dest_path = output_folder.join("generated_transactions.rs");

        match fs::write(dest_path.clone(), code) {
            Ok(_) => {
                println!("Successfully generated the transactions code.");
                Ok(())
            },
            Err(e) => Err(anyhow::anyhow!(
                "Failed to generate the transactions code for dest_path:{:?}, {:?}",
                dest_path,
                e
            )),
        }
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

    pub async fn validate_and_run(
        &self,
        output_path: &Path,
        network: Option<String>,
    ) -> anyhow::Result<()> {
        // Validate the configuration.
        self.validate()?;

        // Run the transaction importer for each network.
        for (network_name, network_config) in self.configs.iter() {
            // If network is specified, only run the transaction importer for the specified network.
            if let Some(network) = &network {
                if network != network_name {
                    continue;
                }
            }
            // Modify the output path by appending the network name to the base path
            let modified_output_path = match network_name.as_str() {
                "mainnet" => output_path.join(IMPORTED_MAINNET_TXNS),
                "testnet" => output_path.join(IMPORTED_TESTNET_TXNS),
                "devnet" => output_path.join(IMPORTED_DEVNET_TXNS),
                _ => {
                    return Err(anyhow::anyhow!(
                        "[Transaction Importer] Unknown network: {}",
                        network_name
                    ));
                },
            };

            tokio::fs::create_dir_all(&modified_output_path)
                .await
                .context(format!(
                    "[Transaction Importer] Failed to create output directory for network: {}",
                    network_name
                ))?;

            network_config
                .run(modified_output_path.as_path())
                .await
                .context(format!(
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
    // Fund the address and execute the script with the account.
    pub sender_address: String,
}

/// Convert relative path to absolute path.
fn convert_relative_path_to_absolute_path(path: &Path) -> PathBuf {
    if path.is_relative() {
        let current_dir = std::env::current_dir().unwrap();
        current_dir.join(path)
    } else {
        path.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_import_transactions_duplicate_output_name() {
        let importing_transactions_config = r#"
            {
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
            }
        "#;
        let transaction_generator_config: TransactionImporterConfig =
            serde_yaml::from_str(importing_transactions_config).unwrap();
        // create a temporary folder for the output.
        let tempfile = tempfile::tempdir().unwrap();
        let result = transaction_generator_config
            .validate_and_run(tempfile.path(), None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_script_transactions_duplicate_output_name() {
        // Create a temporary folder for the move scripts.
        let tempfile = tempfile::tempdir().unwrap();
        let output_folder = tempfile.path().join("output");
        tokio::fs::create_dir(&output_folder).await.unwrap();
        let move_folder_path = tempfile.path().join("move_fixtures");
        tokio::fs::create_dir(&move_folder_path).await.unwrap();

        let first_script_transactions = r#"
            transactions:
              - script_path: "simple_script_1"
                output_name: "output.json"
        "#;
        let second_script_transactions = r#"
            transactions:
              - script_path: "simple_script_2"
                output_name: "output.json"
        "#;
        let first_script_transactions_path =
            move_folder_path.join("first_script_transactions.yaml");
        let second_script_transactions_path =
            move_folder_path.join("second_script_transactions.yaml");
        tokio::fs::write(&first_script_transactions_path, first_script_transactions)
            .await
            .unwrap();
        tokio::fs::write(&second_script_transactions_path, second_script_transactions)
            .await
            .unwrap();
        let indexer_cli_args = IndexerCliArgs {
            testing_folder: tempfile.path().to_path_buf(),
            output_folder,
            mode: Mode::Script,
            network: None,
        };
        let result = indexer_cli_args.run().await;
        assert!(result.is_err());
    }
}
