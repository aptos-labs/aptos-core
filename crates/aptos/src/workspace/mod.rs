// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Workspace commands for the Aptos CLI.
//! This module is only compiled when the "workspace" feature is enabled.

use crate::common::types::{CliCommand, CliTypedResult};
use aptos_workspace_server::WorkspaceCommand;
use async_trait::async_trait;

#[async_trait]
impl CliCommand<()> for WorkspaceCommand {
    fn command_name(&self) -> &'static str {
        "Workspace"
    }

    async fn execute(self) -> CliTypedResult<()> {
        self.run().await?;

        Ok(())
    }
}
