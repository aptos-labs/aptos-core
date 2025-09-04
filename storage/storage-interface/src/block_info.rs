// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_crypto::hash::HashValue;
use velor_types::{
    account_address::AccountAddress, account_config::NewBlockEvent, transaction::Version,
};
use serde::{Deserialize, Serialize};
use std::ops::Deref;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub enum BlockInfo {
    V0(BlockInfoV0),
}

impl BlockInfo {
    pub fn from_new_block_event(version: Version, new_block_event: &NewBlockEvent) -> Self {
        let NewBlockEvent {
            hash,
            epoch,
            round,
            height: _,
            previous_block_votes_bitvec: _,
            proposer,
            failed_proposer_indices: _,
            timestamp,
        } = new_block_event;

        Self::V0(BlockInfoV0 {
            id: HashValue::from_slice(hash.as_slice()).unwrap(),
            epoch: *epoch,
            round: *round,
            proposer: *proposer,
            first_version: version,
            timestamp_usecs: *timestamp,
        })
    }
}

impl Deref for BlockInfo {
    type Target = BlockInfoV0;

    fn deref(&self) -> &Self::Target {
        match self {
            BlockInfo::V0(v0) => v0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub struct BlockInfoV0 {
    /// Block hash.
    id: HashValue,
    epoch: u64,
    round: u64,
    proposer: AccountAddress,
    first_version: Version,
    timestamp_usecs: u64,
}

impl BlockInfoV0 {
    pub fn id(&self) -> HashValue {
        self.id
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn round(&self) -> u64 {
        self.round
    }

    pub fn proposer(&self) -> AccountAddress {
        self.proposer
    }

    pub fn timestamp_usecs(&self) -> u64 {
        self.timestamp_usecs
    }

    pub fn first_version(&self) -> Version {
        self.first_version
    }
}
