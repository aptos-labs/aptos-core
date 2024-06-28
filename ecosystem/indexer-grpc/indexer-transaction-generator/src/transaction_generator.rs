// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Move Executor is a wrapped local node that can execute Move scripts and modules.

use crate::{
    managed_node::{LocalnetNodeArgs, ManagedNode},
    test_case::TestCase,
};
use anyhow::Context;
use clap::Parser;
use std::path::PathBuf;

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
}

impl TransactionGeneratorArgs {
    pub async fn run(&self) -> anyhow::Result<()> {
        let managed_node = self.localnet_node.start_node().await?;
        TransactionGenerator::new(
            managed_node,
            self.test_cases_folder.clone(),
            self.output_folder.clone(),
        )
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
            self.generate(TestCase::load(test_case_path).context("Load test failure.")?)
                .await?;
        }

        // Stop the node after the transaction generation.
        self.managed_node.stop().await
    }

    async fn generate(&self, test_case: TestCase) -> anyhow::Result<()> {
        // TODO: finish the test generation logic.
        println!(
            "Generate transaction for test case {} at {:?} ",
            test_case.name, self.output_folder,
        );
        Ok(())
    }
}
