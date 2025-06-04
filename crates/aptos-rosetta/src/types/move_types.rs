// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Types and identifiers for parsing Move pub structs and types

use crate::AccountAddress;
use aptos_types::event::EventHandle;
use serde::{Deserialize, Serialize};

pub const ACCOUNT_MODULE: &str = "account";
pub const APTOS_ACCOUNT_MODULE: &str = "aptos_account";
pub const APTOS_COIN_MODULE: &str = "aptos_coin";
pub const COIN_MODULE: &str = "coin";
pub const STAKE_MODULE: &str = "stake";
pub const STAKING_PROXY_MODULE: &str = "staking_proxy";
pub const STAKING_CONTRACT_MODULE: &str = "staking_contract";
pub const VESTING_MODULE: &str = "vesting";
pub const DELEGATION_POOL_MODULE: &str = "delegation_pool";
pub const OBJECT_MODULE: &str = "object";
pub const PRIMARY_FUNGIBLE_STORE_MODULE: &str = "primary_fungible_store";
pub const FUNGIBLE_ASSET_MODULE: &str = "fungible_asset";
pub const DISPATCHABLE_FUNGIBLE_ASSET_MODULE: &str = "dispatchable_fungible_asset";

pub const ACCOUNT_RESOURCE: &str = "Account";
pub const APTOS_COIN_RESOURCE: &str = "AptosCoin";
pub const COIN_INFO_RESOURCE: &str = "CoinInfo";
pub const COIN_STORE_RESOURCE: &str = "CoinStore";
pub const STAKE_POOL_RESOURCE: &str = "StakePool";
pub const STAKING_CONTRACT_RESOURCE: &str = "StakingContract";
pub const STORE_RESOURCE: &str = "Store";
pub const FUNGIBLE_STORE_RESOURCE: &str = "FungibleStore";
pub const STAKING_GROUP_UPDATE_COMMISSION_RESOURCE: &str = "StakingGroupUpdateCommissionEvent";
pub const VESTING_RESOURCE: &str = "Vesting";
pub const DELEGATION_POOL_RESOURCE: &str = "DelegationPool";
pub const WITHDRAW_STAKE_EVENT: &str = "WithdrawStakeEvent";
pub const WITHDRAW_STAKE: &str = "WithdrawStake";
pub const OBJECT_CORE_RESOURCE: &str = "ObjectCore";

pub const OBJECT_RESOURCE_GROUP: &str = "ObjectGroup";

pub const CREATE_ACCOUNT_FUNCTION: &str = "create_account";
pub const TRANSFER_FUNCTION: &str = "transfer";
pub const TRANSFER_COINS_FUNCTION: &str = "transfer_coins";
pub const BALANCE_FUNCTION: &str = "balance";

// Staking Contract
pub const RESET_LOCKUP_FUNCTION: &str = "reset_lockup";
pub const CREATE_STAKING_CONTRACT_FUNCTION: &str = "create_staking_contract";
pub const SWITCH_OPERATOR_WITH_SAME_COMMISSION_FUNCTION: &str =
    "switch_operator_with_same_commission";
pub const UPDATE_VOTER_FUNCTION: &str = "update_voter";
pub const UNLOCK_STAKE_FUNCTION: &str = "unlock_stake";
// TODO fix the typo in function name. commision -> commission (this has to be done on-chain first)
// TODO: Handle update_commission and update_commision
pub const UPDATE_COMMISSION_FUNCTION: &str = "update_commision";
pub const DISTRIBUTE_STAKING_REWARDS_FUNCTION: &str = "distribute";

// Delegation Pool Contract
pub const DELEGATION_POOL_ADD_STAKE_FUNCTION: &str = "add_stake";
pub const DELEGATION_POOL_UNLOCK_FUNCTION: &str = "unlock";
pub const DELEGATION_POOL_WITHDRAW_FUNCTION: &str = "withdraw";

pub const DECIMALS_FIELD: &str = "decimal";
pub const DEPOSIT_EVENTS_FIELD: &str = "deposit_events";
pub const WITHDRAW_EVENTS_FIELD: &str = "withdraw_events";
pub const SET_OPERATOR_EVENTS_FIELD: &str = "set_operator_events";
pub const SEQUENCE_NUMBER_FIELD: &str = "sequence_number";
pub const SYMBOL_FIELD: &str = "symbol";

