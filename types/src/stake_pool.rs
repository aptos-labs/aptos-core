// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{account_address::AccountAddress, event::EventHandle};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct StakePool {
    pub active: u64,
    pub inactive: u64,
    pub pending_active: u64,
    pub pending_inactive: u64,
    pub locked_until_secs: u64,
    pub operator_address: AccountAddress,
    pub delegated_voter: AccountAddress,

    initialize_validator_events: EventHandle,
    set_operator_events: EventHandle,
    add_stake_events: EventHandle,
    reactivate_stake_events: EventHandle,
    rotate_consensus_key_events: EventHandle,
    update_network_and_fullnode_addresses_events: EventHandle,
    increase_lockup_events: EventHandle,
    join_validator_set_events: EventHandle,
    distribute_rewards_events: EventHandle,
    unlock_stake_events: EventHandle,
    withdraw_stake_events: EventHandle,
    leave_validator_set_events: EventHandle,
}
