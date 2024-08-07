// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_indexer_transaction_generator::managed_node::ManagedNode;

#[tokio::main]
async fn main() -> Result<()> {
    let mut managed_node = ManagedNode::start(&None, None, None).await?;
    managed_node.stop().await?;
    println!("Transaction generator finished.");
    Ok(())
}
