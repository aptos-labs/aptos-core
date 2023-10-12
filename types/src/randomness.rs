// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use serde::{Deserialize, Serialize};

use crate::block_info::Round;

// Each validator will send a randomness share of size rand_size * rand_num / NUM_VALIDATORS (assuming even stake distribution)
pub const RAND_SIZE: usize = 96;
pub const NUM_SHARES_PER_VALIDATOR: usize = 1;
pub const PROOF_SIZE: usize = 1;
pub const SHARE_SIZE: usize = RAND_SIZE * NUM_SHARES_PER_VALIDATOR;
pub const DECISION_SIZE: usize = RAND_SIZE;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, CryptoHasher, BCSCryptoHash)]
pub struct RandMetadata {
    epoch: u64,
    round: Round,
    bloch_id: HashValue,
    timestamp: u64,
}

impl RandMetadata {
    pub fn new(epoch: u64, round: Round, bloch_id: HashValue, timestamp: u64) -> Self {
        Self { epoch, round, bloch_id, timestamp }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, CryptoHasher, BCSCryptoHash)]
pub struct RandProof {
    // rand todo: fill
    bytes: Vec<u8>,
}

impl RandProof {
    pub fn new_for_test() -> Self {
        Self { bytes: vec![0; PROOF_SIZE] }
    }
}

#[derive(Clone, Serialize, Deserialize, CryptoHasher, Debug, PartialEq, Eq)]
pub struct Randomness {
    metadata: RandMetadata,
    value: Vec<u8>,    // rand todo: fill
}

impl Randomness {
    pub fn new(metadata: RandMetadata, value: Vec<u8>) -> Self {
        Self { metadata, value }
    }

    pub fn new_for_test(epoch: u64, round: Round, block_hash: HashValue, timestamp: u64) -> Self {
        let metadata = RandMetadata::new(epoch, round, block_hash, timestamp);
        let value = vec![0; RAND_SIZE];
        Self { metadata, value }
    }

    // Only used for the execution interface of ordering_state_computer which does not actually execute
    pub fn dummy() -> Self {
        let metadata = RandMetadata::new(0, 0, HashValue::zero(), 0);
        let value = vec![];
        Self { metadata, value }
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.epoch
    }

    pub fn round(&self) -> Round {
        self.metadata.round
    }

    pub fn block_id(&self) -> HashValue {
        self.metadata.bloch_id
    }

    pub fn timestamp(&self) -> u64 {
        self.metadata.timestamp
    }
}

#[derive(Clone, Serialize, Deserialize, CryptoHasher, Debug, PartialEq, Eq)]
pub struct RandDecision {
    randomness: Randomness,
    proof: RandProof,
}

impl RandDecision {
    pub fn new(randomness: Randomness, proof: RandProof) -> Self {
        Self { randomness, proof }
    }

    pub fn new_for_test(epoch: u64, round: Round, block_hash: HashValue, timestamp: u64) -> Self {
        let metadata = RandMetadata::new(epoch, round, block_hash, timestamp);
        let randomness = Randomness::new(metadata, vec![0; DECISION_SIZE]);
        let proof = RandProof { bytes: vec![0; PROOF_SIZE] };
        Self { randomness, proof }
    }

    pub fn randomness(&self) -> &Randomness {
        &self.randomness
    }

    pub fn proof(&self) -> &RandProof {
        &self.proof
    }

    pub fn epoch(&self) -> u64 {
        self.randomness.epoch()
    }

    pub fn round(&self) -> Round {
        self.randomness.round()
    }

    pub fn block_id(&self) -> HashValue {
        self.randomness.block_id()
    }

    pub fn timestamp(&self) -> u64 {
        self.randomness.timestamp()
    }

    pub fn verify(&self, _rand_config: &RandConfig) -> anyhow::Result<()> {
        // rand todo: fill

        Ok(())
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct RandConfig {
    // rand todo: fill
    weights: Vec<u64>,
    weight_f: u64,  // fallback threshold
    weight_o: u64,  // optimistic threshold
}

impl RandConfig {
    pub fn new(weights: Vec<u64>, weight_f: u64, weight_o: u64) -> Self {
        Self { weights, weight_f, weight_o }
    }

    pub fn new_for_testing(num_validators: usize) -> Self {
        let weights = vec![NUM_SHARES_PER_VALIDATOR as u64, num_validators as u64];
        let num_shares = NUM_SHARES_PER_VALIDATOR * num_validators;
        Self { weights, weight_f: (num_shares / 3) as u64, weight_o: (num_shares * 2 / 3) as u64 }
    }

    pub fn weight_f(&self) -> u64 {
        self.weight_f
    }

    pub fn weight_o(&self) -> u64 {
        self.weight_o
    }
}