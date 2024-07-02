// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    capture_transactions::capture_transactions, move_executor::MoveExecutor,
    DEFAULT_LOCAL_FAUCET_URL, DEFAULT_LOCAL_NODE_API_URL,
};
use anyhow::Context;
use clap::Parser;
use reqwest::Client;
use std::{path::PathBuf, time::Duration};
use tokio::{process::Child, time::sleep};

const NODE_HEALTH_CHECK_COUNT: u8 = 60;
const TEST_CASE_FOLDER_PREFIX: &str = "test_case_";
const TEST_CASE_SETUP_MODULE_PREFIX: &str = "setup_";
const TEST_CASE_STEP_PREFIX: &str = "step_";

/// Args to start the transaction generator.
#[derive(Debug, Parser)]
pub struct TransactionGeneratorArgs {
    /// The path to the test cases main folder.
    #[clap(long)]
    pub test_cases_folder: PathBuf,

    /// The path of generated test cases.
    #[clap(long)]
    pub output_test_cases_folder: PathBuf,

    /// The path of the aptos node binary.
    /// This adds the ability to use a custom binary for the local node.
    #[clap(long)]
    pub aptos_node_binary: PathBuf,

    /// The path of local node config file to override the default config.
    #[clap(long)]
    pub node_config: Option<PathBuf>,

    /// The node version; this is to generate the test cases for a specific version.
    /// If not provided, it will default to the `main` branch.
    #[clap(long, default_value = "main")]
    pub node_version: String,
}

impl TransactionGeneratorArgs {
    /// A new transaction generator with test cases loaded.
    pub fn get_transaction_generator(self) -> TransactionGenerator {
        TransactionGenerator::new(
            self.test_cases_folder,
            self.output_test_cases_folder,
            self.aptos_node_binary,
            self.node_version,
            self.node_config,
        )
    }
}

/// Struct that generates transactions for testing purposes.
/// Internally, it brings up a local node and sends transactions based on the test case.
#[derive(Debug)]
pub struct TransactionGenerator {
    /// The local node that the transaction generator uses to send transactions.
    // node: Node,
    /// The test case that the transaction generator uses to generate transactions.
    // test_case: TestCaseConfig,
    test_cases_folder: PathBuf,

    /// The folder where the generated test cases will be stored.
    output_test_cases_folder: PathBuf,

    /// Override node config path.
    node_config: Option<PathBuf>,

    /// Whether the transaction generator has been initialized correctly.
    is_initialized: bool,

    /// The aptos node binary path. This is used to start the local node.
    aptos_node_binary: PathBuf,

    /// The process handle of the local node.
    node_process: Option<Child>,

    /// Release version: `1.13.0`` or `latest`.
    node_version: String,

    /// Move Executor.
    move_executor: Option<MoveExecutor>,
}

impl TransactionGenerator {
    /// Create a new transaction generator.
    /// The transaction generator is not initialized until `initialize` is called.
    fn new(
        test_cases_folder: PathBuf,
        output_test_cases_folder: PathBuf,
        aptos_node_binary: PathBuf,
        node_version: String,
        node_config: Option<PathBuf>,
    ) -> Self {
        Self {
            test_cases_folder,
            output_test_cases_folder,
            node_config,
            is_initialized: false,
            node_process: None,
            aptos_node_binary,
            node_version,
            move_executor: None,
        }
    }

    /// Initialize the transaction executor, which includes:
    ///  - A local node.
    ///  - MoveExecutor.
    pub async fn initialize(&mut self) -> anyhow::Result<()> {
        // Start the local node.
        self.node_process =
            Some(start_localnode(self.aptos_node_binary.clone(), self.node_config.clone()).await?);
        // Create the Move executor.
        let move_executor = MoveExecutor::new(DEFAULT_LOCAL_NODE_API_URL)
            .context("Failed to create Move executor.")?;
        self.move_executor = Some(move_executor);
        self.is_initialized = true;
        Ok(())
    }

