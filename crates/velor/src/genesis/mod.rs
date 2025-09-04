// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod git;
pub mod keys;
#[cfg(test)]
mod tests;
pub mod tools;

use crate::{
    common::{
        types::{CliError, CliTypedResult, PromptOptions},
        utils::{check_if_file_exists, dir_default_to_current, write_to_file},
    },
    genesis::git::{
        Client, GitOptions, BALANCES_FILE, EMPLOYEE_VESTING_ACCOUNTS_FILE, LAYOUT_FILE,
        OPERATOR_FILE, OWNER_FILE,
    },
    CliCommand, CliResult,
};
use velor_crypto::{
    bls12381, ed25519::ED25519_PUBLIC_KEY_LENGTH, x25519, ValidCryptoMaterial,
    ValidCryptoMaterialStringExt,
};
use velor_genesis::{
    builder::GenesisConfiguration,
    config::{
        AccountBalanceMap, EmployeePoolMap, HostAndPort, Layout, StringOperatorConfiguration,
        StringOwnerConfiguration, ValidatorConfiguration,
    },
    mainnet::MainnetGenesisInfo,
    GenesisInfo,
};
use velor_logger::info;
use velor_types::{
    account_address::{AccountAddress, AccountAddressWithChecks},
    on_chain_config::{OnChainConsensusConfig, OnChainExecutionConfig},
};
use velor_vm_genesis::{default_gas_schedule, AccountBalance, EmployeePool};
use async_trait::async_trait;
use clap::Parser;
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, HashSet},
    path::{Path, PathBuf},
    str::FromStr,
};

const WAYPOINT_FILE: &str = "waypoint.txt";
const GENESIS_FILE: &str = "genesis.blob";

/// Tool for setting up an Velor chain Genesis transaction
///
/// This tool sets up a space for multiple initial "validator"
/// accounts to build a genesis transaction for a new chain.
#[derive(Parser)]
pub enum GenesisTool {
    GenerateAdminWriteSet(keys::GenerateAdminWriteSet),
    GenerateGenesis(GenerateGenesis),
    GetPoolAddresses(tools::PoolAddresses),
    GenerateKeys(keys::GenerateKeys),
    GenerateLayoutTemplate(keys::GenerateLayoutTemplate),
    SetupGit(git::SetupGit),
    SetValidatorConfiguration(keys::SetValidatorConfiguration),
}

impl GenesisTool {
    pub async fn execute(self) -> CliResult {
        match self {
            GenesisTool::GenerateAdminWriteSet(tool) => tool.execute_serialized_success().await,
            GenesisTool::GenerateGenesis(tool) => tool.execute_serialized().await,
            GenesisTool::GetPoolAddresses(tool) => tool.execute_serialized().await,
            GenesisTool::GenerateKeys(tool) => tool.execute_serialized().await,
            GenesisTool::GenerateLayoutTemplate(tool) => tool.execute_serialized_success().await,
            GenesisTool::SetupGit(tool) => tool.execute_serialized_success().await,
            GenesisTool::SetValidatorConfiguration(tool) => tool.execute_serialized_success().await,
        }
    }
}

