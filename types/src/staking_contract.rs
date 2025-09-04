// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{account_address::AccountAddress, event::EventHandle};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DistributionPoolShare {
    key: AccountAddress,
    value: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DistributionPool {
    shareholders_limit: u64,
    total_coins: u64,
    total_shares: u64,
    shares: Vec<DistributionPoolShare>,
    shareholders: Vec<AccountAddress>,
    scaling_factor: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StakingContract {
    pub principal: u64,
    pub pool_address: AccountAddress,
    owner_cap: AccountAddress,
    pub commission_percentage: u64,
    distribution_pool: DistributionPool,
    signer_cap: AccountAddress,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StakingContractEntry {
    pub key: AccountAddress,
    pub value: StakingContract,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StakingContractStore {
    pub staking_contracts: Vec<StakingContractEntry>,

    // Events.
    create_staking_contract_events: EventHandle,
    update_voter_events: EventHandle,
    reset_lockup_events: EventHandle,
    add_stake_events: EventHandle,
    request_commission_events: EventHandle,
    unlock_stake_events: EventHandle,
    switch_operator_events: EventHandle,
    add_distribution_events: EventHandle,
    distribute_events: EventHandle,
}
