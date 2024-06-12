// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use which::which;

/// The timeout in seconds to wait for the node to be ready.
const NODE_READY_TIMEOUT_IN_SECS: u64 = 30;
const DEFAULT_PROFILE_NAME: &str = "default";
const APTOS_CLI_BINARY_NAME: &str = "aptos";
const LOCAL_FAUCET_URL: &str = "http://localhost:8081";

/// The Move executor is the interface to interact with the Aptos CLI.
/// TODO: finish the comment.

#[derive(Debug)]
pub struct MoveExecutor {
    /// The url of the node API endpoint to send transactions to.
    /// Default is localnet, `http://localhost:8080`.
    /// TODO: change to `Url` type.
    node_api_endpoint: String,
}

// The config file for the Aptos CLI.
#[derive(Debug, Serialize, Deserialize)]
struct AptosCliConfig {
    profiles: Profiles,
}

#[derive(Debug, Serialize, Deserialize)]
struct Profiles {
    default: Profile,
    // Ignores the rest of the profiles.
}

#[derive(Debug, Serialize, Deserialize)]
struct Profile {
    amount_to_fund: u64,
    is_setup: bool,
}

// The config toml for the Move module.
#[derive(Debug, Serialize, Deserialize)]
struct MoveConfig {
    package: Package,
    // Ignores the rest of the fields.
}

#[derive(Debug, Serialize, Deserialize)]
struct Package {
    name: String,
    // Ignores the rest of the fields.
}

// The outcome of the Aptos Cli execution.
#[derive(Debug, Serialize, Deserialize)]
struct AptosCliOutput {
    #[serde(rename = "Result")]
    result: AptosCliResult,
}

#[derive(Debug, Serialize, Deserialize)]
struct AptosCliResult {
    version: u64,
    // Ignores the rest of the fields.
}

impl MoveExecutor {
    /// Create a new Move executor.
    pub fn new(node_api_endpoint: &str) -> anyhow::Result<Self> {
        // Check if the aptos binary is installed.
        let _ = which(APTOS_CLI_BINARY_NAME).context(format!(
            "{} binary not found in PATH.",
            APTOS_CLI_BINARY_NAME
        ))?;
        Ok(Self {
            node_api_endpoint: node_api_endpoint.to_string(),
        })
    }

    /// Execute the Move module and return the transaction version if it's not setup step.
    /// The Move module folder should contain a `.aptos/config.toml` file.
    pub async fn execute(&self, move_module_folder: &PathBuf) -> anyhow::Result<Option<u64>> {
        // Wait for the node to be ready.
        self.wait_if_not_ready()
            .await
            .context("Node is not ready")?;

        // Change directory to the Move module folder.
        std::env::set_current_dir(move_module_folder).context(format!(
            "Failed to set current working directory to {:?}",
            move_module_folder
        ))?;

        // Load the config.
        let config = self
            .load_config(move_module_folder.clone())
            .await
            .context(format!(
                "Failed to load the config for module {:?}",
                move_module_folder
            ))?;

        // Fund the account.
        self.fund_account(config.amount_to_fund)
            .await
            .context("Failed to fund the account")?;

        if config.is_setup {
            // Publish the Move module.
            let mut publish_cmd = tokio::process::Command::new(APTOS_CLI_BINARY_NAME);
            publish_cmd.arg("move").arg("publish").arg("--assume-yes");
            let output = publish_cmd
                .output()
                .await
                .context(format!("Failed to publish module {:?}", move_module_folder))?;
            if !output.status.success() {
                return Err(anyhow::anyhow!(
                    "Failed to publish Move module: {:?}",
                    output
                ));
            }
            Ok(None)
        } else {
            // Compile the Move script first.
            let mut compile_cmd = tokio::process::Command::new(APTOS_CLI_BINARY_NAME);
            compile_cmd.arg("move").arg("compile");
            let output = compile_cmd
                .output()
                .await
                .context(format!("Failed to run script {:?}", move_module_folder))?;
            if !output.status.success() {
                return Err(anyhow::anyhow!(
                    "Failed to compile Move script: {:?}",
                    output
                ));
            }
            // Parse the Move.toml file to get the module name.
            let move_toml_path = move_module_folder.join("Move.toml");
            let move_toml_content = tokio::fs::read_to_string(&move_toml_path).await?;
            let move_config: MoveConfig = toml::from_str(&move_toml_content).context(format!(
                "Failed to parse Move.toml file at {:?}",
                move_toml_path
            ))?;
            let package_name = move_config.package.name;

            // Run the compiled Move Script.
            let mut run_cmd = tokio::process::Command::new(APTOS_CLI_BINARY_NAME);
            run_cmd
                .arg("move")
                .arg("run-script")
                .arg("--compiled-script-path")
                .arg(format!("build/{}/bytecode_scripts/main.mv", package_name))
                .arg("--assume-yes");
            match run_cmd.output().await {
                Ok(output) => {
                    if !output.status.success() {
                        return Err(anyhow::anyhow!("Failed to run Move script: {:?}", output));
                    }
                    let aptos_cli_output: AptosCliOutput =
                        match serde_json::from_slice(&output.stdout)
                            .context("Failed to parse aptos output.")
                        {
                            Ok(output) => output,
                            Err(e) => {
                                // This means the
                                return Err(anyhow::anyhow!(
                                    "Failed to parse aptos output: {:?}",
                                    e
                                ));
                            },
                        };
                    Ok(Some(aptos_cli_output.result.version))
                },
                Err(e) => Err(anyhow::anyhow!("Failed to run Move script: {:?}", e)),
            }
        }
    }

