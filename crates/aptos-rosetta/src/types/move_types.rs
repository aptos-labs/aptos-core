// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Types and identifiers for parsing Move structs and types

use crate::AccountAddress;
use aptos_types::event::EventHandle;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const ACCOUNT_MODULE: &str = "account";
pub const APTOS_ACCOUNT_MODULE: &str = "aptos_account";
pub const APTOS_COIN_MODULE: &str = "aptos_coin";
pub const COIN_MODULE: &str = "coin";
pub const STAKE_MODULE: &str = "stake";
pub const STAKING_PROXY_MODULE: &str = "staking_proxy";
pub const STAKING_CONTRACT_MODULE: &str = "staking_contract";

pub const ACCOUNT_RESOURCE: &str = "Account";
pub const APTOS_COIN_RESOURCE: &str = "AptosCoin";
pub const COIN_INFO_RESOURCE: &str = "CoinInfo";
pub const COIN_STORE_RESOURCE: &str = "CoinStore";
pub const STAKE_POOL_RESOURCE: &str = "StakePool";

pub const CREATE_ACCOUNT_FUNCTION: &str = "create_account";
pub const TRANSFER_FUNCTION: &str = "transfer";
pub const SET_OPERATOR_FUNCTION: &str = "set_operator";
pub const SET_VOTER_FUNCTION: &str = "set_voter";
pub const SET_DELEGATED_VOTER_FUNCTION: &str = "set_delegated_voter";

pub const DECIMALS_FIELD: &str = "decimal";
pub const DEPOSIT_EVENTS_FIELD: &str = "deposit_events";
pub const WITHDRAW_EVENTS_FIELD: &str = "withdraw_events";
pub const SET_OPERATOR_EVENTS_FIELD: &str = "set_operator_events";
pub const SEQUENCE_NUMBER_FIELD: &str = "sequence_number";
pub const SYMBOL_FIELD: &str = "symbol";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StakingContractStore {
    pub staking_contracts: BTreeMap<AccountAddress, StakingContract>,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StakingContract {
    pub principal: u64,
    pub pool_address: AccountAddress,
    pub owner_cap: Capability,
    pub commission_percentage: u64,
    pub distribution_pool: Pool,
    pub signer_cap: Capability,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Capability {
    pub account: AccountAddress,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Pool {
    shareholders_limit: u64,
    total_coins: u64,
    total_shares: u64,
    shares: BTreeMap<AccountAddress, u64>,
    shareholders: Vec<AccountAddress>,
    // Default to 1. This can be used to minimize rounding errors when computing shares and coins amount.
    // However, users need to make sure the coins amount don't overflow when multiplied by the scaling factor.
    scaling_factor: u64,
}
