// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{CliCommand, CliResult, CliTypedResult, TransactionOptions};
use aptos_rest_client::Transaction;
use aptos_transaction_builder::aptos_stdlib;
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use clap::Parser;

/// Tool for manipulating stake
///
#[derive(Parser)]
pub enum StakeTool {
    AddStake(AddStake),
    UnlockStake(UnlockStake),
    WithdrawStake(WithdrawStake),
    IncreaseLockup(IncreaseLockup),
    InitializeStakeOwner(InitializeStakeOwner),
    SetOperator(SetOperator),
    SetDelegatedVoter(SetDelegatedVoter),
}

impl StakeTool {
    pub async fn execute(self) -> CliResult {
        use StakeTool::*;
        match self {
            AddStake(tool) => tool.execute_serialized().await,
            UnlockStake(tool) => tool.execute_serialized().await,
            WithdrawStake(tool) => tool.execute_serialized().await,
            IncreaseLockup(tool) => tool.execute_serialized().await,
            InitializeStakeOwner(tool) => tool.execute_serialized().await,
            SetOperator(tool) => tool.execute_serialized().await,
            SetDelegatedVoter(tool) => tool.execute_serialized().await,
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
            .submit_transaction(aptos_stdlib::stake_add_stake(self.amount))
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
            .submit_transaction(aptos_stdlib::stake_unlock(self.amount))
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
    /// Amount of coins to withdraw
    #[clap(long)]
    pub amount: u64,
}

#[async_trait]
impl CliCommand<Transaction> for WithdrawStake {
    fn command_name(&self) -> &'static str {
        "WithdrawStake"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        self.node_op_options
            .submit_transaction(aptos_stdlib::stake_withdraw(self.amount))
            .await
    }
}

/// Increase lockup of all staked coins in an account
#[derive(Parser)]
pub struct IncreaseLockup {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<Transaction> for IncreaseLockup {
    fn command_name(&self) -> &'static str {
        "IncreaseLockup"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        self.txn_options
            .submit_transaction(aptos_stdlib::stake_increase_lockup())
            .await
    }
}

/// Register stake owner, to gain capability to delegate
/// operator or voting capability to a different account.
#[derive(Parser)]
pub struct InitializeStakeOwner {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    #[clap(long)]
    pub initial_stake_amount: u64,
    #[clap(long)]
    pub operator_address: Option<AccountAddress>,
    #[clap(long)]
    pub voter_address: Option<AccountAddress>,
}

#[async_trait]
impl CliCommand<Transaction> for InitializeStakeOwner {
    fn command_name(&self) -> &'static str {
        "InitializeStakeOwner"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        let owner_address = self.txn_options.sender_address()?;
        self.txn_options
            .submit_transaction(aptos_stdlib::stake_initialize_owner_only(
                self.initial_stake_amount,
                self.operator_address.unwrap_or(owner_address),
                self.voter_address.unwrap_or(owner_address),
            ))
            .await
    }
}

/// Delegate operator (running validator node) capability from the stake owner
/// to the given voter address
#[derive(Parser)]
pub struct SetOperator {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    #[clap(long)]
    pub operator_address: AccountAddress,
}

#[async_trait]
impl CliCommand<Transaction> for SetOperator {
    fn command_name(&self) -> &'static str {
        "SetOperator"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        self.txn_options
            .submit_transaction(aptos_stdlib::stake_set_operator(self.operator_address))
            .await
    }
}

/// Delegate voting capability from the stake owner to the given voter address
#[derive(Parser)]
pub struct SetDelegatedVoter {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    #[clap(long)]
    pub voter_address: AccountAddress,
}

#[async_trait]
impl CliCommand<Transaction> for SetDelegatedVoter {
    fn command_name(&self) -> &'static str {
        "SetDelegatedVoter"
    }

    async fn execute(mut self) -> CliTypedResult<Transaction> {
        self.txn_options
            .submit_transaction(aptos_stdlib::stake_set_delegated_voter(self.voter_address))
            .await
    }
}
