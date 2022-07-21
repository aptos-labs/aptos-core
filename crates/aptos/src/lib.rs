// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

pub mod account;
pub mod common;
pub mod config;
pub mod genesis;
pub mod move_tool;
pub mod node;
pub mod op;
pub mod test;

use crate::common::types::{CliCommand, CliResult, CliTypedResult};
use async_trait::async_trait;
use clap::Parser;
use std::collections::BTreeMap;

shadow_rs::shadow!(build);

/// CLI tool for interacting with the Aptos blockchain and nodes
///
#[derive(Parser)]
#[clap(name = "aptos", author, version, propagate_version = true)]
pub enum Tool {
    #[clap(subcommand)]
    Account(account::AccountTool),
    #[clap(subcommand)]
    Config(config::ConfigTool),
    #[clap(subcommand)]
    Genesis(genesis::GenesisTool),
    Info(InfoTool),
    Init(common::init::InitTool),
    #[clap(subcommand)]
    Key(op::key::KeyTool),
    #[clap(subcommand)]
    Move(move_tool::MoveTool),
    #[clap(subcommand)]
    Node(node::NodeTool),
}

impl Tool {
    pub async fn execute(self) -> CliResult {
        use Tool::*;
        match self {
            Account(tool) => tool.execute().await,
            Config(tool) => tool.execute().await,
            Genesis(tool) => tool.execute().await,
            Info(tool) => tool.execute_serialized().await,
            // TODO: Replace entirely with config init
            Init(tool) => tool.execute_serialized_success().await,
            Key(tool) => tool.execute().await,
            Move(tool) => tool.execute().await,
            Node(tool) => tool.execute().await,
        }
    }
}

/// Show information about the build of the CLI
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
        let mut build_information: std::collections::BTreeMap<String, String> = BTreeMap::new();
        build_information.insert(
            aptos_telemetry::build_information::BUILD_BRANCH.into(),
            build::BRANCH.into(),
        );
        build_information.insert(
            aptos_telemetry::build_information::BUILD_CARGO_VERSION.into(),
            build::CARGO_VERSION.into(),
        );
        build_information.insert(
            aptos_telemetry::build_information::BUILD_COMMIT_HASH.into(),
            build::COMMIT_HASH.into(),
        );
        build_information.insert(
            aptos_telemetry::build_information::BUILD_OS.into(),
            build::BUILD_OS.into(),
        );
        build_information.insert(
            aptos_telemetry::build_information::BUILD_PKG_VERSION.into(),
            build::PKG_VERSION.into(),
        );
        build_information.insert(
            aptos_telemetry::build_information::BUILD_RUST_CHANNEL.into(),
            build::RUST_CHANNEL.into(),
        );
        build_information.insert(
            aptos_telemetry::build_information::BUILD_RUST_VERSION.into(),
            build::RUST_VERSION.into(),
        );
        Ok(build_information)
    }
}

pub fn build_commit_hash() -> String {
    build::COMMIT_HASH.to_string()
}
