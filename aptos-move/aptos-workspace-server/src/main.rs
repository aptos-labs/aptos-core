// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    aptos_workspace_server::run_all_services().await?;

    Ok(())
}
