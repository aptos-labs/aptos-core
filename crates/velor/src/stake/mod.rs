// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        types::{
            CliCommand, CliError, CliResult, CliTypedResult, TransactionOptions, TransactionSummary,
        },
        utils::prompt_yes_with_override,
    },
    node::{get_stake_pools, StakePoolType},
};
use velor_cached_packages::velor_stdlib;
use velor_types::{
    account_address::{
        create_vesting_contract_address, default_stake_pool_address, AccountAddress,
    },
    vesting::VestingAdminStore,
};
use async_trait::async_trait;
use clap::Parser;

/// Tool for manipulating stake and stake pools
///
#[derive(Parser)]
pub enum StakeTool {
    AddStake(AddStake),
    CreateStakingContract(CreateStakingContract),
    DistributeVestedCoins(DistributeVestedCoins),
    IncreaseLockup(IncreaseLockup),
    InitializeStakeOwner(InitializeStakeOwner),
    RequestCommission(RequestCommission),
    SetDelegatedVoter(SetDelegatedVoter),
    SetOperator(SetOperator),
    UnlockStake(UnlockStake),
    UnlockVestedCoins(UnlockVestedCoins),
    WithdrawStake(WithdrawStake),
}

impl StakeTool {
    pub async fn execute(self) -> CliResult {
        use StakeTool::*;
        match self {
            AddStake(tool) => tool.execute_serialized().await,
            CreateStakingContract(tool) => tool.execute_serialized().await,
            DistributeVestedCoins(tool) => tool.execute_serialized().await,
            IncreaseLockup(tool) => tool.execute_serialized().await,
            InitializeStakeOwner(tool) => tool.execute_serialized().await,
            RequestCommission(tool) => tool.execute_serialized().await,
            SetDelegatedVoter(tool) => tool.execute_serialized().await,
            SetOperator(tool) => tool.execute_serialized().await,
            UnlockStake(tool) => tool.execute_serialized().await,
            UnlockVestedCoins(tool) => tool.execute_serialized().await,
            WithdrawStake(tool) => tool.execute_serialized().await,
        }
    }
}

/// Add APT to a stake pool
///
/// This command allows stake pool owners to add APT to their stake.
#[derive(Parser)]
pub struct AddStake {
    /// Amount of Octas (10^-8 APT) to add to stake
    #[clap(long)]
    pub amount: u64,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<Vec<TransactionSummary>> for AddStake {
    fn command_name(&self) -> &'static str {
        "AddStake"
    }

    async fn execute(mut self) -> CliTypedResult<Vec<TransactionSummary>> {
        let client = self
            .txn_options
            .rest_options
            .client(&self.txn_options.profile_options)?;
        let amount = self.amount;
        let owner_address = self.txn_options.sender_address()?;
        let mut transaction_summaries: Vec<TransactionSummary> = vec![];

        let stake_pool_results = get_stake_pools(&client, owner_address).await?;
        for stake_pool in stake_pool_results {
            match stake_pool.pool_type {
                StakePoolType::Direct => {
                    transaction_summaries.push(
                        self.txn_options
                            .submit_transaction(velor_stdlib::stake_add_stake(amount))
                            .await
                            .map(|inner| inner.into())?,
                    );
                },
                StakePoolType::StakingContract => {
                    transaction_summaries.push(
                        self.txn_options
                            .submit_transaction(velor_stdlib::staking_contract_add_stake(
                                stake_pool.operator_address,
                                amount,
                            ))
                            .await
                            .map(|inner| inner.into())?,
                    );
                },
                StakePoolType::Vesting => {
                    return Err(CliError::UnexpectedError(
                        "Adding stake is not supported for vesting contracts".into(),
                    ))
                },
            }
        }
        Ok(transaction_summaries)
    }
}

