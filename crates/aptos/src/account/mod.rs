// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::{
    types::{CliCommand, CliResult},
    utils::init_logger,
};
use clap::Subcommand;

pub mod create;
pub mod fund;
pub mod list;
pub mod transfer;

/// CLI tool for interacting with accounts
///
#[derive(Debug, Subcommand)]
pub enum AccountTool {
    Create(create::CreateAccount),
    Fund(fund::FundAccount),
    List(list::ListAccount),
    Transfer(transfer::TransferCoins),
}

impl AccountTool {
    pub async fn execute(self) -> CliResult {
        init_logger();
        match self {
            AccountTool::Create(tool) => tool.execute_serialized().await,
            AccountTool::Fund(tool) => tool.execute_serialized().await,
            AccountTool::List(tool) => tool.execute_serialized().await,
            AccountTool::Transfer(tool) => tool.execute_serialized().await,
        }
    }
}
