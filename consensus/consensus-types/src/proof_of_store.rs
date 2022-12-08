// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::Round;
use anyhow::Context;
use aptos_crypto::HashValue;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::aggregate_signature::AggregateSignature;
use aptos_types::validator_verifier::ValidatorVerifier;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Deserialize, Serialize, Hash)]
pub struct LogicalTime {
    epoch: u64,
    round: Round,
}

impl LogicalTime {
    pub fn new(epoch: u64, round: Round) -> Self {
        Self { epoch, round }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn round(&self) -> Round {
        self.round
    }
}

#[derive(
    Clone, Debug, Deserialize, Serialize, CryptoHasher, BCSCryptoHash, PartialEq, Eq, Hash,
)]
pub struct SignedDigestInfo {
    pub digest: HashValue,
    pub expiration: LogicalTime,
    pub num_txns: u64,
    pub num_bytes: u64,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct ProofOfStore {
    info: SignedDigestInfo,
    multi_signature: AggregateSignature,
}

#[allow(dead_code)]
impl ProofOfStore {
    pub fn new(info: SignedDigestInfo, multi_signature: AggregateSignature) -> Self {
        Self {
            info,
            multi_signature,
        }
    }

    pub fn info(&self) -> &SignedDigestInfo {
        &self.info
    }

    pub fn digest(&self) -> &HashValue {
        &self.info.digest
    }

    pub fn expiration(&self) -> LogicalTime {
        self.info.expiration
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        validator
            .verify_multi_signatures(&self.info, &self.multi_signature)
            .context("Failed to verify ProofOfStore")
    }

    pub fn epoch(&self) -> u64 {
        self.info.expiration.epoch
    }
}
