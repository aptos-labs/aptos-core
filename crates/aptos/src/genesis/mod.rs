// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod config;
pub mod git;
pub mod keys;

use crate::{
    common::types::CliTypedResult,
    genesis::{
        config::{Layout, ValidatorConfiguration},
        git::{GitOptions, LAYOUT_NAME},
    },
    CliCommand, CliResult,
};
use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_types::chain_id::ChainId;
use async_trait::async_trait;
use clap::Parser;
use serde::Serialize;

/// Tool for setting up and building the Genesis transaction
///
#[derive(Parser)]
pub enum GenesisTool {
    GenerateGenesis(GenerateGenesis),
    GenerateKeys(keys::GenerateKeys),
    SetupGit(git::SetupGit),
    SetValidatorConfiguration(keys::SetValidatorConfiguration),
}

impl GenesisTool {
    pub async fn execute(self) -> CliResult {
        match self {
            GenesisTool::GenerateGenesis(tool) => tool.execute_serialized().await,
            GenesisTool::GenerateKeys(tool) => tool.execute_serialized().await,
            GenesisTool::SetupGit(tool) => tool.execute_serialized_success().await,
            GenesisTool::SetValidatorConfiguration(tool) => tool.execute_serialized_success().await,
        }
    }
}

/// Generate genesis from a git repo
#[derive(Parser)]
pub struct GenerateGenesis {
    #[clap(flatten)]
    github_options: GitOptions,
}

#[async_trait]
impl CliCommand<GenesisInfo> for GenerateGenesis {
    fn command_name(&self) -> &'static str {
        "GenerateGenesis"
    }

    async fn execute(self) -> CliTypedResult<GenesisInfo> {
        // TODO: Generate genesis, this right now just reads all users
        fetch_genesis_info(self.github_options)
    }
}

pub fn fetch_genesis_info(git_options: GitOptions) -> CliTypedResult<GenesisInfo> {
    let client = git_options.get_client()?;
    let layout: Layout = client.get(LAYOUT_NAME)?;

    let mut configs = Vec::new();
    for user in &layout.users {
        configs.push(client.get(user)?);
    }

    Ok(GenesisInfo {
        chain_id: layout.chain_id,
        root_key: layout.root_key,
        participants: configs,
    })
}

#[derive(Debug, Serialize)]
pub struct GenesisInfo {
    chain_id: ChainId,
    root_key: Ed25519PublicKey,
    participants: Vec<ValidatorConfiguration>,
}
