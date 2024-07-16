// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Move Executor is a wrapped local node that can execute Move scripts and modules.

use crate::{
    managed_node::{LocalnetNodeArgs, ManagedNode},
    test_case::{Step, TestCase},
};
use anyhow::Context;
use aptos::{account::fund::FundWithFaucet, common::types::CliCommand};
use aptos_protos::indexer::v1::{raw_data_client::RawDataClient, GetTransactionsRequest};
use clap::Parser;
use std::path::PathBuf;

const LOCAL_INDEXER_GRPC_URL: &str = "http://127.0.0.1:50051";
const TRANSACTION_STREAM_TIMEOUT_IN_SECS: u64 = 60;

#[derive(Parser)]
pub struct TransactionGeneratorArgs {
    /// Path to the root folder for the test cases.
    #[clap(long)]
    pub test_cases_folder: PathBuf,
    /// Path to the root folder for the output transaction.
    #[clap(long)]
    pub output_folder: PathBuf,

    /// Localnet node arguments.
    #[clap(flatten)]
    pub localnet_node: LocalnetNodeArgs,

    /// Release version.
    #[clap(long, default_value = "main")]
    pub release_version: String,
}

impl TransactionGeneratorArgs {
    pub async fn run(&self) -> anyhow::Result<()> {
        let managed_node = self.localnet_node.start_node().await?;
        let output_folder = self
            .output_folder
            .clone()
            .join(self.release_version.clone());
        TransactionGenerator::new(managed_node, self.test_cases_folder.clone(), output_folder)
            .run()
            .await
    }
}

/// TransactionGenerator takes a test case folder each time and outputs the transaction files.
pub struct TransactionGenerator {
    // Node is part of the transaction generator, which requires a running node.
    managed_node: ManagedNode,

    // Root folder for the test cases.
    test_cases_folder: PathBuf,

    // Output folder for the transactions.
    output_folder: PathBuf,
}