/// Unlock staked APT in a stake pool
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
impl CliCommand<Vec<TransactionSummary>> for UnlockStake {
    fn command_name(&self) -> &'static str {
        "UnlockStake"
    }

    async fn execute(mut self) -> CliTypedResult<Vec<TransactionSummary>> {
        let client = self
            .txn_options
            .rest_options
            .client(&self.txn_options.profile_options)?;
        let amount = self.amount;
        let owner_address = self.txn_options.sender_address()?;
        let mut transaction_summaries: Vec<TransactionSummary> = vec![];

        let stake_pool_results = get_stake_pools(&client, owner_address).await?;
        for stake_pool in stake_pool_results {
            match stake_pool.pool_type {
                StakePoolType::Direct => {
                    transaction_summaries.push(
                        self.txn_options
                            .submit_transaction(velor_stdlib::stake_unlock(amount))
                            .await
                            .map(|inner| inner.into())?,
                    );
                },
                StakePoolType::StakingContract => {
                    transaction_summaries.push(
                        self.txn_options
                            .submit_transaction(velor_stdlib::staking_contract_unlock_stake(
                                stake_pool.operator_address,
                                amount,
                            ))
                            .await
                            .map(|inner| inner.into())?,
                    );
                },
                StakePoolType::Vesting => {
                    return Err(CliError::UnexpectedError(
                        "Unlocking stake is not supported for vesting contracts".into(),
                    ))
                },
            }
        }
        Ok(transaction_summaries)
    }
}

/// Withdraw unlocked staked APT from a stake pool
///
/// This allows users to withdraw stake back into their CoinStore.
/// Before calling `WithdrawStake`, `UnlockStake` must be called first.
#[derive(Parser)]
pub struct WithdrawStake {
    /// Amount of Octas (10^-8 APT) to withdraw.
    /// This only applies to stake pools owned directly by the owner account, instead of via
    /// a staking contract. In the latter case, when withdrawal is issued, all coins are distributed
    #[clap(long)]
    pub amount: u64,

    #[clap(flatten)]
    pub(crate) node_op_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<Vec<TransactionSummary>> for WithdrawStake {
    fn command_name(&self) -> &'static str {
        "WithdrawStake"
    }

    async fn execute(mut self) -> CliTypedResult<Vec<TransactionSummary>> {
        let client = self
            .node_op_options
            .rest_options
            .client(&self.node_op_options.profile_options)?;
        let amount = self.amount;
        let owner_address = self.node_op_options.sender_address()?;
        let mut transaction_summaries: Vec<TransactionSummary> = vec![];

        let stake_pool_results = get_stake_pools(&client, owner_address).await?;
        for stake_pool in stake_pool_results {
            match stake_pool.pool_type {
                StakePoolType::Direct => {
                    transaction_summaries.push(
                        self.node_op_options
                            .submit_transaction(velor_stdlib::stake_withdraw(amount))
                            .await
                            .map(|inner| inner.into())?,
                    );
                },
                StakePoolType::StakingContract => {
                    transaction_summaries.push(
                        self.node_op_options
                            .submit_transaction(velor_stdlib::staking_contract_distribute(
                                owner_address,
                                stake_pool.operator_address,
                            ))
                            .await
                            .map(|inner| inner.into())?,
                    );
                },
                StakePoolType::Vesting => {
                    return Err(CliError::UnexpectedError(
                        "Stake withdrawal from vesting contract should use distribute-vested-coins"
                            .into(),
                    ))
                },
            }
        }
        Ok(transaction_summaries)
    }
}

/// Increase lockup of all staked APT in a stake pool
///
/// Lockup may need to be increased in order to vote on a proposal.
#[derive(Parser)]
pub struct IncreaseLockup {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<Vec<TransactionSummary>> for IncreaseLockup {
    fn command_name(&self) -> &'static str {
        "IncreaseLockup"
    }

    async fn execute(mut self) -> CliTypedResult<Vec<TransactionSummary>> {
        let client = self
            .txn_options
            .rest_options
            .client(&self.txn_options.profile_options)?;
        let owner_address = self.txn_options.sender_address()?;
        let mut transaction_summaries: Vec<TransactionSummary> = vec![];

        let stake_pool_results = get_stake_pools(&client, owner_address).await?;
        for stake_pool in stake_pool_results {
            match stake_pool.pool_type {
                StakePoolType::Direct => {
                    transaction_summaries.push(
                        self.txn_options
                            .submit_transaction(velor_stdlib::stake_increase_lockup())
                            .await
                            .map(|inner| inner.into())?,
                    );
                },
                StakePoolType::StakingContract => {
                    transaction_summaries.push(
                        self.txn_options
                            .submit_transaction(velor_stdlib::staking_contract_reset_lockup(
                                stake_pool.operator_address,
                            ))
                            .await
                            .map(|inner| inner.into())?,
                    );
                },
                StakePoolType::Vesting => {
                    transaction_summaries.push(
                        self.txn_options
                            .submit_transaction(velor_stdlib::vesting_reset_lockup(
                                stake_pool.vesting_contract.unwrap(),
                            ))
                            .await
                            .map(|inner| inner.into())?,
                    );
                },
            }
        }
        Ok(transaction_summaries)
    }
}

