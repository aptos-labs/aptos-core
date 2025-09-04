// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_address::AccountAddress,
    account_config::CORE_CODE_ADDRESS,
    event::{EventHandle, EventKey},
};
use anyhow::Result;
use velor_crypto::HashValue;
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    move_resource::{MoveResource, MoveStructType},
};
use serde::{Deserialize, Serialize};

/// Struct that represents a NewBlockEvent.
/// Should be kept in-sync with NewBlockEvent move struct in block.move.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewBlockEvent {
    pub hash: AccountAddress,
    pub epoch: u64,
    pub round: u64,
    pub height: u64,
    pub previous_block_votes_bitvec: Vec<u8>,
    pub proposer: AccountAddress,
    pub failed_proposer_indices: Vec<u64>,
    // usecs (microseconds)
    pub timestamp: u64,
}

impl NewBlockEvent {
    pub fn hash(&self) -> Result<HashValue> {
        Ok(HashValue::from_slice(self.hash.to_vec())?)
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn round(&self) -> u64 {
        self.round
    }

    pub fn height(&self) -> u64 {
        self.height
    }

    pub fn previous_block_votes_bitvec(&self) -> &Vec<u8> {
        &self.previous_block_votes_bitvec
    }

    pub fn proposer(&self) -> AccountAddress {
        self.proposer
    }

    /// The list of indices in the validators list,
    /// of consecutive proposers from the immediately preceeding
    /// rounds that didn't produce a successful block
    pub fn failed_proposer_indices(&self) -> &Vec<u64> {
        &self.failed_proposer_indices
    }

    pub fn proposed_time(&self) -> u64 {
        self.timestamp
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn new(
        hash: AccountAddress,
        epoch: u64,
        round: u64,
        height: u64,
        previous_block_votes_bitvec: Vec<u8>,
        proposer: AccountAddress,
        failed_proposer_indices: Vec<u64>,
        timestamp: u64,
    ) -> Self {
        Self {
            hash,
            epoch,
            round,
            height,
            previous_block_votes_bitvec,
            proposer,
            failed_proposer_indices,
            timestamp,
        }
    }
}

impl MoveStructType for NewBlockEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("block");
    const STRUCT_NAME: &'static IdentStr = ident_str!("NewBlockEvent");
}

pub fn new_block_event_key() -> EventKey {
    EventKey::new(3, CORE_CODE_ADDRESS)
}

/// Should be kept in-sync with BlockResource move struct in block.move.
#[derive(Deserialize, Serialize)]
pub struct BlockResource {
    height: u64,
    epoch_interval: u64,
    new_block_events: EventHandle,
    update_epoch_interval_events: EventHandle,
}

impl BlockResource {
    pub fn new_block_events(&self) -> &EventHandle {
        &self.new_block_events
    }

    pub fn height(&self) -> u64 {
        self.height
    }

    pub fn epoch_interval(&self) -> u64 {
        self.epoch_interval
    }
}

impl MoveStructType for BlockResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("block");
    const STRUCT_NAME: &'static IdentStr = ident_str!("BlockResource");
}

impl MoveResource for BlockResource {}
