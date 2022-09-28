// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod git;
pub mod keys;
#[cfg(test)]
mod tests;

use crate::common::utils::dir_default_to_current;
use crate::genesis::git::{OPERATOR_FILE, OWNER_FILE};
use crate::{
    common::{
        types::{CliError, CliTypedResult, PromptOptions},
        utils::{check_if_file_exists, write_to_file},
    },
    genesis::git::{
        Client, GitOptions, BALANCES_FILE, EMPLOYEE_VESTING_ACCOUNTS_FILE, LAYOUT_FILE,
    },
    CliCommand, CliResult,
};
use aptos_crypto::{bls12381, ed25519::Ed25519PublicKey, x25519, ValidCryptoMaterialStringExt};
use aptos_genesis::builder::GenesisConfiguration;
use aptos_genesis::config::{StringOperatorConfiguration, StringOwnerConfiguration};
use aptos_genesis::{
    config::{Layout, ValidatorConfiguration},
    mainnet::MainnetGenesisInfo,
    GenesisInfo,
};
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use clap::Parser;
use std::path::Path;
use std::{path::PathBuf, str::FromStr};
use vm_genesis::{AccountMap, EmployeeAccountMap};

const WAYPOINT_FILE: &str = "waypoint.txt";
const GENESIS_FILE: &str = "genesis.blob";

/// Tool for setting up an Aptos chain Genesis transaction
///
/// This tool sets up a space for multiple initial "validator"
/// accounts to build a genesis transaction for a new chain.
#[derive(Parser)]
pub enum GenesisTool {
    GenerateGenesis(GenerateGenesis),
    GenerateKeys(keys::GenerateKeys),
    GenerateLayoutTemplate(keys::GenerateLayoutTemplate),
    GenerateAdminWriteSet(keys::GenerateAdminWriteSet),
    SetupGit(git::SetupGit),
    SetValidatorConfiguration(keys::SetValidatorConfiguration),
}

impl GenesisTool {
    pub async fn execute(self) -> CliResult {
        match self {
            GenesisTool::GenerateGenesis(tool) => tool.execute_serialized().await,
            GenesisTool::GenerateKeys(tool) => tool.execute_serialized().await,
            GenesisTool::GenerateLayoutTemplate(tool) => tool.execute_serialized_success().await,
            GenesisTool::GenerateAdminWriteSet(tool) => tool.execute_serialized_success().await,
            GenesisTool::SetupGit(tool) => tool.execute_serialized_success().await,
            GenesisTool::SetValidatorConfiguration(tool) => tool.execute_serialized_success().await,
        }
    }
}

/// Generate genesis from a git repository
#[derive(Parser)]
pub struct GenerateGenesis {
    /// Output directory for Genesis file and waypoint
    #[clap(long, parse(from_os_str))]
    output_dir: Option<PathBuf>,
    /// Whether this is mainnet genesis.
    #[clap(long)]
    mainnet: Option<bool>,

    #[clap(flatten)]
    prompt_options: PromptOptions,
    #[clap(flatten)]
    git_options: GitOptions,
}

#[async_trait]
impl CliCommand<Vec<PathBuf>> for GenerateGenesis {
    fn command_name(&self) -> &'static str {
        "GenerateGenesis"
    }

    async fn execute(self) -> CliTypedResult<Vec<PathBuf>> {
        let output_dir = dir_default_to_current(self.output_dir.clone())?;
        let genesis_file = output_dir.join(GENESIS_FILE);
        let waypoint_file = output_dir.join(WAYPOINT_FILE);
        check_if_file_exists(genesis_file.as_path(), self.prompt_options)?;
        check_if_file_exists(waypoint_file.as_path(), self.prompt_options)?;

        // Generate genesis and waypoint files
        let (genesis_bytes, waypoint) = if self.mainnet.unwrap_or_default() {
            let mut mainnet_genesis = fetch_mainnet_genesis_info(self.git_options)?;
            let genesis_bytes = bcs::to_bytes(mainnet_genesis.clone().get_genesis())
                .map_err(|e| CliError::BCS(GENESIS_FILE, e))?;
            (genesis_bytes, mainnet_genesis.generate_waypoint()?)
        } else {
            let mut test_genesis = fetch_genesis_info(self.git_options)?;
            let genesis_bytes = bcs::to_bytes(test_genesis.clone().get_genesis())
                .map_err(|e| CliError::BCS(GENESIS_FILE, e))?;
            (genesis_bytes, test_genesis.generate_waypoint()?)
        };
        write_to_file(genesis_file.as_path(), GENESIS_FILE, &genesis_bytes)?;
        write_to_file(
            waypoint_file.as_path(),
            WAYPOINT_FILE,
            waypoint.to_string().as_bytes(),
        )?;
        Ok(vec![genesis_file, waypoint_file])
    }
}

