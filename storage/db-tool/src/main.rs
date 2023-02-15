// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_db_tool::DBTool;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    DBTool::from_args().run().await?;
    Ok(())
}
