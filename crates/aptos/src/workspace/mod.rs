// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{CliCommand, CliResult, CliTypedResult};
use async_trait::async_trait;
use clap::Parser;

/// Tool for operations related to Aptos Workspace
#[derive(Parser)]
pub enum WorkspaceTool {
    Run(RunWorkspace),
}

impl WorkspaceTool {
    pub async fn execute(self) -> CliResult {
        use WorkspaceTool::*;

        match self {
            Run(cmd) => cmd.execute_serialized_without_logger().await,
        }
    }
}

#[derive(Parser)]
pub struct RunWorkspace;

#[async_trait]
impl CliCommand<()> for RunWorkspace {
    fn command_name(&self) -> &'static str {
        "RunWorkspace"
    }

    async fn execute(self) -> CliTypedResult<()> {
        aptos_workspace_server::run_all_services().await?;

        Ok(())
    }
}
