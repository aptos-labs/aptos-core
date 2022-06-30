// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        types::{
            CliCommand, CliError, CliResult, CliTypedResult, ProfileOptions, RestOptions,
            TransactionOptions,
        },
        utils::read_from_file,
    },
    genesis::git::from_yaml,
};
use aptos_crypto::{ed25519::Ed25519PublicKey, x25519, ValidCryptoMaterialStringExt};
use aptos_genesis::config::{HostAndPort, ValidatorConfiguration};
use aptos_rest_client::Transaction;
use aptos_types::{account_address::AccountAddress, account_config::aptos_root_address};
use async_trait::async_trait;
use clap::Parser;
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

/// Tool for manipulating nodes
///
#[derive(Parser)]
pub enum NodeTool {
    AddStake(AddStake),
    UnlockStake(UnlockStake),
    WithdrawStake(WithdrawStake),
    IncreaseLockup(IncreaseLockup),
    RegisterValidatorCandidate(RegisterValidatorCandidate),
    JoinValidatorSet(JoinValidatorSet),
    LeaveValidatorSet(LeaveValidatorSet),
    ShowValidatorConfig(ShowValidatorConfig),
    ShowValidatorSet(ShowValidatorSet),
    ShowValidatorStake(ShowValidatorStake),
}

impl NodeTool {
    pub async fn execute(self) -> CliResult {
        use NodeTool::*;
        match self {
            AddStake(tool) => tool.execute_serialized().await,
            UnlockStake(tool) => tool.execute_serialized().await,
            WithdrawStake(tool) => tool.execute_serialized().await,
            IncreaseLockup(tool) => tool.execute_serialized().await,
            RegisterValidatorCandidate(tool) => tool.execute_serialized().await,
            JoinValidatorSet(tool) => tool.execute_serialized().await,
            LeaveValidatorSet(tool) => tool.execute_serialized().await,
            ShowValidatorSet(tool) => tool.execute_serialized().await,
            ShowValidatorStake(tool) => tool.execute_serialized().await,
            ShowValidatorConfig(tool) => tool.execute_serialized().await,
        }
    }
}

/// Stake coins for an account to the stake pool
#[derive(Parser)]
pub struct AddStake {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    /// Amount of coins to add to stake
    #[clap(long)]
    pub amount: u64,
}

#[async_trait]
impl CliCommand<Transaction> for AddStake {
    fn command_name(&self) -> &'static str {
        "AddStake"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        self.txn_options
            .submit_script_function(
                AccountAddress::ONE,
                "Stake",
                "add_stake",
                vec![],
                vec![bcs::to_bytes(&self.amount)?],
            )
            .await
    }
}

/// Unlock staked coins
///
/// Coins can only be unlocked if they no longer have an applied lockup period
#[derive(Parser)]
pub struct UnlockStake {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    /// Amount of coins to unlock
    #[clap(long)]
    pub amount: u64,
}

#[async_trait]
impl CliCommand<Transaction> for UnlockStake {
    fn command_name(&self) -> &'static str {
        "UnlockStake"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        self.txn_options
            .submit_script_function(
                AccountAddress::ONE,
                "Stake",
                "unlock",
                vec![],
                vec![bcs::to_bytes(&self.amount)?],
            )
            .await
    }
}

/// Withdraw all unlocked staked coins
///
/// Before calling `WithdrawStake`, `UnlockStake` must be called first.
#[derive(Parser)]
pub struct WithdrawStake {
    #[clap(flatten)]
    pub(crate) node_op_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<Transaction> for WithdrawStake {
    fn command_name(&self) -> &'static str {
        "WithdrawStake"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        self.node_op_options
            .submit_script_function(AccountAddress::ONE, "Stake", "withdraw", vec![], vec![])
            .await
    }
}

/// Increase lockup of all staked coins in an account
#[derive(Parser)]
pub struct IncreaseLockup {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    /// Number of seconds to increase the lockup period by
    #[clap(long)]
    pub(crate) lockup_timestamp_secs: u64,
}

#[async_trait]
impl CliCommand<Transaction> for IncreaseLockup {
    fn command_name(&self) -> &'static str {
        "IncreaseLockup"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        if self.lockup_timestamp_secs
            <= SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
        {
            return Err(CliError::CommandArgumentError(
                "--lockup-timestamp-secs is in the past".to_string(),
            ));
        }

