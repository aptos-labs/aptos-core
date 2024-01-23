// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::block_info::Round;
use aptos_crypto::HashValue;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub struct RandMetadataToSign {
    pub epoch: u64,
    pub round: Round,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub struct RandMetadata {
    pub metadata_to_sign: RandMetadataToSign,
    // not used for signing
    pub block_id: HashValue,
    pub timestamp: u64,
}

impl RandMetadata {
    pub fn new(epoch: u64, round: Round, block_id: HashValue, timestamp: u64) -> Self {
        Self {
            metadata_to_sign: RandMetadataToSign { epoch, round },
            block_id,
            timestamp,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // only sign (epoch, round) to produce randomness
        bcs::to_bytes(&self.metadata_to_sign)
            .expect("[RandMessage] RandMetadata serialization failed!")
    }

    pub fn round(&self) -> Round {
        self.metadata_to_sign.round
    }

    pub fn epoch(&self) -> u64 {
        self.metadata_to_sign.epoch
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_for_testing(round: Round) -> Self {
        Self::new(1, round, HashValue::zero(), 1)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Randomness {
    metadata: RandMetadata,
    #[serde(with = "serde_bytes")]
    randomness: Vec<u8>,
}

impl Randomness {
    pub fn new(metadata: RandMetadata, randomness: Vec<u8>) -> Self {
        Self {
            metadata,
            randomness,
        }
    }

    pub fn metadata(&self) -> &RandMetadata {
        &self.metadata
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.metadata_to_sign.epoch
    }

    pub fn round(&self) -> Round {
        self.metadata.metadata_to_sign.round
    }

    pub fn randomness(&self) -> &[u8] {
        &self.randomness
    }
}

impl Default for Randomness {
    fn default() -> Self {
        let metadata = RandMetadata::new(0, 0, HashValue::zero(), 0);
        let randomness = vec![];
        Self {
            metadata,
            randomness,
        }
    }
}
