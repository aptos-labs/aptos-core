// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{account_address::AccountAddress, event::EventHandle};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct StakePool {
    pub active: u64,
    pub inactive: u64,
    pub pending_active: u64,
    pub pending_inactive: u64,
    pub locked_until_secs: u64,
    pub operator_address: AccountAddress,
    pub delegated_voter: AccountAddress,

    pub initialize_validator_events: EventHandle,
    pub set_operator_events: EventHandle,
    pub add_stake_events: EventHandle,
    pub reactivate_stake_events: EventHandle,
    pub rotate_consensus_key_events: EventHandle,
    pub update_network_and_fullnode_addresses_events: EventHandle,
    pub increase_lockup_events: EventHandle,
    pub join_validator_set_events: EventHandle,
    pub distribute_rewards_events: EventHandle,
    pub unlock_stake_events: EventHandle,
    pub withdraw_stake_events: EventHandle,
    pub leave_validator_set_events: EventHandle,
}