// Staking Contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingContract {
    pub principal: u64,
    pub pool_address: AccountAddress,
    pub owner_cap: Capability,
    pub commission_percentage: u64,
    pub distribution_pool: Pool,
    pub signer_cap: Capability,
}

impl StakingContract {
    pub fn get_balance(&self, account_address: &AccountAddress) -> Option<u64> {
        self.distribution_pool.get_balance(account_address)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Store {
    pub staking_contracts: Vec<(AccountAddress, StakingContract)>,
    pub create_staking_contract_events: EventHandle,
    pub update_voter_events: EventHandle,
    pub reset_lockup_events: EventHandle,
    pub add_stake_events: EventHandle,
    pub request_commission_events: EventHandle,
    pub unlock_stake_events: EventHandle,
    pub switch_operator_events: EventHandle,
    pub add_distribution_events: EventHandle,
    pub distribute_events: EventHandle,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StakingGroupUpdateCommissionEvent {
    pub update_commission_events: EventHandle,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCommissionEvent {
    pub staker: AccountAddress,
    pub operator: AccountAddress,
    pub old_commission_percentage: u64,
    pub new_commission_percentage: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateStakingContractEvent {
    pub operator: AccountAddress,
    pub voter: AccountAddress,
    pub pool_address: AccountAddress,
    pub principal: u64,
    pub commission_percentage: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateVoterEvent {
    pub operator: AccountAddress,
    pub pool_address: AccountAddress,
    pub old_voter: AccountAddress,
    pub new_voter: AccountAddress,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResetLockupEvent {
    pub operator: AccountAddress,
    pub pool_address: AccountAddress,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddStakeEvent {
    pub operator: AccountAddress,
    pub pool_address: AccountAddress,
    pub amount: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestCommissionEvent {
    pub operator: AccountAddress,
    pub pool_address: AccountAddress,
    pub accumulated_rewards: u64,
    pub commission_amount: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnlockStakeEvent {
    pub operator: AccountAddress,
    pub pool_address: AccountAddress,
    pub amount: u64,
    pub commission_paid: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwitchOperatorEvent {
    pub old_operator: AccountAddress,
    pub new_operator: AccountAddress,
    pub pool_address: AccountAddress,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddDistributionEvent {
    pub operator: AccountAddress,
    pub pool_address: AccountAddress,
    pub amount: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DistributeEvent {
    pub operator: AccountAddress,
    pub pool_address: AccountAddress,
    pub recipient: AccountAddress,
    pub amount: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pool {
    pub shareholders_limit: u64,
    pub total_coins: u64,
    pub total_shares: u64,
    pub shares: Vec<(AccountAddress, u64)>,
    pub shareholders: Vec<AccountAddress>,
    pub scaling_factor: u64,
}

impl Pool {
    pub fn get_balance(&self, account_address: &AccountAddress) -> Option<u64> {
        self.shares
            .iter()
            .find(|(address, _)| address == account_address)
            .map(|(_, shares)| (*shares * self.total_coins) / self.total_shares)
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub pool_address: AccountAddress,
}

// Delegation Pool Contract
#[derive(Debug, Serialize, Deserialize)]
pub struct SharesPool {
    pub shareholders_limit: u64,
    pub total_coins: u64,
    pub total_shares: u64,
    pub shares: Vec<(AccountAddress, u64)>,
    pub shareholders: Vec<AccountAddress>,
    pub scaling_factor: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ObservedLockupCycle {
    pub index: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DelegationPool {
    pub active_shares: SharesPool,
    pub observed_lockup_cycle: ObservedLockupCycle,
    pub inactive_shares: Vec<(ObservedLockupCycle, SharesPool)>,
    pub pending_withdrawals: Vec<(AccountAddress, ObservedLockupCycle)>,
    pub stake_pool_signer_cap: Capability,
    pub total_coins_inactive: u64,
    pub operator_commission_percentage: u64,

    pub add_stake_events: EventHandle,
    pub reactivate_stake_events: EventHandle,
    pub unlock_stake_events: EventHandle,
    pub withdraw_stake_events: EventHandle,
    pub distribute_commission_events: EventHandle,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddDelegationEvent {
    pub pool_address: AccountAddress,
    pub delegator_address: AccountAddress,
    pub amount_added: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UndelegationEvent {
    pub pool_address: AccountAddress,
    pub delegator_address: AccountAddress,
    pub amount_unlocked: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WithdrawUndelegatedEvent {
    pub pool_address: AccountAddress,
    pub delegator_address: AccountAddress,
    pub amount_withdrawn: u64,
}
