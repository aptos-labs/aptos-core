// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod git;
pub mod keys;
#[cfg(test)]
mod tests;

use crate::{
    common::{
        types::{CliError, CliTypedResult, PromptOptions},
        utils::{check_if_file_exists, init_logger, write_to_file},
    },
    genesis::git::{Client, GitOptions, LAYOUT_NAME},
    CliCommand, CliResult,
};
use aptos_crypto::{bls12381, ed25519::Ed25519PublicKey, x25519, ValidCryptoMaterialStringExt};
use aptos_genesis::{
    config::{HostAndPort, Layout, ValidatorConfiguration},
    GenesisInfo,
};
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, str::FromStr};

const WAYPOINT_FILE: &str = "waypoint.txt";
const GENESIS_FILE: &str = "genesis.blob";

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
        init_logger();
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
    prompt_options: PromptOptions,
    #[clap(flatten)]
    git_options: GitOptions,
    #[clap(long, parse(from_os_str), default_value = ".")]
    output_dir: PathBuf,
}

#[async_trait]
impl CliCommand<Vec<PathBuf>> for GenerateGenesis {
    fn command_name(&self) -> &'static str {
        "GenerateGenesis"
    }

    async fn execute(self) -> CliTypedResult<Vec<PathBuf>> {
        let genesis_file = self.output_dir.join(GENESIS_FILE);
        let waypoint_file = self.output_dir.join(WAYPOINT_FILE);
        check_if_file_exists(genesis_file.as_path(), self.prompt_options)?;
        check_if_file_exists(waypoint_file.as_path(), self.prompt_options)?;

        // Generate genesis file
        let mut genesis_info = fetch_genesis_info(self.git_options)?;
        let genesis = genesis_info.get_genesis();
        write_to_file(
            genesis_file.as_path(),
            GENESIS_FILE,
            &bcs::to_bytes(genesis).map_err(|e| CliError::BCS(GENESIS_FILE, e))?,
        )?;

        // Generate waypoint file
        let waypoint = genesis_info.generate_waypoint()?;
        write_to_file(
            waypoint_file.as_path(),
            WAYPOINT_FILE,
            waypoint.to_string().as_bytes(),
        )?;
        Ok(vec![genesis_file, waypoint_file])
    }
}

/// Retrieves all information for genesis from the Git repository
pub fn fetch_genesis_info(git_options: GitOptions) -> CliTypedResult<GenesisInfo> {
    let client = git_options.get_client()?;
    let layout: Layout = client.get(LAYOUT_NAME)?;

    let mut validators = Vec::new();
    let mut errors = Vec::new();
    for user in &layout.users {
        match get_config(&client, user) {
            Ok(validator) => {
                validators.push(validator);
            }
            Err(failure) => {
                if let CliError::UnexpectedError(failure) = failure {
                    errors.push(format!("{}: {}", user, failure));
                } else {
                    errors.push(format!("{}: {:?}", user, failure));
                }
            }
        }
    }

    // Collect errors, and print out failed inputs
    if !errors.is_empty() {
        eprintln!(
            "Failed to parse genesis inputs:\n{}",
            serde_yaml::to_string(&errors).unwrap()
        );
        return Err(CliError::UnexpectedError(
            "Failed to parse genesis inputs".to_string(),
        ));
    }

    let modules = client.get_modules("framework")?;

    Ok(GenesisInfo::new(
        layout.chain_id,
        layout.root_key,
        validators,
        modules,
        layout.min_price_per_gas_unit,
        layout.allow_new_validators,
        layout.min_stake,
        layout.max_stake,
        layout.min_lockup_duration_secs,
        layout.max_lockup_duration_secs,
        layout.epoch_duration_secs,
        layout.initial_lockup_timestamp,
    )?)
}

/// Do proper parsing so more information is known about failures
fn get_config(client: &Client, user: &str) -> CliTypedResult<ValidatorConfiguration> {
    let config = client.get::<StringValidatorConfiguration>(user)?;

    // Convert each individually
    let account_address = AccountAddress::from_str(&config.account_address)
        .map_err(|_| CliError::UnexpectedError("account_address invalid".to_string()))?;
    let account_key = Ed25519PublicKey::from_encoded_string(&config.account_public_key)
        .map_err(|_| CliError::UnexpectedError("account_key invalid".to_string()))?;
    let consensus_key = bls12381::PublicKey::from_encoded_string(&config.consensus_public_key)
        .map_err(|_| CliError::UnexpectedError("consensus_key invalid".to_string()))?;
    let proof_of_possession =
        bls12381::ProofOfPossession::from_encoded_string(&config.proof_of_possession)
            .map_err(|_| CliError::UnexpectedError("proof_of_possession invalid".to_string()))?;
    let validator_network_key =
        x25519::PublicKey::from_encoded_string(&config.validator_network_public_key)
            .map_err(|_| CliError::UnexpectedError("validator_network_key invalid".to_string()))?;
    let validator_host = config.validator_host.clone();
    let full_node_network_key =
        if let Some(ref full_node_network_key) = config.full_node_network_public_key {
            Some(
                x25519::PublicKey::from_encoded_string(full_node_network_key).map_err(|_| {
                    CliError::UnexpectedError("full_node_network_key invalid".to_string())
                })?,
            )
        } else {
            None
        };
    let full_node_host = config.full_node_host;

    Ok(ValidatorConfiguration {
        account_address,
        consensus_public_key: consensus_key,
        proof_of_possession,
        account_public_key: account_key,
        validator_network_public_key: validator_network_key,
        validator_host,
        full_node_network_public_key: full_node_network_key,
        full_node_host,
        stake_amount: config.stake_amount,
    })
}

/// For better parsing error messages
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StringValidatorConfiguration {
    /// Account address
    pub account_address: String,
    /// Key used for signing in consensus
    pub consensus_public_key: String,
    /// Proof of possession of the consensus key
    pub proof_of_possession: String,
    /// Key used for signing transactions with the account
    pub account_public_key: String,
    /// Public key used for validator network identity (same as account address)
    pub validator_network_public_key: String,
    /// Host for validator which can be an IP or a DNS name
    pub validator_host: HostAndPort,
    /// Public key used for full node network identity (same as account address)
    pub full_node_network_public_key: Option<String>,
    /// Host for full node which can be an IP or a DNS name and is optional
    pub full_node_host: Option<HostAndPort>,
    /// Stake amount for consensus
    pub stake_amount: u64,
}
