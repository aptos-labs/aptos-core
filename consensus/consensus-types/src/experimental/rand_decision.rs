// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::common::{Round, Author};
use aptos_crypto::HashValue;
use aptos_short_hex_str::AsShortHexStr;
use aptos_types::{validator_verifier::ValidatorVerifier, block_info::BlockInfo};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

type Epoch = u64;

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct RandDecision {
    block_info: BlockInfo,
    rand: Vec<u8>,  // place holder for aggregated VRF randomness
}

// this is required by structured log
impl Debug for RandDecision {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for RandDecision {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Randomness Decision: [{}]", self.block_info)
    }
}

impl RandDecision {
    pub fn new(block_info: BlockInfo, rand: Vec<u8>) -> Self {
        Self { block_info, rand }
    }

    pub fn round(&self) -> Round {
        self.block_info.round()
    }

    pub fn epoch(&self) -> u64 {
        self.block_info.epoch()
    }

    pub fn block_info(&self) -> &BlockInfo {
        &self.block_info
    }

    /// Verifies that the signatures carried in the message forms a valid quorum,
    /// and then verifies the signature.
    pub fn verify(&self, _validator: &ValidatorVerifier) -> anyhow::Result<()> {
        // todo: also need to verify the validity of the aggregated VRF
        Ok(())
    }

    pub fn rand(&self) -> &Vec<u8> {
        &self.rand
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct RandDecisions {
    item_id: HashValue,   // hash of the ordered_item
    epoch: Epoch,
    decisions: Vec<Option<RandDecision>>,
    author: Author,
}

// this is required by structured log
impl Debug for RandDecisions {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for RandDecisions {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "RandDecisions: [item_id: {}, epoch {}, decisions {:?}]",
            self.item_id.short_str(),
            self.epoch,
            self.decisions,
        )
    }
}

impl RandDecisions {
    /// Generates a new RandShare
    pub fn new(
        item_id: HashValue,
        epoch: Epoch,
        decisions: Vec<Option<RandDecision>>,
        author: Author,
    ) -> Self {
        Self {
            item_id,
            epoch,
            decisions,
            author,
        }
    }

    pub fn item_id(&self) -> HashValue {
        self.item_id
    }

    pub fn epoch(&self) -> Epoch {
        self.epoch
    }

    /// Verifies that the consensus data hash of LedgerInfo corresponds to the commit proposal,
    /// and then verifies the signature.
    pub fn verify(&self, _validator: &ValidatorVerifier) -> anyhow::Result<()> {
        // todo: also need to verify the validity of the VRF share
        Ok(())
    }

    pub fn decisions(&self) -> &Vec<Option<RandDecision>> {
        &self.decisions
    }

    pub fn rounds(&self) -> Vec<Round> {
        self.decisions.iter().filter_map(|s| s.as_ref().map(|share| share.round())).collect()
    }

    pub fn timestamps(&self) -> Vec<u64> {
        self.decisions.iter().filter_map(|s| s.as_ref().map(|share| share.block_info().timestamp_usecs())).collect()
    }

    pub fn author(&self) -> Author {
        self.author
    }
}
