// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![deny(unsafe_code)]

pub mod account;
mod aptos_context;
pub mod common;
pub mod config;
pub mod genesis;
pub mod governance;
pub mod node;
pub mod op;
pub mod stake;
#[cfg(any(test, feature = "fuzzing"))]
pub mod test;
#[cfg(feature = "localnet")]
pub mod update;
#[cfg(feature = "localnet")]
pub mod workspace;

use crate::common::{
    types::{CliCommand, CliError, CliResult, CliTypedResult},
    utils::cli_build_information,
};
pub use aptos_context::RealAptosContext;
use aptos_move_cli::MoveEnv;
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use async_trait::async_trait;
use clap::Parser;
use std::{collections::BTreeMap, sync::Arc};

/// Create a fully wired `MoveEnv` with `RealAptosContext` and debugger support.
///
/// Use this when constructing move CLI commands (e.g., `RunScript`, `RunFunction`)
/// outside the main CLI dispatch.
pub fn create_move_env() -> Arc<MoveEnv> {
    Arc::new(MoveEnv::new(
        Box::new(RealAptosContext),
        Box::new(|client| Ok(Box::new(AptosDebugger::rest_client(client)?))),
    ))
}

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
    Move(aptos_move_cli::MoveTool),
    #[clap(subcommand)]
    Multisig(account::MultisigAccountTool),
    #[clap(subcommand)]
    Node(node::NodeTool),
    #[clap(subcommand)]
    Stake(stake::StakeTool),
    #[cfg(feature = "localnet")]
    #[clap(subcommand)]
    Update(update::UpdateTool),
    #[cfg(feature = "localnet")]
    #[clap(subcommand, hide(true))]
    Workspace(aptos_workspace_server::WorkspaceCommand),
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
            Move(tool) => tool.execute(create_move_env()).await,
            Multisig(tool) => tool.execute().await,
            Node(tool) => tool.execute().await,
            Stake(tool) => tool.execute().await,
            #[cfg(feature = "localnet")]
            Update(tool) => tool.execute().await,
            #[cfg(feature = "localnet")]
            Workspace(workspace) => {
                let start_time = std::time::Instant::now();
                let result: CliTypedResult<()> = workspace
                    .run()
                    .await
                    .map_err(|e| CliError::UnexpectedError(format!("{:#}", e)));
                aptos_cli_common::to_common_success_result("Workspace", start_time, result, true)
                    .await
            },
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
        "GetCLIInfo"
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
