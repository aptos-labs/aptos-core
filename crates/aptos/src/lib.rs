// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![deny(unsafe_code)]

pub mod account;
pub mod common;
pub mod config;
pub mod genesis;
pub mod governance;
pub mod move_tool;
pub mod node;
pub mod op;
pub mod stake;
#[cfg(any(test, feature = "fuzzing"))]
pub mod test;
pub mod update;
pub mod workspace;

use crate::common::{
    types::{CliCommand, CliResult, CliTypedResult},
    utils::cli_build_information,
};
use aptos_workspace_server::WorkspaceCommand;
use async_trait::async_trait;
use clap::Parser;
use std::collections::BTreeMap;

/// Command Line Interface (CLI) for developing and interacting with the Aptos blockchain
#[derive(Parser)]
#[clap(name = "aptos", author, version, propagate_version = true, styles = aptos_cli_common::aptos_cli_style())]
pub enum Tool {
    #[clap(subcommand)]
    Account(account::AccountTool),
    #[clap(subcommand)]
    Config(config::ConfigTool),
    #[clap(subcommand)]
    Genesis(genesis::GenesisTool),
    #[clap(subcommand)]
    Governance(governance::GovernanceTool),
    Info(InfoTool),
    Init(common::init::InitTool),
    #[clap(subcommand)]
    Key(op::key::KeyTool),
    #[clap(subcommand)]
    Move(move_tool::MoveTool),
    #[clap(subcommand)]
    Multisig(account::MultisigAccountTool),
    #[clap(subcommand)]
    Node(node::NodeTool),
    #[clap(subcommand)]
    Stake(stake::StakeTool),
    #[clap(subcommand)]
    Update(update::UpdateTool),
    #[clap(subcommand, hide(true))]
    Workspace(WorkspaceCommand),
}

impl Tool {
    pub async fn execute(self) -> CliResult {
        use Tool::*;
        match self {
            Account(tool) => tool.execute().await,
            Config(tool) => tool.execute().await,
            Genesis(tool) => tool.execute().await,
            Governance(tool) => tool.execute().await,
            Info(tool) => tool.execute_serialized().await,
            // TODO: Replace entirely with config init
            Init(tool) => tool.execute_serialized_success().await,
            Key(tool) => tool.execute().await,
            Move(tool) => tool.execute().await,
            Multisig(tool) => tool.execute().await,
            Node(tool) => tool.execute().await,
            Stake(tool) => tool.execute().await,
            Update(tool) => tool.execute().await,
            Workspace(workspace) => workspace.execute_serialized_without_logger().await,
        }
    }
}

/// Show build information about the CLI
///
/// This is useful for debugging as well as determining what versions are compatible with the CLI
#[derive(Parser)]
pub struct InfoTool {}

#[async_trait]
impl CliCommand<BTreeMap<String, String>> for InfoTool {
    fn command_name(&self) -> &'static str {
        "xxxGetCLIInfo"
    }

    async fn execute(self) -> CliTypedResult<BTreeMap<String, String>> {
        Ok(cli_build_information())
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Tool::command().debug_assert()
}
