// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_indexer_grpc_server_framework::setup_logging;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    setup_logging(None);

    let args = aptos_indexer_grpc_integration_test_transaction_generator::transaction_generator::TransactionGeneratorArgs::parse();
    let mut transaction_generator = args.get_transaction_generator();
    transaction_generator.initialize().await?;
    transaction_generator.build_test_cases().await?;
    tracing::info!("All test cases are generated.");
    Ok(())
}
