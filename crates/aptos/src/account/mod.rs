// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::CliResult;
use clap::Subcommand;

pub mod create;
pub mod list;

/// CLI tool for interacting with accounts
///
#[derive(Debug, Subcommand)]
pub enum AccountTool {
    Create(create::CreateAccount),
    List(list::ListResources),
}

impl AccountTool {
    pub async fn execute(self) -> CliResult {
        match self {
            AccountTool::Create(tool) => tool.execute().await,
            AccountTool::List(tool) => tool.execute().await,
        }
    }
}