/// Initialize a stake pool owner
///
/// Initializing stake owner adds the capability to delegate the
/// stake pool to an operator, or delegate voting to a different account.
#[derive(Parser)]
pub struct InitializeStakeOwner {
    /// Initial amount of Octas (10^-8 APT) to be staked
    #[clap(long)]
    pub initial_stake_amount: u64,

    /// Account Address of delegated operator
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub operator_address: Option<AccountAddress>,

    /// Account address of delegated voter
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
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
            .submit_transaction(velor_stdlib::stake_initialize_stake_owner(
                self.initial_stake_amount,
                self.operator_address.unwrap_or(owner_address),
                self.voter_address.unwrap_or(owner_address),
            ))
            .await
            .map(|inner| inner.into())
    }
}

/// Delegate operator capability to another account
///
/// This changes the operator capability from its current operator to a different operator.
/// By default, the operator of a stake pool is the owner of the stake pool
#[derive(Parser)]
pub struct SetOperator {
    /// Account Address of delegated operator
    ///
    /// If not specified, it will be the same as the owner
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub operator_address: AccountAddress,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<Vec<TransactionSummary>> for SetOperator {
    fn command_name(&self) -> &'static str {
        "SetOperator"
    }

    async fn execute(mut self) -> CliTypedResult<Vec<TransactionSummary>> {
        let client = self
            .txn_options
            .rest_options
            .client(&self.txn_options.profile_options)?;
        let owner_address = self.txn_options.sender_address()?;
        let new_operator_address = self.operator_address;
        let mut transaction_summaries: Vec<TransactionSummary> = vec![];

        let stake_pool_results = get_stake_pools(&client, owner_address).await?;
        for stake_pool in stake_pool_results {
            match stake_pool.pool_type {
                StakePoolType::Direct => {
                    transaction_summaries.push(
                        self.txn_options
                            .submit_transaction(velor_stdlib::stake_set_operator(
                                new_operator_address,
                            ))
                            .await
                            .map(|inner| inner.into())?,
                    );
                },
                StakePoolType::StakingContract => {
                    transaction_summaries.push(
                        self.txn_options
                            .submit_transaction(
                                velor_stdlib::staking_contract_switch_operator_with_same_commission(
                                    stake_pool.operator_address,
                                    new_operator_address,
                                ),
                            )
                            .await
                            .map(|inner| inner.into())?,
                    );
                },
                StakePoolType::Vesting => {
                    transaction_summaries.push(
                        self.txn_options
                            .submit_transaction(
                                velor_stdlib::vesting_update_operator_with_same_commission(
                                    stake_pool.vesting_contract.unwrap(),
                                    new_operator_address,
                                ),
                            )
                            .await
                            .map(|inner| inner.into())?,
                    );
                },
            }
        }
        Ok(transaction_summaries)
    }
}

/// Delegate voting capability to another account
///
/// Delegates voting capability from its current voter to a different voter.
/// By default, the voter of a stake pool is the owner of the stake pool
#[derive(Parser)]
pub struct SetDelegatedVoter {
    /// Account Address of delegated voter
    ///
    /// If not specified, it will be the same as the owner
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub voter_address: AccountAddress,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<Vec<TransactionSummary>> for SetDelegatedVoter {
    fn command_name(&self) -> &'static str {
        "SetDelegatedVoter"
    }

