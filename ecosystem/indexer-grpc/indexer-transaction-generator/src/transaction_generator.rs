// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Move Executor is a wrapped local node that can execute Move scripts and modules.

use crate::{
    managed_node::{LocalnetNodeArgs, ManagedNode},
    test_case::{Step, TestCase},
};
use anyhow::Context;
use aptos::{account::fund::FundWithFaucet, common::types::CliCommand};
use aptos_indexer_grpc_utils::create_data_service_grpc_client;
use aptos_protos::indexer::v1::{raw_data_client::RawDataClient, GetTransactionsRequest};
use clap::{Parser, Subcommand};
use std::{path::PathBuf, time::Duration};
use tonic::transport::Channel;
use url::Url;

/// GRPC request metadata key for the token ID.
const GRPC_API_GATEWAY_API_KEY_HEADER: &str = "authorization";
const LOCAL_INDEXER_GRPC_URL: &str = "http://127.0.0.1:50051";
const TRANSACTION_STREAM_TIMEOUT_IN_SECS: u64 = 60;

#[derive(Parser)]
pub struct IndexerCliArgs {
    #[clap(subcommand)]
    pub command: Command,

    #[clap(long)]
    pub output_folder: PathBuf,
}

// IndexerCliArgs is the main entry point for the transaction generator.
// It takes a subcommand to import or generate transactions.
#[derive(Subcommand)]
pub enum Command {
    /// Import transactions from a running transaction stream service.
    Import(TransactionImporterArgs),
    /// Generate transactions from the test cases.
    Generate(TransactionGeneratorArgs),
}

impl IndexerCliArgs {
    pub async fn run(&self) -> anyhow::Result<()> {
        match &self.command {
            Command::Import(args) => args.run(self.output_folder.clone()).await,
            Command::Generate(args) => args.run(self.output_folder.clone()).await,
        }
    }
}

#[derive(Parser)]
pub struct TransactionImporterArgs {
    /// Path to the root folder for the output transaction.
    #[clap(long)]
    pub url: Url,

    #[clap(long)]
    pub key: String,

    #[clap(long)]
    pub version: u64,

    #[clap(long)]
    pub transaction_name: String,
}

impl TransactionImporterArgs {
    pub async fn run(&self, output_folder: PathBuf) -> anyhow::Result<()> {
        // Create a client and send the request.
        let mut client: RawDataClient<Channel> = create_data_service_grpc_client(
            self.url.clone(),
            Some(Duration::from_secs(TRANSACTION_STREAM_TIMEOUT_IN_SECS)),
        )
        .await?;
        let mut request = tonic::Request::new(aptos_protos::indexer::v1::GetTransactionsRequest {
            starting_version: Some(self.version),
            transactions_count: Some(1),
            ..GetTransactionsRequest::default()
        });
        request.metadata_mut().insert(
            GRPC_API_GATEWAY_API_KEY_HEADER,
            format!("Bearer {}", self.key.clone()).parse().unwrap(),
        );
        // Capture the transaction.
        let response = client.get_transactions(request).await?;
        let mut response = response.into_inner();
        let mut transactions = Vec::new();
        while let Ok(Some(resp_item)) = response.message().await {
            for transaction in resp_item.transactions {
                transactions.push(transaction);
            }
        }
        if transactions.is_empty() {
            anyhow::bail!("Failed to fetch the transaction.");
        }
        let transaction = transactions.first().unwrap();
        let transaction_file = self.transaction_name.clone().replace('-', "_");
        std::fs::write(
            output_folder.join(transaction_file).with_extension("json"),
            serde_json::to_string_pretty(&transaction)?,
        )
        .context("Failed to write transaction to file.")
    }
}

#[derive(Parser)]
pub struct TransactionGeneratorArgs {
    /// Path to the root folder for the test cases.
    #[clap(long)]
    pub move_fixtures: PathBuf,
    /// Localnet node arguments.
    #[clap(flatten)]
    pub localnet_node: LocalnetNodeArgs,
}

impl TransactionGeneratorArgs {
    pub async fn run(&self, output_folder: PathBuf) -> anyhow::Result<()> {
        let managed_node = self.localnet_node.start_node().await?;
        TransactionGenerator::new(managed_node, self.move_fixtures.clone(), output_folder)
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
        let test_case_output_folder = self.output_folder.clone();
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
                Step::Action((path, _, config)) => {
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
                        if let Some(config) = config {
                            if let Some(output_name) = config.output_name {
                                transactions_version_to_capture
                                    .push((transaction_summary.version.unwrap(), output_name));
                            }
                        }
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
        transaction_versions_with_names: Vec<(u64, String)>,
        test_case_output_folder: PathBuf,
    ) -> anyhow::Result<()> {
        if transaction_versions_with_names.is_empty() {
            anyhow::bail!("No transaction versions provided to capture.");
        }
        let transaction_versions = transaction_versions_with_names
            .iter()
            .map(|(version, _)| *version)
            .collect::<Vec<_>>();
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
        for (idx, transaction) in transactions.iter().enumerate() {
            let output_name = transaction_versions_with_names.get(idx).unwrap().1.clone();
            let transaction_file =
                test_case_output_folder.join(format!("generated_{}.json", output_name));
            std::fs::write(
                transaction_file,
                serde_json::to_string_pretty(&transaction)?,
            )
            .expect("Failed to write transaction to file.");
        }
        Ok(())
    }
}
