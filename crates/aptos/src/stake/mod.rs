// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{
    CliCommand, CliResult, CliTypedResult, TransactionOptions, TransactionSummary,
};
use crate::common::utils::prompt_yes_with_override;
use aptos_types::account_address::{default_stake_pool_address, AccountAddress};
use async_trait::async_trait;
use cached_packages::aptos_stdlib;
use clap::Parser;

/// Tool for manipulating stake
///
#[derive(Parser)]
pub enum StakeTool {
    AddStake(AddStake),
    CreateStakingContract(CreateStakingContract),
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
            CreateStakingContract(tool) => tool.execute_serialized().await,
            UnlockStake(tool) => tool.execute_serialized().await,
            WithdrawStake(tool) => tool.execute_serialized().await,
            IncreaseLockup(tool) => tool.execute_serialized().await,
            InitializeStakeOwner(tool) => tool.execute_serialized().await,
            SetOperator(tool) => tool.execute_serialized().await,
            SetDelegatedVoter(tool) => tool.execute_serialized().await,
        }
    }
}

/// Stake APT coins to the stake pool
///
/// This command allows stake pool owners to add APT coins to their stake.
#[derive(Parser)]
pub struct AddStake {
    /// Amount of Octas (10^-8 APT) to add to stake
    #[clap(long)]
    pub amount: u64,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for AddStake {
    fn command_name(&self) -> &'static str {
        "AddStake"
    }

    async fn execute(mut self) -> CliTypedResult<TransactionSummary> {
        self.txn_options
            .submit_transaction(aptos_stdlib::stake_add_stake(self.amount))
            .await
            .map(|inner| inner.into())
    }
}

/// Unlock staked APT coins
///
/// APT coins can only be unlocked if they no longer have an applied lockup period
#[derive(Parser)]
pub struct UnlockStake {
    /// Amount of Octas (10^-8 APT) to unlock
    #[clap(long)]
    pub amount: u64,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for UnlockStake {
    fn command_name(&self) -> &'static str {
        "UnlockStake"
    }

    async fn execute(mut self) -> CliTypedResult<TransactionSummary> {
        self.txn_options
            .submit_transaction(aptos_stdlib::stake_unlock(self.amount))
            .await
            .map(|inner| inner.into())
    }
}

/// Withdraw unlocked staked APT coins
///
/// This allows users to withdraw stake back into their CoinStore.
/// Before calling `WithdrawStake`, `UnlockStake` must be called first.
#[derive(Parser)]
pub struct WithdrawStake {
    /// Amount of Octas (10^-8 APT) to withdraw
    #[clap(long)]
    pub amount: u64,

    #[clap(flatten)]
    pub(crate) node_op_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for WithdrawStake {
    fn command_name(&self) -> &'static str {
        "WithdrawStake"
    }

    async fn execute(mut self) -> CliTypedResult<TransactionSummary> {
        self.node_op_options
            .submit_transaction(aptos_stdlib::stake_withdraw(self.amount))
            .await
            .map(|inner| inner.into())
    }
}

/// Increase lockup of all staked APT coins in the stake pool
///
/// Lockup may need to be increased in order to vote on a proposal.
#[derive(Parser)]
pub struct IncreaseLockup {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for IncreaseLockup {
    fn command_name(&self) -> &'static str {
        "IncreaseLockup"
    }

    async fn execute(mut self) -> CliTypedResult<TransactionSummary> {
        self.txn_options
            .submit_transaction(aptos_stdlib::stake_increase_lockup())
            .await
            .map(|inner| inner.into())
    }
}

/// Initialize stake owner
///
/// Initializing stake owner adds the capability to delegate the
/// stake pool to an operator, or delegate voting to a different account.
#[derive(Parser)]
pub struct InitializeStakeOwner {
    /// Initial amount of Octas (10^-8 APT) to be staked
    #[clap(long)]
    pub initial_stake_amount: u64,

    /// Account Address of delegated operator
    ///
    /// If not specified, it will be the same as the owner
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    pub operator_address: Option<AccountAddress>,

    /// Account address of delegated voter
    ///
    /// If not specified, it will be the same as the owner
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    pub voter_address: Option<AccountAddress>,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for InitializeStakeOwner {
    fn command_name(&self) -> &'static str {
        "InitializeStakeOwner"
    }

    async fn execute(mut self) -> CliTypedResult<TransactionSummary> {
        let owner_address = self.txn_options.sender_address()?;
        self.txn_options
            .submit_transaction(aptos_stdlib::stake_initialize_stake_owner(
                self.initial_stake_amount,
                self.operator_address.unwrap_or(owner_address),
                self.voter_address.unwrap_or(owner_address),
            ))
            .await
            .map(|inner| inner.into())
    }
}

/// Delegate operator capability from the stake owner to another account
#[derive(Parser)]
pub struct SetOperator {
    /// Account Address of delegated operator
    ///
    /// If not specified, it will be the same as the owner
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    pub operator_address: AccountAddress,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for SetOperator {
    fn command_name(&self) -> &'static str {
        "SetOperator"
    }

    async fn execute(mut self) -> CliTypedResult<TransactionSummary> {
        self.txn_options
            .submit_transaction(aptos_stdlib::stake_set_operator(self.operator_address))
            .await
            .map(|inner| inner.into())
    }
}

/// Delegate voting capability from the stake owner to another account
#[derive(Parser)]
pub struct SetDelegatedVoter {
    /// Account Address of delegated voter
    ///
    /// If not specified, it will be the same as the owner
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    pub voter_address: AccountAddress,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for SetDelegatedVoter {
    fn command_name(&self) -> &'static str {
        "SetDelegatedVoter"
    }

    async fn execute(mut self) -> CliTypedResult<TransactionSummary> {
        self.txn_options
            .submit_transaction(aptos_stdlib::stake_set_delegated_voter(self.voter_address))
            .await
            .map(|inner| inner.into())
    }
}

#[derive(Parser)]
pub struct CreateStakingContract {
    /// Account Address of operator
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    pub operator: AccountAddress,

    /// Account Address of delegated voter
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    pub voter: AccountAddress,

    /// Amount to create the staking contract with
    #[clap(long)]
    pub amount: u64,

    /// Percentage of accumulated rewards to pay the operator as commission
    #[clap(long)]
    pub commission_percentage: u64,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for CreateStakingContract {
    fn command_name(&self) -> &'static str {
        "CreateStakingContract"
    }

    async fn execute(mut self) -> CliTypedResult<TransactionSummary> {
        let pool_address = default_stake_pool_address(
            self.txn_options.profile_options.account_address()?,
            self.operator,
        );
        prompt_yes_with_override(
            &format!(
                "Creating a new staking contract with pool address 0x{}. Confirm?",
                pool_address
            ),
            self.txn_options.prompt_options,
        )?;

        self.txn_options
            .submit_transaction(aptos_stdlib::staking_contract_create_staking_contract(
                self.operator,
                self.voter,
                self.amount,
                self.commission_percentage,
                vec![],
            ))
            .await
            .map(|inner| inner.into())
    }
}
