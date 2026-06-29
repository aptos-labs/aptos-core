// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use aptos_release_tool::{run, Argument};
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    run(Argument::parse()).await
}
