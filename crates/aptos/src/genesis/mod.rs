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
use aptos_types::{
    chain_id::ChainId,
    on_chain_config::{ConsensusConfigV2, OnChainConsensusConfig, VMPublishingOption},
};
use async_trait::async_trait;
use clap::Parser;
use vm_genesis::Validator;

const MIN_PRICE_PER_GAS_UNIT: u64 = 1;

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

/// Generate genesis from a git repository
#[derive(Parser)]
pub struct GenerateGenesis {
    #[clap(flatten)]
    github_options: GitOptions,
}

#[async_trait]
impl CliCommand<()> for GenerateGenesis {
    fn command_name(&self) -> &'static str {
        "GenerateGenesis"
    }

    async fn execute(self) -> CliTypedResult<()> {
        let genesis_info = fetch_genesis_info(self.github_options)?;

        let consensus_config = OnChainConsensusConfig::V2(ConsensusConfigV2 {
            two_chain: true,
            decoupled_execution: true,
            back_pressure_limit: 10,
            exclude_round: 20,
        });

        vm_genesis::encode_genesis_transaction(
            genesis_info.root_key.clone(),
            &genesis_info.validators,
            &genesis_info.modules,
            Some(VMPublishingOption::open()), // TODO: Remove
            consensus_config,                 // TODO: Remove
            genesis_info.chain_id,
            MIN_PRICE_PER_GAS_UNIT,
        );

        Ok(())
    }
}

/// Retrieves all information for genesis from the Git repository
pub fn fetch_genesis_info(git_options: GitOptions) -> CliTypedResult<GenesisInfo> {
    let client = git_options.get_client()?;
    let layout: Layout = client.get(LAYOUT_NAME)?;

    let mut validators = Vec::new();
    for user in &layout.users {
        validators.push(client.get::<ValidatorConfiguration>(user)?.into());
    }

    let modules = client.get_modules(&layout.modules_folder)?;

    Ok(GenesisInfo {
        chain_id: layout.chain_id,
        root_key: layout.root_key,
        validators,
        modules,
    })
}

/// Holder object for all pieces needed to generate a genesis transaction
#[derive(Clone)]
pub struct GenesisInfo {
    chain_id: ChainId,
    root_key: Ed25519PublicKey,
    validators: Vec<Validator>,
    modules: Vec<Vec<u8>>,
}
