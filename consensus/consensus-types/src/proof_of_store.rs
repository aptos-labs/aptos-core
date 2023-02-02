// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::Round;
use anyhow::Context;
use aptos_crypto::{bls12381, CryptoMaterialError, HashValue};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::{
    aggregate_signature::AggregateSignature, validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier, PeerId,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

impl SignedDigestInfo {
    pub fn new(digest: HashValue, expiration: LogicalTime, num_txns: u64, num_bytes: u64) -> Self {
        Self {
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
    peer_id: PeerId,
    info: SignedDigestInfo,
    signature: bls12381::Signature,
}

impl SignedDigest {
    pub fn new(
        epoch: u64,
        digest: HashValue,
        expiration: LogicalTime,
        num_txns: u64,
        num_bytes: u64,
        validator_signer: Arc<ValidatorSigner>,
    ) -> Result<Self, CryptoMaterialError> {
        let info = SignedDigestInfo::new(digest, expiration, num_txns, num_bytes);
        let signature = validator_signer.sign(&info)?;

        Ok(Self {
            epoch,
            peer_id: validator_signer.author(),
            info,
            signature,
        })
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        Ok(validator.verify(self.peer_id, &self.info, &self.signature)?)
    }
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

    pub fn epoch(&self) -> u64 {
        self.info.expiration.epoch
    }
}