/// Retrieves all information for mainnet genesis from the Git repository
pub fn fetch_mainnet_genesis_info(git_options: GitOptions) -> CliTypedResult<MainnetGenesisInfo> {
    let client = git_options.get_client()?;
    let layout: Layout = client.get(Path::new(LAYOUT_FILE))?;

    let accounts: Vec<AccountMap> = client.get(Path::new(BALANCES_FILE))?;
    let employee_vesting_accounts: Vec<EmployeeAccountMap> =
        client.get(Path::new(EMPLOYEE_VESTING_ACCOUNTS_FILE))?;
    let validators = get_validator_configs(&client, &layout, true).map_err(parse_error)?;
    let framework = client.get_framework()?;
    Ok(MainnetGenesisInfo::new(
        layout.chain_id,
        accounts,
        employee_vesting_accounts,
        validators,
        framework,
        &GenesisConfiguration {
            allow_new_validators: true,
            epoch_duration_secs: layout.epoch_duration_secs,
            is_test: false,
            min_stake: layout.min_stake,
            min_voting_threshold: layout.min_voting_threshold,
            max_stake: layout.max_stake,
            recurring_lockup_duration_secs: layout.recurring_lockup_duration_secs,
            required_proposer_stake: layout.required_proposer_stake,
            rewards_apy_percentage: layout.rewards_apy_percentage,
            voting_duration_secs: layout.voting_duration_secs,
            voting_power_increase_limit: layout.voting_power_increase_limit,
        },
    )?)
}

/// Retrieves all information for genesis from the Git repository
pub fn fetch_genesis_info(git_options: GitOptions) -> CliTypedResult<GenesisInfo> {
    let client = git_options.get_client()?;
    let layout: Layout = client.get(Path::new(LAYOUT_FILE))?;

    if layout.root_key.is_none() {
        return Err(CliError::UnexpectedError(
            "Layout field root_key was not set.  Please provide a hex encoded Ed25519PublicKey."
                .to_string(),
        ));
    }

    let validators = get_validator_configs(&client, &layout, false).map_err(parse_error)?;
    let framework = client.get_framework()?;
    Ok(GenesisInfo::new(
        layout.chain_id,
        layout.root_key.unwrap(),
        validators,
        framework,
        &GenesisConfiguration {
            allow_new_validators: layout.allow_new_validators,
            epoch_duration_secs: layout.epoch_duration_secs,
            is_test: layout.is_test,
            min_stake: layout.min_stake,
            min_voting_threshold: layout.min_voting_threshold,
            max_stake: layout.max_stake,
            recurring_lockup_duration_secs: layout.recurring_lockup_duration_secs,
            required_proposer_stake: layout.required_proposer_stake,
            rewards_apy_percentage: layout.rewards_apy_percentage,
            voting_duration_secs: layout.voting_duration_secs,
            voting_power_increase_limit: layout.voting_power_increase_limit,
        },
    )?)
}

fn parse_error(errors: Vec<String>) -> CliError {
    eprintln!(
        "Failed to parse genesis inputs:\n{}",
        serde_yaml::to_string(&errors).unwrap()
    );
    CliError::UnexpectedError("Failed to parse genesis inputs".to_string())
}

