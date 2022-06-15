// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::Round;
use anyhow::Context;
use aptos_crypto::ed25519::Ed25519Signature;
use aptos_crypto::HashValue;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::account_address::AccountAddress as PeerId;
use aptos_types::validator_signer::ValidatorSigner;
use aptos_types::validator_verifier::ValidatorVerifier;
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
}

impl SignedDigestInfo {
    pub fn new(digest: HashValue, expiration: LogicalTime) -> Self {
        Self { digest, expiration }
    }
}

// TODO: implement properly (and proper place) w.o. public fields.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SignedDigest {
    epoch: u64,
    pub peer_id: PeerId,
    pub info: SignedDigestInfo,
    pub signature: Ed25519Signature,
}

impl SignedDigest {
    pub fn new(
        epoch: u64,
        peer_id: PeerId,
        digest: HashValue,
        expiration: LogicalTime,
        validator_signer: Arc<ValidatorSigner>,
    ) -> Self {
        let info = SignedDigestInfo::new(digest, expiration);
        let signature = validator_signer.sign(&info);

        Self {
            epoch,
            peer_id,
            info,
            signature,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        Ok(validator.verify(self.peer_id, &self.info, &self.signature)?)
    }
}

#[derive(Debug, PartialEq)]
pub enum SignedDigestError {
    WrongDigest,
    DuplicatedSignature,
}

//TODO: sign hashValue and expiration - make ProofOfStoreInfo and sign it
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub struct ProofOfStore {
    epoch: u64,
    info: SignedDigestInfo,
    aggregated_signature: BTreeMap<PeerId, Ed25519Signature>,
    // TODO: should we add sender + signature(digest + sender)?
}

#[allow(dead_code)]
impl ProofOfStore {
    pub fn new(epoch: u64, info: SignedDigestInfo) -> Self {
        Self {
            epoch,
            info,
            aggregated_signature: BTreeMap::new(),
        }
    }

    pub fn digest(&self) -> &HashValue {
        &self.info.digest
    }

    pub fn ready(&self, validator_verifier: &ValidatorVerifier) -> bool {
        validator_verifier
            .check_voting_power(self.aggregated_signature.keys())
            .is_ok()
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        validator
            .verify_aggregated_struct_signature(&self.info, &self.aggregated_signature)
            .context("Failed to verify ProofOfStore")
    }

    pub fn shuffled_signers(&self) -> Vec<PeerId> {
        let mut ret: Vec<PeerId> = self.aggregated_signature.keys().cloned().collect();
        ret.shuffle(&mut thread_rng());
        ret
    }

    pub fn add_signature(
        &mut self,
        signer_id: PeerId,
        signature: Ed25519Signature,
    ) -> Result<(), SignedDigestError> {
        if self.aggregated_signature.contains_key(&signer_id) {
            return Err(SignedDigestError::DuplicatedSignature);
        }

        self.aggregated_signature.insert(signer_id, signature);
        return Ok(());
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}