    async fn execute(mut self) -> CliTypedResult<Vec<TransactionSummary>> {
        let client = self
            .txn_options
            .rest_options
            .client(&self.txn_options.profile_options)?;
        let owner_address = self.txn_options.sender_address()?;
        let new_voter_address = self.voter_address;
        let mut transaction_summaries: Vec<TransactionSummary> = vec![];

        let stake_pool_results = get_stake_pools(&client, owner_address).await?;
        for stake_pool in stake_pool_results {
            match stake_pool.pool_type {
                StakePoolType::Direct => {
                    transaction_summaries.push(
                        self.txn_options
                            .submit_transaction(velor_stdlib::stake_set_delegated_voter(
                                new_voter_address,
                            ))
                            .await
                            .map(|inner| inner.into())?,
                    );
                },
                StakePoolType::StakingContract => {
                    transaction_summaries.push(
                        self.txn_options
                            .submit_transaction(velor_stdlib::staking_contract_update_voter(
                                stake_pool.operator_address,
                                new_voter_address,
                            ))
                            .await
                            .map(|inner| inner.into())?,
                    );
                },
                StakePoolType::Vesting => {
                    transaction_summaries.push(
                        self.txn_options
                            .submit_transaction(velor_stdlib::vesting_update_voter(
                                stake_pool.vesting_contract.unwrap(),
                                new_voter_address,
                            ))
                            .await
                            .map(|inner| inner.into())?,
                    );
                },
            }
        }
        Ok(transaction_summaries)
    }
}

/// Create a staking contract stake pool
///
///
#[derive(Parser)]
pub struct CreateStakingContract {
    /// Account Address of operator
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub operator: AccountAddress,

    /// Account Address of delegated voter
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
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
            .submit_transaction(velor_stdlib::staking_contract_create_staking_contract(
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

/// Distribute fully unlocked coins from vesting
///
/// Distribute fully unlocked coins (rewards and/or vested coins) from the vesting contract
/// to shareholders.
#[derive(Parser)]
pub struct DistributeVestedCoins {
    /// Address of the vesting contract's admin.
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub admin_address: AccountAddress,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for DistributeVestedCoins {
    fn command_name(&self) -> &'static str {
        "DistributeVestedCoins"
    }

    async fn execute(mut self) -> CliTypedResult<TransactionSummary> {
        let vesting_contract_address = create_vesting_contract_address(self.admin_address, 0, &[]);
        self.txn_options
            .submit_transaction(velor_stdlib::vesting_distribute(vesting_contract_address))
            .await
            .map(|inner| inner.into())
    }
}

/// Unlock vested coins
///
/// Unlock vested coins according to the vesting contract's schedule.
/// This also unlocks any accumulated staking rewards and pays commission to the operator of the
/// vesting contract's stake pool first.
///
/// The unlocked vested tokens and staking rewards are still subject to the staking lockup and
/// cannot be withdrawn until after the lockup expires.
#[derive(Parser)]
pub struct UnlockVestedCoins {
    /// Address of the vesting contract's admin.
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub admin_address: AccountAddress,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for UnlockVestedCoins {
    fn command_name(&self) -> &'static str {
        "UnlockVestedCoins"
    }

    async fn execute(mut self) -> CliTypedResult<TransactionSummary> {
        let vesting_contract_address = create_vesting_contract_address(self.admin_address, 0, &[]);
        self.txn_options
            .submit_transaction(velor_stdlib::vesting_vest(vesting_contract_address))
            .await
            .map(|inner| inner.into())
    }
}

/// Request commission from running a stake pool
///
/// Allows operators or owners to request commission from running a stake pool (only if there's a
/// staking contract set up with the staker).  The commission will be withdrawable at the end of the
/// stake pool's current lockup period.
#[derive(Parser)]
pub struct RequestCommission {
    /// Address of the owner of the stake pool
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub owner_address: AccountAddress,

    /// Address of the operator of the stake pool
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub operator_address: AccountAddress,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for RequestCommission {
    fn command_name(&self) -> &'static str {
        "RequestCommission"
    }

    async fn execute(mut self) -> CliTypedResult<TransactionSummary> {
        let client = self
            .txn_options
            .rest_options
            .client(&self.txn_options.profile_options)?;

        // If this is a vesting stake pool, retrieve the associated vesting contract
        let vesting_admin_store = client
            .get_account_resource_bcs::<VestingAdminStore>(
                self.owner_address,
                "0x1::vesting::AdminStore",
            )
            .await;

        // Note: this only works if the vesting contract has exactly one staking contract
        // associated
        let staker_address = if let Ok(vesting_admin_store) = vesting_admin_store {
            vesting_admin_store.into_inner().vesting_contracts[0]
        } else {
            self.owner_address
        };
        self.txn_options
            .submit_transaction(velor_stdlib::staking_contract_request_commission(
                staker_address,
                self.operator_address,
            ))
            .await
            .map(|inner| inner.into())
    }
}
