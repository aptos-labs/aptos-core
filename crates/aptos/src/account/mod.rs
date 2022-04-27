// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::{types::CliResult, utils::to_common_result};
use clap::Subcommand;

pub mod create;
pub mod list;
pub mod transfer;

/// CLI tool for interacting with accounts
///
#[derive(Debug, Subcommand)]
pub enum AccountTool {
    Create(create::CreateAccount),
    List(list::ListResources),
    Transfer(transfer::TransferCoins),
}

impl AccountTool {
    pub async fn execute(self) -> CliResult {
        match self {
            AccountTool::Create(tool) => {
                to_common_result("CreateAccount", tool.execute().await).await
            }
            AccountTool::List(tool) => {
                to_common_result("ListResources", tool.execute().await).await
            }
            AccountTool::Transfer(tool) => {
                to_common_result("TransferCoins", tool.execute().await).await
            }
        }
    }
}