        self.txn_options
            .submit_script_function(
                AccountAddress::ONE,
                "Stake",
                "increase_lockup",
                vec![],
                vec![bcs::to_bytes(&self.lockup_timestamp_secs)?],
            )
            .await
    }
}

/// Register the current account as a Validator candidate
#[derive(Parser)]
pub struct RegisterValidatorCandidate {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    /// Validator Configuration file, created from the `genesis set-validator-configuration` command
    #[clap(long)]
    pub(crate) validator_config_file: Option<PathBuf>,
    /// Hex encoded Consensus public key
    #[clap(long, parse(try_from_str = Ed25519PublicKey::from_encoded_string))]
    pub(crate) consensus_public_key: Option<Ed25519PublicKey>,
    /// Host and port pair for the validator e.g. 127.0.0.1:6180
    #[clap(long)]
    pub(crate) validator_host: Option<HostAndPort>,
    /// Validator x25519 public network key
    #[clap(long, parse(try_from_str = x25519::PublicKey::from_encoded_string))]
    pub(crate) validator_network_public_key: Option<x25519::PublicKey>,
    /// Host and port pair for the fullnode e.g. 127.0.0.1:6180.  Optional
    #[clap(long)]
    pub(crate) full_node_host: Option<HostAndPort>,
    /// Full node x25519 public network key
    #[clap(long, parse(try_from_str = x25519::PublicKey::from_encoded_string))]
    pub(crate) full_node_network_public_key: Option<x25519::PublicKey>,
}

impl RegisterValidatorCandidate {
    fn process_inputs(
        &self,
    ) -> CliTypedResult<(
        Ed25519PublicKey,
        x25519::PublicKey,
        Option<x25519::PublicKey>,
        HostAndPort,
        Option<HostAndPort>,
    )> {
        let validator_config = self.read_validator_config()?;

        let consensus_public_key = if let Some(ref consensus_public_key) = self.consensus_public_key
        {
            consensus_public_key.clone()
        } else if let Some(ref validator_config) = validator_config {
            validator_config.consensus_public_key.clone()
        } else {
            return Err(CliError::CommandArgumentError(
                "Must provide either --validator-config-file or --consensus-public-key".to_string(),
            ));
        };

        let validator_network_public_key =
            if let Some(public_key) = self.validator_network_public_key {
                public_key
            } else if let Some(ref validator_config) = validator_config {
                validator_config.validator_network_public_key
            } else {
                return Err(CliError::CommandArgumentError(
                    "Must provide either --validator-config-file or --validator-network-public-key"
                        .to_string(),
                ));
            };

        let full_node_network_public_key =
            if let Some(public_key) = self.full_node_network_public_key {
                Some(public_key)
            } else if let Some(ref validator_config) = validator_config {
                validator_config.full_node_network_public_key
            } else {
                None
            };

        let validator_host = if let Some(ref host) = self.validator_host {
            host.clone()
        } else if let Some(ref validator_config) = validator_config {
            validator_config.validator_host.clone()
        } else {
            return Err(CliError::CommandArgumentError(
                "Must provide either --validator-config-file or --validator-host".to_string(),
            ));
        };

        let full_node_host = if let Some(ref host) = self.full_node_host {
            Some(host.clone())
        } else if let Some(ref validator_config) = validator_config {
            validator_config.full_node_host.clone()
        } else {
            None
        };

        Ok((
            consensus_public_key,
            validator_network_public_key,
            full_node_network_public_key,
            validator_host,
            full_node_host,
        ))
    }