fn get_validator_configs(
    client: &Client,
    layout: &Layout,
    is_mainnet: bool,
) -> Result<Vec<ValidatorConfiguration>, Vec<String>> {
    let mut validators = Vec::new();
    let mut errors = Vec::new();
    for user in &layout.users {
        match get_config(client, user, is_mainnet) {
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

    if errors.is_empty() {
        Ok(validators)
    } else {
        Err(errors)
    }
}

/// Do proper parsing so more information is known about failures
fn get_config(
    client: &Client,
    user: &str,
    is_mainnet: bool,
) -> CliTypedResult<ValidatorConfiguration> {
    // Load a user's configuration files
    let dir = PathBuf::from(user);
    let owner_file = dir.join(OWNER_FILE);
    let owner_file = owner_file.as_path();
    let owner_config = client.get::<StringOwnerConfiguration>(owner_file)?;

    // Check and convert fields in owner file
    let owner_account_address = parse_required_option(
        &owner_config.owner_account_address,
        owner_file,
        "owner_account_address",
        AccountAddress::from_str,
    )?;
    let owner_account_public_key = parse_required_option(
        &owner_config.owner_account_public_key,
        owner_file,
        "owner_account_public_key",
        Ed25519PublicKey::from_encoded_string,
    )?;

    let operator_account_address = parse_required_option(
        &owner_config.operator_account_address,
        owner_file,
        "operator_account_address",
        AccountAddress::from_str,
    )?;
    let operator_account_public_key = parse_required_option(
        &owner_config.operator_account_public_key,
        owner_file,
        "operator_account_public_key",
        Ed25519PublicKey::from_encoded_string,
    )?;

    let voter_account_address = parse_required_option(
        &owner_config.voter_account_address,
        owner_file,
        "voter_account_address",
        AccountAddress::from_str,
    )?;
    let voter_account_public_key = parse_required_option(
        &owner_config.voter_account_public_key,
        owner_file,
        "voter_account_public_key",
        Ed25519PublicKey::from_encoded_string,
    )?;

    let stake_amount = parse_required_option(
        &owner_config.stake_amount,
        owner_file,
        "stake_amount",
        u64::from_str,
    )?;

    // Default to 0 for commission percentage if missing.
    let commission_percentage = parse_optional_option(
        &owner_config.commission_percentage,
        owner_file,
        "commission_percentage",
        u64::from_str,
    )?
    .unwrap_or(0);

    // Default to true for whether the validator should be joining during genesis.
    let join_during_genesis = parse_optional_option(
        &owner_config.join_during_genesis,
        owner_file,
        "join_during_genesis",
        bool::from_str,
    )?
    .unwrap_or(true);

    // We don't require the operator file if the validator is not joining during genesis.
    if is_mainnet && !join_during_genesis {
        return Ok(ValidatorConfiguration {
            owner_account_address,
            owner_account_public_key,
            operator_account_address,
            operator_account_public_key,
            voter_account_address,
            voter_account_public_key,
            consensus_public_key: None,
            proof_of_possession: None,
            validator_network_public_key: None,
            validator_host: None,
            full_node_network_public_key: None,
            full_node_host: None,
            stake_amount,
            commission_percentage,
            join_during_genesis,
        });
    };

    let operator_file = dir.join(OPERATOR_FILE);
    let operator_file = operator_file.as_path();
    let operator_config = client.get::<StringOperatorConfiguration>(operator_file)?;

    // Check and convert fields in operator file
    let operator_account_address_from_file = parse_required_option(
        &operator_config.operator_account_address,
        operator_file,
        "operator_account_address",
        AccountAddress::from_str,
    )?;
    let operator_account_public_key_from_file = parse_required_option(
        &operator_config.operator_account_public_key,
        operator_file,
        "operator_account_public_key",
        Ed25519PublicKey::from_encoded_string,
    )?;
    let consensus_public_key = parse_required_option(
        &operator_config.consensus_public_key,
        operator_file,
        "consensus_public_key",
        bls12381::PublicKey::from_encoded_string,
    )?;
    let consensus_proof_of_possession = parse_required_option(
        &operator_config.consensus_proof_of_possession,
        operator_file,
        "consensus_proof_of_possession",
        bls12381::ProofOfPossession::from_encoded_string,
    )?;
    let validator_network_public_key = parse_required_option(
        &operator_config.validator_network_public_key,
        operator_file,
        "validator_network_public_key",
        x25519::PublicKey::from_encoded_string,
    )?;
    let full_node_network_public_key = parse_optional_option(
        &operator_config.full_node_network_public_key,
        operator_file,
        "full_node_network_public_key",
        x25519::PublicKey::from_encoded_string,
    )?;

    // Verify owner & operator agree on operator
    if operator_account_address != operator_account_address_from_file {
        return Err(
            CliError::CommandArgumentError(
                format!("Operator account {} in owner file {} does not match operator account {} in operator file {}",
                        operator_account_address,
                        owner_file.display(),
                        operator_account_address_from_file,
                        operator_file.display()
                )));
    }
    if operator_account_public_key != operator_account_public_key_from_file {
        return Err(
            CliError::CommandArgumentError(
                format!("Operator public key {} in owner file {} does not match operator public key {} in operator file {}",
                        operator_account_public_key,
                        owner_file.display(),
                        operator_account_public_key_from_file,
                        operator_file.display()
                )));
    }

    // Build Validator configuration
    Ok(ValidatorConfiguration {
        owner_account_address,
        owner_account_public_key,
        operator_account_address,
        operator_account_public_key,
        voter_account_address,
        voter_account_public_key,
        consensus_public_key: Some(consensus_public_key),
        proof_of_possession: Some(consensus_proof_of_possession),
        validator_network_public_key: Some(validator_network_public_key),
        validator_host: Some(operator_config.validator_host),
        full_node_network_public_key,
        full_node_host: operator_config.full_node_host,
        stake_amount,
        commission_percentage,
        join_during_genesis,
    })
}

fn parse_required_option<F: Fn(&str) -> Result<T, E>, T, E: std::fmt::Display>(
    option: &Option<String>,
    file: &Path,
    field_name: &'static str,
    parse: F,
) -> Result<T, CliError> {
    if let Some(ref field) = option {
        parse(field).map_err(|err| {
            CliError::CommandArgumentError(format!(
                "Field {} is invalid in file {}.  Err: {}",
                field_name,
                file.display(),
                err
            ))
        })
    } else {
        Err(CliError::CommandArgumentError(format!(
            "File {} is missing {}",
            file.display(),
            field_name
        )))
    }
}

fn parse_optional_option<F: Fn(&str) -> Result<T, E>, T, E: std::fmt::Display>(
    option: &Option<String>,
    file: &Path,
    field_name: &'static str,
    parse: F,
) -> Result<Option<T>, CliError> {
    if let Some(ref field) = option {
        parse(field)
            .map_err(|err| {
                CliError::CommandArgumentError(format!(
                    "Field {} is invalid in file {}.  Err: {}",
                    field_name,
                    file.display(),
                    err
                ))
            })
            .map(Some)
    } else {
        Ok(None)
    }
}