/// Generate genesis from a git repository
///
/// This will create a genesis.blob and a waypoint.txt to be used for
/// running a network
#[derive(Parser)]
pub struct GenerateGenesis {
    /// Output directory for Genesis file and waypoint
    #[clap(long, value_parser)]
    output_dir: Option<PathBuf>,
    /// Whether this is mainnet genesis.
    ///
    /// Default is false
    #[clap(long)]
    mainnet: bool,

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
        let (genesis_bytes, waypoint) = if self.mainnet {
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

    if layout.root_key.is_some() {
        return Err(CliError::UnexpectedError(
            "Root key must not be set for mainnet.".to_string(),
        ));
    }

    let total_supply = layout.total_supply.ok_or_else(|| {
        CliError::UnexpectedError("Layout file does not have `total_supply`".to_string())
    })?;

    let account_balance_map: AccountBalanceMap = client.get(Path::new(BALANCES_FILE))?;
    let accounts: Vec<AccountBalance> = account_balance_map.try_into()?;

    // Check that the supply matches the total
    let total_balance_supply: u64 = accounts.iter().map(|inner| inner.balance).sum();
    if total_supply != total_balance_supply {
        return Err(CliError::UnexpectedError(format!(
            "Total supply seen {} doesn't match expected total supply {}",
            total_balance_supply, total_supply
        )));
    }

    // Check that the user has a reasonable amount of APT, since below the minimum gas amount is
    // not useful 1 APT minimally
    const MIN_USEFUL_AMOUNT: u64 = 200000000;
    let ten_percent_of_total = total_supply / 10;
    for account in accounts.iter() {
        if account.balance != 0 && account.balance < MIN_USEFUL_AMOUNT {
            return Err(CliError::UnexpectedError(format!(
                "Account {} has an initial supply below expected amount {} < {}",
                account.account_address, account.balance, MIN_USEFUL_AMOUNT
            )));
        } else if account.balance > ten_percent_of_total {
            return Err(CliError::UnexpectedError(format!(
                "Account {} has an more than 10% of the total balance {} > {}",
                account.account_address, account.balance, ten_percent_of_total
            )));
        }
    }

    // Keep track of accounts for later lookup of balances
    let initialized_accounts: BTreeMap<AccountAddress, u64> = accounts
        .iter()
        .map(|inner| (inner.account_address, inner.balance))
        .collect();

    let employee_vesting_accounts: EmployeePoolMap =
        client.get(Path::new(EMPLOYEE_VESTING_ACCOUNTS_FILE))?;

    let employee_validators: Vec<_> = employee_vesting_accounts
        .inner
        .iter()
        .map(|inner| inner.validator.clone())
        .collect();
    let employee_vesting_accounts: Vec<EmployeePool> = employee_vesting_accounts.try_into()?;
    let validators = get_validator_configs(&client, &layout, true).map_err(parse_error)?;
    let mut unique_accounts = BTreeSet::new();
    let mut unique_network_keys = HashSet::new();
    let mut unique_consensus_keys = HashSet::new();
    let mut unique_consensus_pop = HashSet::new();
    let mut unique_hosts = HashSet::new();

    validate_employee_accounts(
        &employee_vesting_accounts,
        &initialized_accounts,
        &mut unique_accounts,
    )?;

    let mut seen_owners = BTreeMap::new();
    validate_validators(
        &layout,
        &employee_validators,
        &initialized_accounts,
        &mut unique_accounts,
        &mut unique_network_keys,
        &mut unique_consensus_keys,
        &mut unique_consensus_pop,
        &mut unique_hosts,
        &mut seen_owners,
        true,
    )?;
    validate_validators(
        &layout,
        &validators,
        &initialized_accounts,
        &mut unique_accounts,
        &mut unique_network_keys,
        &mut unique_consensus_keys,
        &mut unique_consensus_pop,
        &mut unique_hosts,
        &mut seen_owners,
        false,
    )?;

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
            employee_vesting_start: layout.employee_vesting_start,
            employee_vesting_period_duration: layout.employee_vesting_period_duration,
            consensus_config: OnChainConsensusConfig::default_for_genesis(),
            execution_config: OnChainExecutionConfig::default_for_genesis(),
            gas_schedule: default_gas_schedule(),
            initial_features_override: None,
            randomness_config_override: None,
            jwk_consensus_config_override: None,
            initial_jwks: vec![],
            keyless_groth16_vk: None,
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
            employee_vesting_start: layout.employee_vesting_start,
            employee_vesting_period_duration: layout.employee_vesting_period_duration,
            consensus_config: layout.on_chain_consensus_config,
            execution_config: layout.on_chain_execution_config,
            gas_schedule: default_gas_schedule(),
            initial_features_override: None,
            randomness_config_override: None,
            jwk_consensus_config_override: layout.jwk_consensus_config_override.clone(),
            initial_jwks: layout.initial_jwks.clone(),
            keyless_groth16_vk: layout.keyless_groth16_vk_override.clone(),
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
            },
            Err(failure) => {
                if let CliError::UnexpectedError(failure) = failure {
                    errors.push(format!("{}: {}", user, failure));
                } else {
                    errors.push(format!("{}: {:?}", user, failure));
                }
            },
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
    let owner_account_address: AccountAddress = parse_required_option(
        &owner_config.owner_account_address,
        owner_file,
        "owner_account_address",
        AccountAddressWithChecks::from_str,
    )?
    .into();
    let owner_account_public_key = parse_required_option(
        &owner_config.owner_account_public_key,
        owner_file,
        "owner_account_public_key",
        |str| parse_key(ED25519_PUBLIC_KEY_LENGTH, str),
    )?;

