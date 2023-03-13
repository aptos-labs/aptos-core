// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::Round;
use anyhow::Context;
use aptos_crypto::{bls12381, CryptoMaterialError, HashValue};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::{
    aggregate_signature::AggregateSignature, validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier, PeerId,
};
use rand::{seq::SliceRandom, thread_rng};
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
    pub batch_author: PeerId,
    pub digest: HashValue,
    pub expiration: LogicalTime,
    pub num_txns: u64,
    pub num_bytes: u64,
}

impl SignedDigestInfo {
    pub fn new(
        batch_author: PeerId,
        digest: HashValue,
        expiration: LogicalTime,
        num_txns: u64,
        num_bytes: u64,
    ) -> Self {
        Self {
            batch_author,
            digest,
            expiration,
            num_txns,
            num_bytes,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SignedDigest {
    epoch: u64,
    signer: PeerId,
    info: SignedDigestInfo,
    signature: bls12381::Signature,
}

impl SignedDigest {
    pub fn new(
        batch_author: PeerId,
        epoch: u64,
        digest: HashValue,
        expiration: LogicalTime,
        num_txns: u64,
        num_bytes: u64,
        validator_signer: &ValidatorSigner,
    ) -> Result<Self, CryptoMaterialError> {
        let info = SignedDigestInfo::new(batch_author, digest, expiration, num_txns, num_bytes);
        let signature = validator_signer.sign(&info)?;

        Ok(Self {
            epoch,
            signer: validator_signer.author(),
            info,
            signature,
        })
    }

    pub fn signer(&self) -> PeerId {
        self.signer
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        Ok(validator.verify(self.signer, &self.info, &self.signature)?)
    }

    pub fn info(&self) -> &SignedDigestInfo {
        &self.info
    }

    pub fn signature(self) -> bls12381::Signature {
        self.signature
    }

    pub fn digest(&self) -> HashValue {
        self.info.digest
    }
}

#[derive(Debug, PartialEq)]
pub enum SignedDigestError {
    WrongInfo,
    DuplicatedSignature,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct ProofOfStore {
    info: SignedDigestInfo,
    multi_signature: AggregateSignature,
}

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

    pub fn shuffled_signers(&self, validator: &ValidatorVerifier) -> Vec<PeerId> {
        let mut ret: Vec<PeerId> = self
            .multi_signature
            .get_voter_addresses(&validator.get_ordered_account_addresses());
        ret.shuffle(&mut thread_rng());
        ret
    }

    pub fn epoch(&self) -> u64 {
        self.info.expiration.epoch
    }
}
