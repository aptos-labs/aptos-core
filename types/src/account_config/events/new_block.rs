// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::account_address::AccountAddress;
use anyhow::Result;
use move_deps::move_core_types::{ident_str, identifier::IdentStr, move_resource::MoveStructType};
use serde::{Deserialize, Serialize};

/// Struct that represents a NewBlockEvent.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewBlockEvent {
    epoch: u64,
    round: u64,
    previous_block_votes: Vec<bool>,
    proposer: AccountAddress,
    failed_proposer_indices: Vec<u64>,
    timestamp: u64,
}

impl NewBlockEvent {
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn round(&self) -> u64 {
        self.round
    }

    pub fn previous_block_votes(&self) -> &Vec<bool> {
        &self.previous_block_votes
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

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new(
        epoch: u64,
        round: u64,
        previous_block_votes: Vec<bool>,
        proposer: AccountAddress,
        failed_proposer_indices: Vec<u64>,
        timestamp: u64,
    ) -> Self {
        Self {
            epoch,
            round,
            previous_block_votes,
            proposer,
            failed_proposer_indices,
            timestamp,
        }
    }
}

impl MoveStructType for NewBlockEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("Block");
    const STRUCT_NAME: &'static IdentStr = ident_str!("NewBlockEvent");
}
