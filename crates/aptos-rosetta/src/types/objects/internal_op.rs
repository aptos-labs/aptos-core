// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::*;
use crate::{
    common::native_coin,
    error::ApiResult,
    types::{move_types::*, OperationType},
    ApiError, RosettaContext,
};
use aptos_cached_packages::aptos_stdlib;
use aptos_rest_client::aptos_api_types::U64;
use aptos_types::{
    account_address::AccountAddress,
    transaction::{EntryFunction, TransactionPayload},
};
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, StructTag, TypeTag},
    parser::parse_type_tag,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};

/// A holder for all information related to a specific transaction
/// built from [`Operation`]s
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum InternalOperation {
    CreateAccount(CreateAccount),
    Transfer(Transfer),
    SetOperator(SetOperator),
    SetVoter(SetVoter),
    InitializeStakePool(InitializeStakePool),
    ResetLockup(ResetLockup),
    UnlockStake(UnlockStake),
    UpdateCommission(UpdateCommission),
    WithdrawUndelegated(WithdrawUndelegated),
    DistributeStakingRewards(DistributeStakingRewards),
    AddDelegatedStake(AddDelegatedStake),
    UnlockDelegatedStake(UnlockDelegatedStake),
}

impl InternalOperation {
    /// Pulls the [`InternalOperation`] from the set of [`Operation`]
    /// TODO: this needs to be broken up
    pub fn extract(
        server_context: &RosettaContext,
        operations: &Vec<Operation>,
    ) -> ApiResult<InternalOperation> {
        match operations.len() {
            // Single operation actions
            1 => {
                if let Some(operation) = operations.first() {
                    match OperationType::from_str(&operation.operation_type) {
                        Ok(OperationType::InitializeStakePool) => {
                            if let (
                                Some(OperationMetadata {
                                    new_operator,
                                    new_voter,
                                    staked_balance,
                                    commission_percentage,
                                    ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                let owner_address = account.account_address()?;
                                let operator_address = if let Some(address) = new_operator {
                                    address.account_address()?
                                } else {
                                    owner_address
                                };
                                let voter_address = if let Some(address) = new_voter {
                                    address.account_address()?
                                } else {
                                    owner_address
                                };

                                return Ok(Self::InitializeStakePool(InitializeStakePool {
                                    owner: owner_address,
                                    operator: operator_address,
                                    voter: voter_address,
                                    amount: staked_balance.map(u64::from).unwrap_or_default(),
                                    commission_percentage: commission_percentage
                                        .map(u64::from)
                                        .unwrap_or_default(),
                                    seed: vec![],
                                }));
                            }
                        },
                        Ok(OperationType::CreateAccount) => {
                            if let (
                                Some(OperationMetadata {
                                    sender: Some(sender),
                                    ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                return Ok(Self::CreateAccount(CreateAccount {
                                    sender: sender.account_address()?,
                                    new_account: account.account_address()?,
                                }));
                            }
                        },
                        Ok(OperationType::SetOperator) => {
                            if let (
                                Some(OperationMetadata {
                                    old_operator,
                                    new_operator: Some(new_operator),
                                    ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                let old_operator = if let Some(old_operator) = old_operator {
                                    Some(old_operator.account_address()?)
                                } else {
                                    None
                                };

                                return Ok(Self::SetOperator(SetOperator {
                                    owner: account.account_address()?,
                                    old_operator,
                                    new_operator: new_operator.account_address()?,
                                }));
                            }
                        },
                        Ok(OperationType::SetVoter) => {
                            if let (
                                Some(OperationMetadata {
                                    operator,
                                    new_voter: Some(new_voter),
                                    ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                let operator = if let Some(operator) = operator {
                                    Some(operator.account_address()?)
                                } else {
                                    None
                                };
                                return Ok(Self::SetVoter(SetVoter {
                                    owner: account.account_address()?,
                                    operator,
                                    new_voter: new_voter.account_address()?,
                                }));
                            }
                        },
                        Ok(OperationType::ResetLockup) => {
                            if let (Some(OperationMetadata { operator, .. }), Some(account)) =
                                (&operation.metadata, &operation.account)
                            {
                                let operator = if let Some(operator) = operator {
                                    operator.account_address()?
                                } else {
                                    return Err(ApiError::InvalidInput(Some(
                                        "Reset lockup missing operator field".to_string(),
                                    )));
                                };
                                return Ok(Self::ResetLockup(ResetLockup {
                                    owner: account.account_address()?,
                                    operator,
                                }));
                            }
                        },
                        Ok(OperationType::UnlockStake) => {
                            if let (
                                Some(OperationMetadata {
                                    operator, amount, ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                let operator = if let Some(operator) = operator {
                                    operator.account_address()?
                                } else {
                                    return Err(ApiError::InvalidInput(Some(
                                        "Unlock Stake missing operator field".to_string(),
                                    )));
                                };
                                return Ok(Self::UnlockStake(UnlockStake {
                                    owner: account.account_address()?,
                                    operator,
                                    amount: amount.map(u64::from).unwrap_or_default(),
                                }));
                            }
                        },
                        Ok(OperationType::UpdateCommission) => {
                            if let (
                                Some(OperationMetadata {
                                    operator,
                                    commission_percentage,
                                    ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                let operator = if let Some(operator) = operator {
                                    operator.account_address()?
                                } else {
                                    return Err(ApiError::InvalidInput(Some(
                                        "Unlock Stake missing operator field".to_string(),
                                    )));
                                };
                                return Ok(Self::UpdateCommission(UpdateCommission {
                                    owner: account.account_address()?,
                                    operator,
                                    new_commission_percentage: commission_percentage
                                        .map(u64::from)
                                        .unwrap_or_default(),
                                }));
                            }
                        },
                        Ok(OperationType::DistributeStakingRewards) => {
                            if let (
                                Some(OperationMetadata {
                                    operator: Some(operator),
                                    staker: Some(staker),
                                    ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                return Ok(Self::DistributeStakingRewards(
                                    DistributeStakingRewards {
                                        sender: account.account_address()?,
                                        operator: operator.account_address()?,
                                        staker: staker.account_address()?,
                                    },
                                ));
                            }
                        },
                        Ok(OperationType::AddDelegatedStake) => {
                            if let (
                                Some(OperationMetadata {
                                    pool_address: Some(pool_address),
                                    amount,
                                    ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                return Ok(Self::AddDelegatedStake(AddDelegatedStake {
                                    delegator: account.account_address()?,
                                    pool_address: pool_address.account_address()?,
                                    amount: amount.map(u64::from).unwrap_or_default(),
                                }));
                            }
                        },
                        Ok(OperationType::UnlockDelegatedStake) => {
                            if let (
                                Some(OperationMetadata {
                                    pool_address: Some(pool_address),
                                    amount,
                                    ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                return Ok(Self::UnlockDelegatedStake(UnlockDelegatedStake {
                                    delegator: account.account_address()?,
                                    pool_address: pool_address.account_address()?,
                                    amount: amount.map(u64::from).unwrap_or_default(),
                                }));
                            }
                        },
                        Ok(OperationType::WithdrawUndelegatedFunds) => {
                            if let (
                                Some(OperationMetadata {
                                    pool_address: Some(pool_address),
                                    amount,
                                    ..
                                }),
                                Some(account),
                            ) = (&operation.metadata, &operation.account)
                            {
                                return Ok(Self::WithdrawUndelegated(WithdrawUndelegated {
                                    delegator: account.account_address()?,
                                    amount_withdrawn: amount.map(u64::from).unwrap_or_default(),
                                    pool_address: pool_address.account_address()?,
                                }));
                            }
                        },
                        _ => {},
                    }
                }

                // Return invalid operations if for any reason parsing fails
                Err(ApiError::InvalidOperations(Some(format!(
                    "Unrecognized single operation {:?}",
                    operations
                ))))
            },
            // Double operation actions (only coin transfer)
            2 => Ok(Self::Transfer(Transfer::extract_transfer(
                server_context,
                operations,
            )?)),
            // Anything else is not expected
            _ => Err(ApiError::InvalidOperations(Some(format!(
                "Unrecognized operation combination {:?}",
                operations
            )))),
        }
    }

    /// The sender of the transaction
    pub fn sender(&self) -> AccountAddress {
        match self {
            Self::CreateAccount(inner) => inner.sender,
            Self::Transfer(inner) => inner.sender,
            Self::SetOperator(inner) => inner.owner,
            Self::SetVoter(inner) => inner.owner,
            Self::InitializeStakePool(inner) => inner.owner,
            Self::ResetLockup(inner) => inner.owner,
            Self::UnlockStake(inner) => inner.owner,
            Self::UpdateCommission(inner) => inner.owner,
            Self::WithdrawUndelegated(inner) => inner.delegator,
            Self::DistributeStakingRewards(inner) => inner.sender,
            Self::AddDelegatedStake(inner) => inner.delegator,
            Self::UnlockDelegatedStake(inner) => inner.delegator,
        }
    }

    pub fn payload(
        &self,
    ) -> ApiResult<(aptos_types::transaction::TransactionPayload, AccountAddress)> {
        Ok(match self {
            InternalOperation::CreateAccount(create_account) => (
                aptos_stdlib::aptos_account_create_account(create_account.new_account),
                create_account.sender,
            ),
            InternalOperation::Transfer(transfer) => {
                // Check if the currency is known
                let currency = &transfer.currency;

                // We special case APT, because we don't want the behavior to change
                if currency == &native_coin() {
                    return Ok((
                        aptos_stdlib::aptos_account_transfer(transfer.receiver, transfer.amount.0),
                        transfer.sender,
                    ));
                }

                // For all other coins and FAs we need to handle them accordingly
                if let Some(ref metadata) = currency.metadata {
                    match (&metadata.move_type, &metadata.fa_address) {
                        // For currencies with the coin type, we will always use the coin functionality, even if migrated
                        (Some(coin_type), Some(_)) | (Some(coin_type), None) => {
                            let coin_type_tag = parse_type_tag(coin_type)
                                .map_err(|err| ApiError::InvalidInput(Some(err.to_string())))?;
                            (
                                aptos_stdlib::aptos_account_transfer_coins(
                                    coin_type_tag,
                                    transfer.receiver,
                                    transfer.amount.0,
                                ),
                                transfer.sender,
                            )
                        },
                        // For FA only currencies, we use the FA functionality
                        (None, Some(fa_address_str)) => {
                            let fa_address = AccountAddress::from_str(fa_address_str)?;

                            (
                                TransactionPayload::EntryFunction(EntryFunction::new(
                                    ModuleId::new(
                                        AccountAddress::ONE,
                                        ident_str!("primary_fungible_store").to_owned(),
                                    ),
                                    ident_str!("transfer").to_owned(),
                                    vec![TypeTag::Struct(Box::new(StructTag {
                                        address: AccountAddress::ONE,
                                        module: ident_str!(OBJECT_MODULE).into(),
                                        name: ident_str!(OBJECT_CORE_RESOURCE).into(),
                                        type_args: vec![],
                                    }))],
                                    vec![
                                        bcs::to_bytes(&fa_address).unwrap(),
                                        bcs::to_bytes(&transfer.receiver).unwrap(),
                                        bcs::to_bytes(&transfer.amount.0).unwrap(),
                                    ],
                                )),
                                transfer.sender,
                            )
                        },
                        _ => {
                            return Err(ApiError::InvalidInput(Some(format!(
                                "{} does not have a move type provided",
                                currency.symbol
                            ))))
                        },
                    }
                } else {
                    // This should never happen unless the server's currency list is improperly set
                    return Err(ApiError::InvalidInput(Some(format!(
                        "{} does not have a currency information provided",
                        currency.symbol
                    ))));
                }
            },
            InternalOperation::SetOperator(set_operator) => {
                if set_operator.old_operator.is_none() {
                    return Err(ApiError::InvalidInput(Some(
                        "SetOperator doesn't have an old operator".to_string(),
                    )));
                }
                (
                    aptos_stdlib::staking_contract_switch_operator_with_same_commission(
                        set_operator.old_operator.unwrap(),
                        set_operator.new_operator,
                    ),
                    set_operator.owner,
                )
            },
            InternalOperation::SetVoter(set_voter) => {
                if set_voter.operator.is_none() {
                    return Err(ApiError::InvalidInput(Some(
                        "Set voter doesn't have an operator".to_string(),
                    )));
                }
                (
                    aptos_stdlib::staking_contract_update_voter(
                        set_voter.operator.unwrap(),
                        set_voter.new_voter,
                    ),
                    set_voter.owner,
                )
            },
            InternalOperation::InitializeStakePool(init_stake_pool) => (
                aptos_stdlib::staking_contract_create_staking_contract(
                    init_stake_pool.operator,
                    init_stake_pool.voter,
                    init_stake_pool.amount,
                    init_stake_pool.commission_percentage,
                    init_stake_pool.seed.clone(),
                ),
                init_stake_pool.owner,
            ),
            InternalOperation::ResetLockup(reset_lockup) => (
                aptos_stdlib::staking_contract_reset_lockup(reset_lockup.operator),
                reset_lockup.owner,
            ),
            InternalOperation::UnlockStake(unlock_stake) => (
                aptos_stdlib::staking_contract_unlock_stake(
                    unlock_stake.operator,
                    unlock_stake.amount,
                ),
                unlock_stake.owner,
            ),
            InternalOperation::UpdateCommission(update_commision) => (
                aptos_stdlib::staking_contract_update_commision(
                    update_commision.operator,
                    update_commision.new_commission_percentage,
                ),
                update_commision.owner,
            ),
            InternalOperation::DistributeStakingRewards(distribute_staking_rewards) => (
                aptos_stdlib::staking_contract_distribute(
                    distribute_staking_rewards.staker,
                    distribute_staking_rewards.operator,
                ),
                distribute_staking_rewards.sender,
            ),
            InternalOperation::AddDelegatedStake(add_delegated_stake) => (
                aptos_stdlib::delegation_pool_add_stake(
                    add_delegated_stake.pool_address,
                    add_delegated_stake.amount,
                ),
                add_delegated_stake.delegator,
            ),
            InternalOperation::UnlockDelegatedStake(unlock_delegated_stake) => (
                aptos_stdlib::delegation_pool_unlock(
                    unlock_delegated_stake.pool_address,
                    unlock_delegated_stake.amount,
                ),
                unlock_delegated_stake.delegator,
            ),
            InternalOperation::WithdrawUndelegated(withdraw_undelegated) => (
                aptos_stdlib::delegation_pool_withdraw(
                    withdraw_undelegated.pool_address,
                    withdraw_undelegated.amount_withdrawn,
                ),
                withdraw_undelegated.delegator,
            ),
        })
    }
}

/// Operation to create an account
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CreateAccount {
    pub sender: AccountAddress,
    pub new_account: AccountAddress,
}

/// Operation to transfer coins between accounts
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Transfer {
    pub sender: AccountAddress,
    pub receiver: AccountAddress,
    pub amount: U64,
    pub currency: Currency,
}

impl Transfer {
    pub fn extract_transfer(
        server_context: &RosettaContext,
        operations: &Vec<Operation>,
    ) -> ApiResult<Transfer> {
        // Only support 1:1 P2P transfer
        // This is composed of a Deposit and a Withdraw operation
        if operations.len() != 2 {
            return Err(ApiError::InvalidTransferOperations(Some(
                "Must have exactly 2 operations a withdraw and a deposit",
            )));
        }

        let mut op_map = HashMap::new();
        for op in operations {
            let op_type = OperationType::from_str(&op.operation_type)?;
            op_map.insert(op_type, op);
        }

        if !op_map.contains_key(&OperationType::Deposit) {
            return Err(ApiError::InvalidTransferOperations(Some(
                "Must have a deposit",
            )));
        }

        // Verify accounts and amounts
        let (sender, withdraw_amount) = if let Some(withdraw) = op_map.get(&OperationType::Withdraw)
        {
            if let (Some(account), Some(amount)) = (&withdraw.account, &withdraw.amount) {
                if account.is_base_account() {
                    (account.account_address()?, amount)
                } else {
                    return Err(ApiError::InvalidInput(Some(
                        "Transferring stake amounts is not supported".to_string(),
                    )));
                }
            } else {
                return Err(ApiError::InvalidTransferOperations(Some(
                    "Invalid withdraw account provided",
                )));
            }
        } else {
            return Err(ApiError::InvalidTransferOperations(Some(
                "Must have a withdraw",
            )));
        };

        let (receiver, deposit_amount) = if let Some(deposit) = op_map.get(&OperationType::Deposit)
        {
            if let (Some(account), Some(amount)) = (&deposit.account, &deposit.amount) {
                if account.is_base_account() {
                    (account.account_address()?, amount)
                } else {
                    return Err(ApiError::InvalidInput(Some(
                        "Transferring stake amounts is not supported".to_string(),
                    )));
                }
            } else {
                return Err(ApiError::InvalidTransferOperations(Some(
                    "Invalid deposit account provided",
                )));
            }
        } else {
            return Err(ApiError::InvalidTransferOperations(Some(
                "Must have a deposit",
            )));
        };

        // Currencies have to be the same
        if withdraw_amount.currency != deposit_amount.currency {
            return Err(ApiError::InvalidTransferOperations(Some(
                "Currency mismatch between withdraw and deposit",
            )));
        }

        // Check that the currency is supported
        if !server_context
            .currencies
            .contains(&withdraw_amount.currency)
        {
            return Err(ApiError::UnsupportedCurrency(Some(
                withdraw_amount.currency.symbol.clone(),
            )));
        }

        let withdraw_value = i128::from_str(&withdraw_amount.value)
            .map_err(|_| ApiError::InvalidTransferOperations(Some("Withdraw amount is invalid")))?;
        let deposit_value = i128::from_str(&deposit_amount.value)
            .map_err(|_| ApiError::InvalidTransferOperations(Some("Deposit amount is invalid")))?;

        // We can't create or destroy coins, they must be negatives of each other
        if -withdraw_value != deposit_value {
            return Err(ApiError::InvalidTransferOperations(Some(
                "Withdraw amount must be equal to negative of deposit amount",
            )));
        }

        // We converted to u128 to ensure no loss of precision in comparison,
        // but now we actually have to check it's a u64
        if deposit_value > u64::MAX as i128 {
            return Err(ApiError::InvalidTransferOperations(Some(
                "Transfer amount must not be greater than u64 max",
            )));
        }

        let transfer_amount = deposit_value as u64;

        Ok(Transfer {
            sender,
            receiver,
            amount: transfer_amount.into(),
            currency: deposit_amount.currency.clone(),
        })
    }
}

/// Set operator
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SetOperator {
    pub owner: AccountAddress,
    pub old_operator: Option<AccountAddress>,
    pub new_operator: AccountAddress,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SetVoter {
    pub owner: AccountAddress,
    pub operator: Option<AccountAddress>,
    pub new_voter: AccountAddress,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InitializeStakePool {
    pub owner: AccountAddress,
    pub operator: AccountAddress,
    pub voter: AccountAddress,
    pub amount: u64,
    pub commission_percentage: u64,
    pub seed: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResetLockup {
    pub owner: AccountAddress,
    pub operator: AccountAddress,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UnlockStake {
    pub owner: AccountAddress,
    pub operator: AccountAddress,
    pub amount: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UpdateCommission {
    pub owner: AccountAddress,
    pub operator: AccountAddress,
    pub new_commission_percentage: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WithdrawUndelegated {
    pub delegator: AccountAddress,
    pub pool_address: AccountAddress,
    pub amount_withdrawn: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DistributeStakingRewards {
    pub sender: AccountAddress,
    pub operator: AccountAddress,
    pub staker: AccountAddress,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AddDelegatedStake {
    pub delegator: AccountAddress,
    pub pool_address: AccountAddress,
    pub amount: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UnlockDelegatedStake {
    pub delegator: AccountAddress,
    pub pool_address: AccountAddress,
    pub amount: u64,
}