    fn read_validator_config(&self) -> CliTypedResult<Option<ValidatorConfiguration>> {
        if let Some(ref file) = self.validator_config_file {
            Ok(from_yaml(
                &String::from_utf8(read_from_file(file)?).map_err(CliError::from)?,
            )?)
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl CliCommand<Transaction> for RegisterValidatorCandidate {
    fn command_name(&self) -> &'static str {
        "RegisterValidatorCandidate"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        let (
            consensus_public_key,
            validator_network_public_key,
            full_node_network_public_key,
            validator_host,
            full_node_host,
        ) = self.process_inputs()?;
        let validator_network_addresses =
            vec![validator_host.as_network_address(validator_network_public_key)?];
        let full_node_network_addresses =
            match (full_node_host.as_ref(), full_node_network_public_key) {
                (Some(host), Some(public_key)) => vec![host.as_network_address(public_key)?],
                _ => vec![],
            };

        self.txn_options
            .submit_script_function(
                AccountAddress::ONE,
                "Stake",
                "register_validator_candidate",
                vec![],
                vec![
                    bcs::to_bytes(&consensus_public_key)?,
                    // Double BCS encode, so that we can hide the original type
                    bcs::to_bytes(&bcs::to_bytes(&validator_network_addresses)?)?,
                    bcs::to_bytes(&bcs::to_bytes(&full_node_network_addresses)?)?,
                ],
            )
            .await
    }
}

/// Arguments used for operator of the staking pool
#[derive(Parser)]
pub struct OperatorArgs {
    /// Address of the Staking pool
    #[clap(long)]
    pub(crate) pool_address: Option<AccountAddress>,
}

impl OperatorArgs {
    fn address(&self, profile_options: &ProfileOptions) -> CliTypedResult<AccountAddress> {
        if let Some(address) = self.pool_address {
            Ok(address)
        } else {
            profile_options.account_address()
        }
    }
}

/// Join the validator set after meeting staking requirements
#[derive(Parser)]
pub struct JoinValidatorSet {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    #[clap(flatten)]
    pub(crate) operator_args: OperatorArgs,
}

#[async_trait]
impl CliCommand<Transaction> for JoinValidatorSet {
    fn command_name(&self) -> &'static str {
        "JoinValidatorSet"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        let address = self
            .operator_args
            .address(&self.txn_options.profile_options)?;

        self.txn_options
            .submit_script_function(
                AccountAddress::ONE,
                "Stake",
                "join_validator_set",
                vec![],
                vec![bcs::to_bytes(&address)?],
            )
            .await
    }
}

/// Leave the validator set
#[derive(Parser)]
pub struct LeaveValidatorSet {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    #[clap(flatten)]
    pub(crate) operator_args: OperatorArgs,
}

#[async_trait]
impl CliCommand<Transaction> for LeaveValidatorSet {
    fn command_name(&self) -> &'static str {
        "LeaveValidatorSet"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        let address = self
            .operator_args
            .address(&self.txn_options.profile_options)?;

        self.txn_options
            .submit_script_function(
                AccountAddress::ONE,
                "Stake",
                "leave_validator_set",
                vec![],
                vec![bcs::to_bytes(&address)?],
            )
            .await
    }
}

/// Show validator details of the current validator
#[derive(Parser)]
pub struct ShowValidatorStake {
    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,
    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
    #[clap(flatten)]
    pub(crate) operator_args: OperatorArgs,
}

#[async_trait]
impl CliCommand<serde_json::Value> for ShowValidatorStake {
    fn command_name(&self) -> &'static str {
        "ShowValidatorStake"
    }

    async fn execute(mut self) -> CliTypedResult<serde_json::Value> {
        let client = self.rest_options.client(&self.profile_options.profile)?;
        let address = self.operator_args.address(&self.profile_options)?;
        let response = client
            .get_resource(address, "0x1::Stake::StakePool")
            .await?;
        Ok(response.into_inner())
    }
}

/// Show validator details of the current validator
#[derive(Parser)]
pub struct ShowValidatorConfig {
    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,
    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
    #[clap(flatten)]
    pub(crate) operator_args: OperatorArgs,
}

#[async_trait]
impl CliCommand<serde_json::Value> for ShowValidatorConfig {
    fn command_name(&self) -> &'static str {
        "ShowValidatorConfig"
    }

    async fn execute(mut self) -> CliTypedResult<serde_json::Value> {
        let client = self.rest_options.client(&self.profile_options.profile)?;
        let address = self.operator_args.address(&self.profile_options)?;
        let response = client
            .get_resource(address, "0x1::Stake::ValidatorConfig")
            .await?;
        Ok(response.into_inner())
    }
}

/// Show validator details of the validator set
#[derive(Parser)]
pub struct ShowValidatorSet {
    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,
    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
}

#[async_trait]
impl CliCommand<serde_json::Value> for ShowValidatorSet {
    fn command_name(&self) -> &'static str {
        "ShowValidatorSet"
    }

    async fn execute(mut self) -> CliTypedResult<serde_json::Value> {
        let client = self.rest_options.client(&self.profile_options.profile)?;
        let response = client
            .get_resource(aptos_root_address(), "0x1::Stake::ValidatorSet")
            .await?;
        Ok(response.into_inner())
    }
}
