// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::Round;
use aptos_types::{validator_verifier::ValidatorVerifier, block_info::BlockInfo};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

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
