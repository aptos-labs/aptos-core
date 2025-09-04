// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    velor_workspace_server::WorkspaceCommand::parse()
        .run()
        .await
}