    let operator_account_address: AccountAddress = parse_required_option(
        &owner_config.operator_account_address,
        owner_file,
        "operator_account_address",
        AccountAddressWithChecks::from_str,
    )?
    .into();
    let operator_account_public_key = parse_required_option(
        &owner_config.operator_account_public_key,
        owner_file,
        "operator_account_public_key",
        |str| parse_key(ED25519_PUBLIC_KEY_LENGTH, str),
    )?;

    let voter_account_address: AccountAddress = parse_required_option(
        &owner_config.voter_account_address,
        owner_file,
        "voter_account_address",
        AccountAddressWithChecks::from_str,
    )?
    .into();
    let voter_account_public_key = parse_required_option(
        &owner_config.voter_account_public_key,
        owner_file,
        "voter_account_public_key",
        |str| parse_key(ED25519_PUBLIC_KEY_LENGTH, str),
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
            owner_account_address: owner_account_address.into(),
            owner_account_public_key,
            operator_account_address: operator_account_address.into(),
            operator_account_public_key,
            voter_account_address: voter_account_address.into(),
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
    let operator_account_address_from_file: AccountAddress = parse_required_option(
        &operator_config.operator_account_address,
        operator_file,
        "operator_account_address",
        AccountAddressWithChecks::from_str,
    )?
    .into();
    let operator_account_public_key_from_file = parse_required_option(
        &operator_config.operator_account_public_key,
        operator_file,
        "operator_account_public_key",
        |str| parse_key(ED25519_PUBLIC_KEY_LENGTH, str),
    )?;
    let consensus_public_key = parse_required_option(
        &operator_config.consensus_public_key,
        operator_file,
        "consensus_public_key",
        |str| parse_key(bls12381::PublicKey::LENGTH, str),
    )?;
    let consensus_proof_of_possession = parse_required_option(
        &operator_config.consensus_proof_of_possession,
        operator_file,
        "consensus_proof_of_possession",
        |str| parse_key(bls12381::ProofOfPossession::LENGTH, str),
    )?;
    let validator_network_public_key = parse_required_option(
        &operator_config.validator_network_public_key,
        operator_file,
        "validator_network_public_key",
        |str| parse_key(ED25519_PUBLIC_KEY_LENGTH, str),
    )?;
    let full_node_network_public_key = parse_optional_option(
        &operator_config.full_node_network_public_key,
        operator_file,
        "full_node_network_public_key",
        |str| parse_key(ED25519_PUBLIC_KEY_LENGTH, str),
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
        owner_account_address: owner_account_address.into(),
        owner_account_public_key,
        operator_account_address: operator_account_address.into(),
        operator_account_public_key,
        voter_account_address: voter_account_address.into(),
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

// TODO: Move into the Crypto libraries
fn parse_key<T: ValidCryptoMaterial>(num_bytes: usize, str: &str) -> anyhow::Result<T> {
    let num_chars: usize = num_bytes * 2;
    let mut working = str.trim();

    // Checks if it has a 0x at the beginning, which is okay
    if working.starts_with("0x") {
        working = &working[2..];
    }

    match working.len().cmp(&num_chars) {
        Ordering::Less => {
            anyhow::bail!(
                "Key {} is too short {} must be {} hex characters",
                str,
                working.len(),
                num_chars
            )
        },
        Ordering::Greater => {
            anyhow::bail!(
                "Key {} is too long {} must be {} hex characters with or without a 0x in front",
                str,
                working.len(),
                num_chars
            )
        },
        Ordering::Equal => {},
    }

    if !working.chars().all(|c| char::is_ascii_hexdigit(&c)) {
        anyhow::bail!("Key {} contains a non-hex character", str)
    }

    Ok(T::from_encoded_string(str.trim())?)
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

fn validate_validators(
    layout: &Layout,
    validators: &[ValidatorConfiguration],
    initialized_accounts: &BTreeMap<AccountAddress, u64>,
    unique_accounts: &mut BTreeSet<AccountAddress>,
    unique_network_keys: &mut HashSet<x25519::PublicKey>,
    unique_consensus_keys: &mut HashSet<bls12381::PublicKey>,
    unique_consensus_pops: &mut HashSet<bls12381::ProofOfPossession>,
    unique_hosts: &mut HashSet<HostAndPort>,
    seen_owners: &mut BTreeMap<AccountAddress, usize>,
    is_pooled_validator: bool,
) -> CliTypedResult<()> {
    // check accounts for validators
    let mut errors = vec![];

    for (i, validator) in validators.iter().enumerate() {
        let name = if is_pooled_validator {
            format!("Employee Pool #{}", i)
        } else {
            layout.users.get(i).unwrap().to_string()
        };

        if !initialized_accounts.contains_key(&validator.owner_account_address.into()) {
            errors.push(CliError::UnexpectedError(format!(
                "Owner {} in validator {} is not in the balances.yaml file",
                validator.owner_account_address, name
            )));
        }
        if !initialized_accounts.contains_key(&validator.operator_account_address.into()) {
            errors.push(CliError::UnexpectedError(format!(
                "Operator {} in validator {} is not in the balances.yaml file",
                validator.operator_account_address, name
            )));
        }
        if !initialized_accounts.contains_key(&validator.voter_account_address.into()) {
            errors.push(CliError::UnexpectedError(format!(
                "Voter {} in validator {} is not in the balances.yaml file",
                validator.voter_account_address, name
            )));
        }

        let owner_balance = initialized_accounts
            .get(&validator.owner_account_address.into())
            .unwrap();

        if seen_owners.contains_key(&validator.owner_account_address.into()) {
            errors.push(CliError::UnexpectedError(format!(
                "Owner {} in validator {} has been seen before as an owner of validator {}",
                validator.owner_account_address,
                name,
                seen_owners
                    .get(&validator.owner_account_address.into())
                    .unwrap()
            )));
        }
        seen_owners.insert(validator.owner_account_address.into(), i);

        if unique_accounts.contains(&validator.owner_account_address.into()) {
            errors.push(CliError::UnexpectedError(format!(
                "Owner '{}' in validator {} has already been seen elsewhere",
                validator.owner_account_address, name
            )));
        }
        unique_accounts.insert(validator.owner_account_address.into());

        if unique_accounts.contains(&validator.operator_account_address.into()) {
            errors.push(CliError::UnexpectedError(format!(
                "Operator '{}' in validator {} has already been seen elsewhere",
                validator.operator_account_address, name
            )));
        }
        unique_accounts.insert(validator.operator_account_address.into());

        // Pooled validators have a combined balance
        // TODO: Make this field optional but checked
        if !is_pooled_validator && *owner_balance < validator.stake_amount {
            errors.push(CliError::UnexpectedError(format!(
                "Owner {} in validator {} has less in it's balance {} than the stake amount for the validator {}",
                validator.owner_account_address, name, owner_balance, validator.stake_amount
            )));
        }
        if validator.stake_amount < layout.min_stake {
            errors.push(CliError::UnexpectedError(format!(
                "Validator {} has stake {} under the min stake {}",
                name, validator.stake_amount, layout.min_stake
            )));
        }
        if validator.stake_amount > layout.max_stake {
            errors.push(CliError::UnexpectedError(format!(
                "Validator {} has stake {} over the max stake {}",
                name, validator.stake_amount, layout.max_stake
            )));
        }

        // Ensure that the validator is setup correctly if it's joining in genesis
        if validator.join_during_genesis {
            if validator.validator_network_public_key.is_none() {
                errors.push(CliError::UnexpectedError(format!(
                    "Validator {} does not have a validator network public key, though it's joining during genesis",
                    name
                )));
            }
            if !unique_network_keys.insert(validator.validator_network_public_key.unwrap()) {
                errors.push(CliError::UnexpectedError(format!(
                    "Validator {} has a repeated validator network key{}",
                    name,
                    validator.validator_network_public_key.unwrap()
                )));
            }

            if validator.validator_host.is_none() {
                errors.push(CliError::UnexpectedError(format!(
                    "Validator {} does not have a validator host, though it's joining during genesis",
                    name
                )));
            }
            if !unique_hosts.insert(validator.validator_host.as_ref().unwrap().clone()) {
                errors.push(CliError::UnexpectedError(format!(
                    "Validator {} has a repeated validator host {:?}",
                    name,
                    validator.validator_host.as_ref().unwrap()
                )));
            }

            if validator.consensus_public_key.is_none() {
                errors.push(CliError::UnexpectedError(format!(
                    "Validator {} does not have a consensus public key, though it's joining during genesis",
                    name
                )));
            }
            if !unique_consensus_keys
                .insert(validator.consensus_public_key.as_ref().unwrap().clone())
            {
                errors.push(CliError::UnexpectedError(format!(
                    "Validator {} has a repeated a consensus public key {}",
                    name,
                    validator.consensus_public_key.as_ref().unwrap()
                )));
            }

            if validator.proof_of_possession.is_none() {
                errors.push(CliError::UnexpectedError(format!(
                    "Validator {} does not have a consensus proof of possession, though it's joining during genesis",
                    name
                )));
            }
            if !unique_consensus_pops
                .insert(validator.proof_of_possession.as_ref().unwrap().clone())
            {
                errors.push(CliError::UnexpectedError(format!(
                    "Validator {} has a repeated a consensus proof of possessions {}",
                    name,
                    validator.proof_of_possession.as_ref().unwrap()
                )));
            }

            match (
                validator.full_node_host.as_ref(),
                validator.full_node_network_public_key.as_ref(),
            ) {
                (None, None) => {
                    info!("Validator {} does not have a full node setup", name);
                },
                (Some(_), None) | (None, Some(_)) => {
                    errors.push(CliError::UnexpectedError(format!(
                        "Validator {} has a full node host or public key but not both",
                        name
                    )));
                },
                (Some(full_node_host), Some(full_node_network_public_key)) => {
                    // Ensure that the validator and the full node aren't the same
                    let validator_host = validator.validator_host.as_ref().unwrap();
                    let validator_network_public_key =
                        validator.validator_network_public_key.as_ref().unwrap();
                    if validator_host == full_node_host {
                        errors.push(CliError::UnexpectedError(format!(
                            "Validator {} has a validator and a full node host that are the same {:?}",
                            name,
                            validator_host
                        )));
                    }
                    if !unique_hosts.insert(validator.full_node_host.as_ref().unwrap().clone()) {
                        errors.push(CliError::UnexpectedError(format!(
                            "Validator {} has a repeated full node host {:?}",
                            name,
                            validator.full_node_host.as_ref().unwrap()
                        )));
                    }

                    if validator_network_public_key == full_node_network_public_key {
                        errors.push(CliError::UnexpectedError(format!(
                            "Validator {} has a validator and a full node network public key that are the same {}",
                            name,
                            validator_network_public_key
                        )));
                    }
                    if !unique_network_keys.insert(validator.full_node_network_public_key.unwrap())
                    {
                        errors.push(CliError::UnexpectedError(format!(
                            "Validator {} has a repeated full node network key {}",
                            name,
                            validator.full_node_network_public_key.unwrap()
                        )));
                    }
                },
            }
        } else {
            if validator.validator_network_public_key.is_some() {
                errors.push(CliError::UnexpectedError(format!(
                    "Validator {} has a validator network public key, but it is *NOT* joining during genesis",
                    name
                )));
            }
            if validator.validator_host.is_some() {
                errors.push(CliError::UnexpectedError(format!(
                    "Validator {} has a validator host, but it is *NOT* joining during genesis",
                    name
                )));
            }
            if validator.consensus_public_key.is_some() {
                errors.push(CliError::UnexpectedError(format!(
                    "Validator {} has a consensus public key, but it is *NOT* joining during genesis",
                    name
                )));
            }
            if validator.proof_of_possession.is_some() {
                errors.push(CliError::UnexpectedError(format!(
                    "Validator {} has a consensus proof of possession, but it is *NOT* joining during genesis",
                    name
                )));
            }
            if validator.full_node_network_public_key.is_some() {
                errors.push(CliError::UnexpectedError(format!(
                    "Validator {} has a full node public key, but it is *NOT* joining during genesis",
                    name
                )));
            }
            if validator.full_node_host.is_some() {
                errors.push(CliError::UnexpectedError(format!(
                    "Validator {} has a full node host, but it is *NOT* joining during genesis",
                    name
                )));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        eprintln!("{:#?}", errors);

        Err(CliError::UnexpectedError(
            "Failed to validate validators".to_string(),
        ))
    }
}

fn validate_employee_accounts(
    employee_vesting_accounts: &[EmployeePool],
    initialized_accounts: &BTreeMap<AccountAddress, u64>,
    unique_accounts: &mut BTreeSet<AccountAddress>,
) -> CliTypedResult<()> {
    // Check accounts for employee accounts
    for (i, pool) in employee_vesting_accounts.iter().enumerate() {
        let mut total_stake_pool_amount = 0;
        for (j, account) in pool.accounts.iter().enumerate() {
            if !initialized_accounts.contains_key(account) {
                return Err(CliError::UnexpectedError(format!(
                    "Account #{} '{}' in employee pool #{} is not in the balances.yaml file",
                    j, account, i
                )));
            }
            if unique_accounts.contains(account) {
                return Err(CliError::UnexpectedError(format!(
                    "Account #{} '{}' in employee pool #{} has already been seen elsewhere",
                    j, account, i
                )));
            }
            unique_accounts.insert(*account);

            total_stake_pool_amount += initialized_accounts.get(account).unwrap();
        }

        if total_stake_pool_amount != pool.validator.validator.stake_amount {
            return Err(CliError::UnexpectedError(format!(
                "Stake amount {} in employee pool #{} does not match combined of accounts {}",
                pool.validator.validator.stake_amount, i, total_stake_pool_amount
            )));
        }

        if !initialized_accounts.contains_key(&pool.validator.validator.owner_address) {
            return Err(CliError::UnexpectedError(format!(
                "Owner address {} in employee pool #{} is not in the balances.yaml file",
                pool.validator.validator.owner_address, i
            )));
        }
        if !initialized_accounts.contains_key(&pool.validator.validator.operator_address) {
            return Err(CliError::UnexpectedError(format!(
                "Operator address {} in employee pool #{} is not in the balances.yaml file",
                pool.validator.validator.operator_address, i
            )));
        }
        if !initialized_accounts.contains_key(&pool.validator.validator.voter_address) {
            return Err(CliError::UnexpectedError(format!(
                "Voter address {} in employee pool #{} is not in the balances.yaml file",
                pool.validator.validator.voter_address, i
            )));
        }
        if !initialized_accounts.contains_key(&pool.beneficiary_resetter) {
            return Err(CliError::UnexpectedError(format!(
                "Beneficiary resetter {} in employee pool #{} is not in the balances.yaml file",
                pool.beneficiary_resetter, i
            )));
        }
    }
    Ok(())
}
