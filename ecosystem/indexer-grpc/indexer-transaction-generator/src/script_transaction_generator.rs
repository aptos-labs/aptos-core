// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{ScriptStep, ScriptTransactionGeneratorConfig},
    managed_node::ManagedNode,
};
use anyhow::{Context, Result};
use aptos::{
    account::fund::FundWithFaucet,
    common::types::{CliCommand, MovePackageDir, ScriptFunctionArguments, TransactionOptions},
    governance::CompileScriptFunction,
    move_tool::{CompileScript, RunScript},
};
use aptos_protos::{
    indexer::v1::{raw_data_client::RawDataClient, GetTransactionsRequest},
    transaction::v1::Transaction,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

/// GRPC request metadata key for the token ID.
const LOCAL_INDEXER_GRPC_URL: &str = "http://127.0.0.1:50051";
const TRANSACTION_STREAM_TIMEOUT_IN_SECS: u64 = 60;
const DEFAULT_FUND_AMOUNT_IN_OCTA: u64 = 100_000_000;

impl ScriptTransactionGeneratorConfig {
    fn validate(&self) -> Result<()> {
        // Validate the script transactions.
        // 1. No output file names are duplicated.
        let mut output_files = std::collections::HashSet::new();
        for script_transactions in &self.scripted_transactions {
            for step in &script_transactions.steps {
                if let Some(output_name) = &step.output_name {
                    if !output_files.insert(output_name) {
                        return Err(anyhow::anyhow!(
                            "[Script Transaction Generator] Output file name {} is duplicated",
                            output_name
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn run(&self, output_path: &Path) -> anyhow::Result<()> {
        // Validate the configuration.
        self.validate()?;

        // Submit and capture the transactions.
        for script_transactions in &self.scripted_transactions {
            // Start the localnet.
            // TODO: refresh the localnet for each script transaction.
            let mut managed_node = ManagedNode::start(None, None).await?;

            let mut versions_to_capture = vec![];

            for step in &script_transactions.steps {
                let version = self.execute_script_step(step).await?;
                if let Some(output_name) = &step.output_name {
                    versions_to_capture.push((version, output_name.clone()));
                }
            }
            self.capture_transaction(output_path, versions_to_capture)
                .await?;

            // Stop the localnet.
            managed_node.stop().await?;
        }
        Ok(())
    }

    /// Prepare the script
    async fn prepare_script_step(&self, step: &ScriptStep) -> anyhow::Result<()> {
        // Set the current folder to the script folder.
        std::env::set_current_dir(&step.script_path)
            .context("Failed to set the current directory to the script folder.")?;

        let fund_address = step.fund_address.clone();
        let fund_cmd = create_fund_cmd(DEFAULT_FUND_AMOUNT_IN_OCTA, fund_address);
        let _ = fund_cmd.execute().await.context(format!(
            "Failed to fund the account for account: {}.",
            step.fund_address
                .clone()
                .unwrap_or("Default profile".to_string())
        ))?;
        Ok(())
    }

    async fn execute_script_step(&self, step: &ScriptStep) -> anyhow::Result<u64> {
        self.prepare_script_step(step).await?;

        let path = step.script_path.to_path_buf();
        // Compile the setup script.
        let cmd = create_compile_script_cmd(path.clone());
        let _ = cmd
            .execute()
            .await
            .context(format!("Failed to compile the script: {:?}", path))?;

        // Read the content of the TOML file. This is to get the package name.
        let content =
            std::fs::read_to_string(path.join("Move.toml")).expect("Failed to read TOML file");
        let value = content
            .parse::<toml::Value>()
            .expect("Failed to parse TOML");
        let package_name = value
            .get("package")
            .context("Malformed Move.toml file: No package.")?
            .get("name")
            .context("Malformed Move.toml file: No package name.")?
            .as_str()
            .context("Malformed package name.")?;

        // Run the compiled script.
        let compiled_build_path = path
            .join("build")
            .join(package_name)
            .join("bytecode_scripts")
            .join("main.mv");

        let cmd = create_run_script_cmd(compiled_build_path);
        let transaction_summary = cmd
            .execute()
            .await
            .context(format!("Failed to run the script: {:?}", path))?;
        if let Some(true) = transaction_summary.success {
            Ok(transaction_summary.version.unwrap())
        } else {
            anyhow::bail!("Failed to execute the script: {:?}", path);
        }
    }

    async fn capture_transaction(
        &self,
        output_path: &Path,
        versions_to_capture: Vec<(u64, String)>,
    ) -> anyhow::Result<()> {
        if versions_to_capture.is_empty() {
            anyhow::bail!("No transaction versions provided to capture.");
        }

        // Build the request.
        let first_version = versions_to_capture.first().unwrap().0;
        let last_version = versions_to_capture.last().unwrap().0;
        let transactions_count = last_version + 1 - first_version;
        let request = tonic::Request::new(aptos_protos::indexer::v1::GetTransactionsRequest {
            starting_version: Some(first_version),
            transactions_count: Some(transactions_count),
            ..GetTransactionsRequest::default()
        });

        // Create a client and send the request.
        let mut client = RawDataClient::connect(LOCAL_INDEXER_GRPC_URL).await?;
        let response = client.get_transactions(request).await?;
        let mut response = response.into_inner();
        let mut transactions: Vec<Transaction> = Vec::new();

        tokio::time::timeout(
            std::time::Duration::from_secs(TRANSACTION_STREAM_TIMEOUT_IN_SECS),
            async {
                while let Ok(Some(resp_item)) = response.message().await {
                    for transaction in resp_item.transactions {
                        transactions.push(transaction);
                    }
                }
            },
        )
        .await
        .context("Transaction stream timeout.")?;
        // Create version to transaction hash map.
        let transaction_versions_with_names: HashMap<u64, String> =
            versions_to_capture.into_iter().collect();

        // Write the transactions to the output files.
        for txn in transactions {
            let version = txn.version;
            let output_name = transaction_versions_with_names.get(&version).unwrap();
            let json_string = serde_json::to_string_pretty(&txn).context(format!(
                "[Script Transaction Generator] Transaction at version {} failed to serialized to json string.",
                version
            ))?;
            let output_path = output_path.join(output_name).with_extension("json");
            tokio::fs::write(&output_path, json_string)
                .await
                .context(format!(
                "[Script Transaction Generator] Failed to write transaction at version {} to file.",
                version
            ))?;

            // Output the transaction to the console.
            println!(
                "Transaction {} is captured, path\n\t {:?}",
                output_name, output_path
            );
        }
        Ok(())
    }
}

fn create_compile_script_cmd(package_dir: PathBuf) -> CompileScript {
    let mut move_package_dir = MovePackageDir::default();
    move_package_dir.package_dir = Some(package_dir);

    CompileScript {
        output_file: None,
        move_options: move_package_dir,
    }
}

fn create_run_script_cmd(script_path: PathBuf) -> RunScript {
    let mut transaction_options = TransactionOptions::default();
    transaction_options.prompt_options.assume_yes = true;
    transaction_options.prompt_options.assume_no = false;
    RunScript {
        txn_options: transaction_options,
        compile_proposal_args: CompileScriptFunction {
            compiled_script_path: Some(script_path),
            ..CompileScriptFunction::default()
        },
        script_function_args: ScriptFunctionArguments::default(),
    }
}

fn create_fund_cmd(amount: u64, account: Option<String>) -> FundWithFaucet {
    let mut fund_with_faucet = FundWithFaucet::default();
    fund_with_faucet.amount = amount;
    fund_with_faucet.account = account.map(|s| s.parse().unwrap());
    fund_with_faucet
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_transaction_generator_config_duplicate_failure() {
        let raw_script_transaction_generator_config = r#"{
            "scripted_transactions": [
                {
                    "steps": [
                        {
                            "output_name": "output1",
                            "script_path": "path/to/script1"
                        },
                        {
                            "output_name": "output1",
                            "script_path": "path/to/script2"
                        }
                    ]
                }
            ]
        }"#;

        let script_transaction_generator_config: ScriptTransactionGeneratorConfig =
            serde_yaml::from_str(raw_script_transaction_generator_config).unwrap();
        let result = script_transaction_generator_config.validate();
        assert!(result.is_err());
    }
}