impl TransactionGenerator {
    pub fn new(
        managed_node: ManagedNode,
        test_cases_folder: PathBuf,
        output_folder: PathBuf,
    ) -> Self {
        Self {
            managed_node,
            test_cases_folder,
            output_folder,
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let test_cases = std::fs::read_dir(&self.test_cases_folder)?;
        for test_case in test_cases {
            let test_case = test_case?;
            let test_case_path = test_case.path();
            if !test_case_path.is_dir() {
                continue;
            }
            if !test_case_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("test_")
            {
                continue;
            }
            self.generate(TestCase::load(test_case_path).context("Load test failure.")?)
                .await?;
        }

        // Stop the node after the transaction generation.
        self.managed_node.stop().await
    }

    /// Go through the test case steps and generate the transactions.
    async fn generate(&self, test_case: TestCase) -> anyhow::Result<()> {
        println!(
            "\n Generate transaction for test case {} at {:?} \n",
            test_case.name, self.output_folder,
        );
        // Test case output folder.
        let test_case_output_folder = self.output_folder.join(&test_case.name);
        // If the output folder doesn't exist, create it.
        if !test_case_output_folder.exists() {
            std::fs::create_dir_all(&test_case_output_folder)?;
        }
        let mut transactions_version_to_capture = vec![];
        // Execute the transactions.
        for step in test_case.steps {
            match step {
                Step::Setup((path, _, config)) => {
                    // Change current directory to the setup script folder.
                    std::env::set_current_dir(&path)
                        .context(format!("Failed to change directory to: {:?}", path))?;
                    if let Some(test_config) = config {
                        if let Some(amount) = test_config.fund_amount {
                            println!("Funding account with amount: {}", amount);
                            let fund = FundWithFaucet::new_for_indexer_testing(amount);
                            fund.execute()
                                .await
                                .context("Failed to fund the account.")?;
                            println!("Funded account with amount: {}", amount);
                        }
                    }
                    println!("Compiling and running the setup script: {:?}", path);
                    // Compile the setup script.
                    let cmd =
                        aptos::move_tool::CompilePackage::new_for_indexer_testing(path.clone());
                    let _ = cmd
                        .execute()
                        .await
                        .context(format!("Failed to compile package: {:?}", path))?;
                    println!("Compiled the setup script: {:?}", path);
                    // Publish the setup script.
                    let cmd =
                        aptos::move_tool::PublishPackage::new_for_indexer_testing(path.clone());
                    let _ = cmd
                        .execute()
                        .await
                        .context(format!("Failed to publish module: {:?}", path))?;
                },
                Step::Action((path, _)) => {
                    // Change current directory to the setup script folder.
                    std::env::set_current_dir(&path)
                        .context(format!("Failed to change directory to: {:?}", path))?;
                    // Compile the setup script.
                    let cmd =
                        aptos::move_tool::CompilePackage::new_for_indexer_testing(path.clone());
                    let _ = cmd
                        .execute()
                        .await
                        .context(format!("Failed to compile the script: {:?}", path))?;
                    // Read the content of the TOML file
                    let content = std::fs::read_to_string(path.join("Move.toml"))
                        .expect("Failed to read TOML file");
                    // Parse the TOML content
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
                    let cmd =
                        aptos::move_tool::RunScript::new_for_indexer_testing(compiled_build_path);
                    let transaction_summary = cmd
                        .execute()
                        .await
                        .context(format!("Failed to run the script: {:?}", path))?;
                    if let Some(true) = transaction_summary.success {
                        transactions_version_to_capture.push(
                            transaction_summary
                                .version
                                .context("Failed to get the transaction version.")?,
                        );
                    } else {
                        anyhow::bail!("Failed to execute the script: {:?}", path);
                    }
                },
                _ => {
                    // Ignore other steps.
                },
            }
        }
        self.capture_transactions(transactions_version_to_capture, test_case_output_folder)
            .await
    }

    /// Capture the transactions.
    async fn capture_transactions(
        &self,
        transaction_versions: Vec<u64>,
        test_case_output_folder: PathBuf,
    ) -> anyhow::Result<()> {
        if transaction_versions.is_empty() {
            anyhow::bail!("No transaction versions provided to capture.");
        }
        // Make sure the transactions are sorted.
        let mut transaction_versions = transaction_versions;
        transaction_versions.sort();
        // Build the request.
        let first_version = *transaction_versions.first().unwrap();
        let last_version = *transaction_versions.last().unwrap();
        let transactions_count = last_version - first_version + 1;
        let request = tonic::Request::new(aptos_protos::indexer::v1::GetTransactionsRequest {
            starting_version: Some(first_version),
            transactions_count: Some(transactions_count),
            ..GetTransactionsRequest::default()
        });

        // Create a client and send the request.
        let mut client = RawDataClient::connect(LOCAL_INDEXER_GRPC_URL).await?;
        let response = client.get_transactions(request).await?;
        let mut response = response.into_inner();
        let mut transactions = Vec::new();
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

        // Filter the transactions.
        let transactions = transactions
            .into_iter()
            .filter(|t| transaction_versions.contains(&t.version))
            .collect::<Vec<_>>();

        // If the number of transactions fetched is not equal to the number of requested transactions, return an error.
        if transactions.len() != transaction_versions.len() {
            let fetched_versions = transactions
                .iter()
                .map(|transaction| transaction.version)
                .collect::<Vec<_>>();
            anyhow::bail!(format!(
                "Failed to fetch all requested transactions, expect {:?}, actually got {:?}.",
                transaction_versions, fetched_versions
            ));
        }

        // Change the transactions versions to be 1, 2, 3... This is to make sure diff is stable.
        let transactions = transactions
            .into_iter()
            .enumerate()
            .map(|(i, mut transaction)| {
                transaction.version = i as u64 + 1;
                transaction
            })
            .collect::<Vec<_>>();
        for transaction in transactions {
            let transaction_file =
                test_case_output_folder.join(format!("{}.json", transaction.version));
            std::fs::write(
                transaction_file,
                serde_json::to_string_pretty(&transaction)?,
            )
            .expect("Failed to write transaction to file.");
        }
        Ok(())
    }
}
