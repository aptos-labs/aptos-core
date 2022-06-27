// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        types::{CliCommand, CliError, CliResult, CliTypedResult, TransactionOptions},
        utils::read_from_file,
    },
    genesis::git::from_yaml,
};
use aptos_crypto::{ed25519::Ed25519PublicKey, x25519, PrivateKey, ValidCryptoMaterialStringExt};
use aptos_genesis::{config::HostAndPort, keys::PrivateIdentity};
use aptos_rest_client::Transaction;
use aptos_types::account_address::AccountAddress;
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
    /// Private keys file, created from the `genesis keys` command
    #[clap(long)]
    pub(crate) private_keys_file: Option<PathBuf>,
    /// Hex encoded Consensus public key
    #[clap(long, parse(try_from_str = Ed25519PublicKey::from_encoded_string))]
    pub(crate) consensus_public_key: Option<Ed25519PublicKey>,
    /// Host and port pair for the validator e.g. 127.0.0.1:6180
    #[clap(long)]
    pub(crate) validator_host: HostAndPort,
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
    fn consensus_public_key(&self) -> CliTypedResult<Ed25519PublicKey> {
        if let Some(ref consensus_public_key) = self.consensus_public_key {
            Ok(consensus_public_key.clone())
        } else if let Some(ref file) = self.private_keys_file {
            let identity: PrivateIdentity =
                from_yaml(&String::from_utf8(read_from_file(file)?).map_err(CliError::from)?)?;
            Ok(identity.consensus_private_key.public_key())
        } else {
            Err(CliError::CommandArgumentError(
                "Must provide either --validator-identity-file or --consensus-public-key"
                    .to_string(),
            ))
        }
    }

    fn network_keys(&self) -> CliTypedResult<(x25519::PublicKey, Option<x25519::PublicKey>)> {
        let identity: Option<PrivateIdentity> = if let Some(ref file) = self.private_keys_file {
            from_yaml(&String::from_utf8(read_from_file(file)?).map_err(CliError::from)?)?
        } else {
            None
        };

        let validator_network_public_key =
            if let Some(public_key) = self.validator_network_public_key {
                Ok(public_key)
            } else if let Some(ref identity) = identity {
                Ok(identity.validator_network_private_key.public_key())
            } else {
                Err(CliError::CommandArgumentError(
                    "Must provide either --validator-identity-file or --consensus-public-key"
                        .to_string(),
                ))
            }?;

        let full_node_network_public_key =
            if let Some(public_key) = self.full_node_network_public_key {
                Some(public_key)
            } else {
                identity.map(|identity| identity.full_node_network_private_key.public_key())
            };

        Ok((validator_network_public_key, full_node_network_public_key))
    }
}

#[async_trait]
impl CliCommand<Transaction> for RegisterValidatorCandidate {
    fn command_name(&self) -> &'static str {
        "RegisterValidatorCandidate"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        let consensus_public_key = self.consensus_public_key()?;
        let (validator_network_public_key, full_node_network_public_key) = self.network_keys()?;
        let validator_network_addresses = vec![self
            .validator_host
            .as_network_address(validator_network_public_key)?];
        let full_node_network_addresses =
            match (self.full_node_host.as_ref(), full_node_network_public_key) {
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
        let address = if let Some(address) = self.operator_args.pool_address {
            address
        } else {
            self.txn_options.profile_options.account_address()?
        };

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
        let address = if let Some(address) = self.operator_args.pool_address {
            address
        } else {
            self.txn_options.profile_options.account_address()?
        };

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
