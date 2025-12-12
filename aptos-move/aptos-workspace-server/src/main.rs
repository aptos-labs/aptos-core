// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    aptos_workspace_server::WorkspaceCommand::parse()
        .run()
        .await
}
