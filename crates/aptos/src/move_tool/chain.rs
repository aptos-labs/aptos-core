// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::CliResult;
use aptos_types::account_address::AccountAddress;
use clap::{ArgEnum, Parser, Subcommand};

/// CLI tool for performing onchain tasks
///
#[derive(Debug, ArgEnum, Subcommand)]
pub enum ChainTool {
    List(ListResources),
}

impl ChainTool {
    pub async fn execute(self) -> CliResult {
        match self {
            ChainTool::List(tool) => tool.execute(),
        }
    }
}

#[derive(Debug, Parser)]
pub struct ListResources {
    account: AccountAddress,
}

impl ListResources {
    pub fn execute(self) -> CliResult {
        Ok("".to_string())
    }
}