    /// Build the transactions based on the test cases read.
    /// TODO: refactor this.
    pub async fn build_test_cases(&self) -> anyhow::Result<()> {
        if !self.is_initialized {
            return Err(anyhow::anyhow!("Transaction generator is not initialized."));
        }
        // Scan the folder for test cases.
        let test_cases = std::fs::read_dir(&self.test_cases_folder)
            .context("Failed to read test cases folder.")?;
        for test_case in test_cases {
            let test_case = test_case.context("Failed to read test case entry.")?;
            let path = test_case.path();
            if !path.is_dir() {
                continue;
            }
            if !path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with(TEST_CASE_FOLDER_PREFIX)
            {
                continue;
            }
            self.build_test_case(path).await?;
        }
        Ok(())
    }

    /// Build a single test case.
    pub async fn build_test_case(&self, test_case_path: PathBuf) -> anyhow::Result<()> {
        tracing::info!("Building test case: {:?}", test_case_path);
        // Read the test case.
        let test_case =
            std::fs::read_dir(&test_case_path).context("Failed to read test case folder.")?;
        let mut setups = Vec::new();
        let mut steps = Vec::new();
        for entry in test_case {
            let entry = entry.context("Failed to read test case entry.")?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let file_name = path.file_name().unwrap().to_str().unwrap();
            if file_name.starts_with(TEST_CASE_SETUP_MODULE_PREFIX) {
                let setup_index = file_name
                    .split('_')
                    .last()
                    .context("Failed to get setup index.")?
                    .parse::<u32>()
                    .context("Failed to get setup index.")?;
                setups.push((path, setup_index));
            } else if file_name.starts_with(TEST_CASE_STEP_PREFIX) {
                let step_index = file_name
                    .split('_')
                    .last()
                    .context("Failed to get step index.")?
                    .parse::<u32>()
                    .context("Failed to get step index.")?;
                steps.push((path, step_index));
            }
        }
        // Sort the setups and steps.
        setups.sort_by(|a, b| a.1.cmp(&b.1));
        steps.sort_by(|a, b| a.1.cmp(&b.1));
        tracing::info!("Setups: {:?}", setups);
        tracing::info!("Steps: {:?}", steps);

        let move_executor = self
            .move_executor
            .as_ref()
            .expect("Move executor is not initialized.");

        // Execute the setups.
        for (setup_path, _) in setups {
            let _ = move_executor
                .execute(&setup_path)
                .await
                .context(format!("Failed to execute setups {:?}", setup_path))?;
        }
        let mut transactions = Vec::new();

        // Execute the steps.
        for (step_path, _) in steps {
            let transaction = move_executor
                .execute(&step_path)
                .await
                .context(format!("Failed to execute steps {:?}", step_path))?
                .expect("Transaction is not found.");
            transactions.push(transaction);
        }

        // Capture the transactions.
        let output_folder = self
            .output_test_cases_folder
            .join(
                test_case_path
                    .file_name()
                    .context("Failed to get test name")?,
            )
            .join(&self.node_version);
        capture_transactions(transactions, output_folder)
            .await
            .context(format!(
                "Failed to capture transactions{:?}",
                test_case_path
            ))?;

        Ok(())
    }
}

async fn start_localnode(path: PathBuf, node_config: Option<PathBuf>) -> anyhow::Result<Child> {
    // Start the local node.
    let mut node_process_cmd = tokio::process::Command::new(path);
    // TODO: supports node config.
    node_process_cmd
        .arg("node")
        .arg("run-local-testnet")
        .arg("--force-restart")
        .arg("--assume-yes");

    // Feed the node config if provided.
    if let Some(path) = node_config {
        node_process_cmd.arg("--config").arg(path);
    }

    let node_process = node_process_cmd
        // TODO: fix this with child.kill().
        .kill_on_drop(true)
        .spawn()
        .context("Failed to start local node.")?;
    for _ in 0..NODE_HEALTH_CHECK_COUNT {
        // Curl faucet to make sure the node is up.
        let client = Client::new();
        let response = client
            .get(DEFAULT_LOCAL_FAUCET_URL)
            .timeout(Duration::from_secs(1))
            .send()
            .await;
        if response.is_ok() {
            return Ok(node_process);
        }
        sleep(Duration::from_secs(1)).await;
    }
    Err(anyhow::anyhow!("Local node did not start."))
}
