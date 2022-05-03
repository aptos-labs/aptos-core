// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

pub mod account;
pub mod common;
pub mod config;
pub mod move_tool;
pub mod op;

use crate::common::types::{CliCommand, CliResult};
use clap::Parser;

/// CLI tool for interacting with the Aptos blockchain and nodes
///
#[derive(Parser)]
#[clap(name = "aptos", author, version, propagate_version = true)]
pub enum Tool {
    #[clap(subcommand)]
    Account(account::AccountTool),
    #[clap(subcommand)]
    Config(config::ConfigTool),
    Init(common::init::InitTool),
    #[clap(subcommand)]
    Move(move_tool::MoveTool),
    #[clap(subcommand)]
    Key(op::key::KeyTool),
}

impl Tool {
    pub async fn execute(self) -> CliResult {
        match self {
            Tool::Account(tool) => tool.execute().await,
            Tool::Config(tool) => tool.execute().await,
            // TODO: Replace entirely with config init
            Tool::Init(tool) => tool.execute_serialized_success().await,
            Tool::Move(tool) => tool.execute().await,
            Tool::Key(tool) => tool.execute().await,
        }
    }
}
