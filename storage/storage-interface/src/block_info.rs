// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::hash::HashValue;
use aptos_types::{account_address::AccountAddress, transaction::Version};
use serde::{Deserialize, Serialize};
use std::ops::Deref;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub enum BlockInfo {
    V0(BlockInfoV0),
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
    pub fn new(
        id: HashValue,
        epoch: u64,
        round: u64,
        proposer: AccountAddress,
        timestamp_usecs: u64,
        first_version: Version,
    ) -> Self {
        Self {
            id,
            epoch,
            round,
            proposer,
            timestamp_usecs,
            first_version,
        }
    }

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