    // Internal methods.

    /// Wait for the node to be ready.
    async fn wait_if_not_ready(&self) -> anyhow::Result<()> {
        let client = Client::new();
        let start = std::time::Instant::now();
        loop {
            match client.get(&self.node_api_endpoint).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        break;
                    }
                },
                Err(_) => {
                    if start.elapsed().as_secs() > NODE_READY_TIMEOUT_IN_SECS {
                        return Err(anyhow::anyhow!(
                            "Node is not ready after {} seconds.",
                            NODE_READY_TIMEOUT_IN_SECS
                        ));
                    }
                },
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        Ok(())
    }

    /// Load the config from .aptos/config.toml
    async fn load_config(&self, move_module_folder: PathBuf) -> anyhow::Result<Profile> {
        let config_path = move_module_folder.join(".aptos/config.yaml");
        let config_content = tokio::fs::read_to_string(&config_path).await?;
        let config: AptosCliConfig = serde_yaml::from_str(&config_content)
            .context(format!("Failed to parse config file at {:?}.", config_path))?;
        Ok(config.profiles.default)
    }

    /// Fund the account with the faucet.
    async fn fund_account(&self, amount: u64) -> anyhow::Result<()> {
        let mut fund_account_cmd = tokio::process::Command::new(APTOS_CLI_BINARY_NAME);
        fund_account_cmd
            .arg("account")
            .arg("fund-with-faucet")
            .arg("--amount")
            .arg(amount.to_string())
            .arg("--profile")
            .arg(DEFAULT_PROFILE_NAME)
            .arg("--faucet-url")
            .arg(LOCAL_FAUCET_URL);
        let output = fund_account_cmd.output().await?;
        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to fund account: {:?}", output));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_config_parsing() {
        let raw_yaml_content = r#"---
            profiles:
                default:
                    amount_to_fund: 100000000
                    is_setup: true
                testing:
                    private_key: "0x123"
                    public_key: "0x123"
                    account: 456
                    rest_url: "http://localhost:8080"
                    faucet_url: "http://localhost:8081"
                    # Testing Related
                    amount_to_fund: 100000001
        "#;
        let config: AptosCliConfig = serde_yaml::from_str(raw_yaml_content).unwrap();
        assert_eq!(config.profiles.default.amount_to_fund, 100000000);
        assert!(config.profiles.default.is_setup);
    }

    #[tokio::test]
    async fn test_aptos_cli_output_parsing() {
        let raw_json_content = r#"{
            "Result": {
                "transaction_hash": "0x568e3024d8eca50629ccb01937275506f8dfc848d3dfb1f4701690cc9b412b33",
                "gas_used": 3,
                "gas_unit_price": 100,
                "sender": "49cbfc8d9fc24297df19a47cc05c2fb194d1bebf15b85124bf8f346ffde8eccc",
                "sequence_number": 0,
                "success": true,
                "timestamp_us": 1718143556686324,
                "version": 79373359,
                "vm_status": "Executed successfully"
            }
        }"#;
        let output: AptosCliOutput = serde_json::from_str(raw_json_content).unwrap();
        assert_eq!(output.result.version, 79373359);
    }

    #[tokio::test]
    async fn test_move_config_parsing() {
        let raw_toml_content = r#"
            [package]
            name = "test_script"
            version = "1.0.0"
            authors = []

            [addresses]

            [dev-addresses]

            [dependencies.AptosFramework]
            git = "https://github.com/aptos-labs/aptos-core.git"
            rev = "mainnet"
            subdir = "aptos-move/framework/aptos-framework"

            [dev-dependencies]
        "#;
        let config: MoveConfig = toml::from_str(raw_toml_content).unwrap();
        assert_eq!(config.package.name, "test_script");
    }
}
