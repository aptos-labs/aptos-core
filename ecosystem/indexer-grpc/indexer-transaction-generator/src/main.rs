// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_indexer_transaction_generator::transaction_generator::TransactionGeneratorArgs;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse the command line arguments.
    let args = TransactionGeneratorArgs::parse();
    args.run().await
}
