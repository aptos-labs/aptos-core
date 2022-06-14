// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::batch_store::LogicalTime;
use anyhow::bail;
use aptos_crypto::hash::{DefaultHasher};
use aptos_crypto::{ed25519::Ed25519Signature, HashValue};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::validator_signer::ValidatorSigner;
use aptos_types::{validator_verifier::ValidatorVerifier, PeerId};
use bcs::to_bytes;
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use aptos_types::transaction::SignedTransaction;

pub(crate) type BatchId = u64;
pub type Data = Vec<SignedTransaction>;


#[derive(Clone, Eq, Deserialize, Serialize, PartialEq, Debug)]
pub(crate) struct PersistedValue {
    pub(crate) maybe_payload: Option<Data>,
    pub(crate) expiration: LogicalTime,
    pub(crate) author: PeerId,
    pub(crate) num_bytes: usize,
}

impl PersistedValue {
    pub(crate) fn new(
        maybe_payload: Option<Data>,
        expiration: LogicalTime,
        author: PeerId,
        num_bytes: usize,
    ) -> Self {
        Self {
            maybe_payload,
            expiration,
            author,
            num_bytes,
        }
    }

    pub(crate) fn remove_payload(&mut self) {
        self.maybe_payload = None;
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, CryptoHasher, BCSCryptoHash)]
pub struct FragmentInfo {
    epoch: u64,
    batch_id: u64,
    fragment_id: usize,
    payload: Data,
    maybe_expiration: Option<LogicalTime>,
}

#[allow(dead_code)]
impl FragmentInfo {
    fn new(
        epoch: u64,
        batch_id: u64,
        fragment_id: usize,
        fragment_payload: Data,
        maybe_expiration: Option<LogicalTime>,
    ) -> Self {
        Self {
            epoch,
            batch_id,
            fragment_id,
            payload: fragment_payload,
            maybe_expiration,
        }
    }

    pub(crate) fn take_transactions(self) -> Data {
        self.payload
    }

    pub(crate) fn fragment_id(&self) -> usize {
        self.fragment_id
    }

    pub(crate) fn batch_id(&self) -> BatchId {
        self.batch_id
    }

    pub(crate) fn maybe_expiration(&self) -> Option<LogicalTime> {
        self.maybe_expiration.clone()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, CryptoHasher, BCSCryptoHash)]
pub struct Fragment {
    pub source: PeerId,
    pub fragment_info: FragmentInfo,
    pub signature: Ed25519Signature,
}

#[allow(dead_code)]
impl Fragment {
    pub fn new(
        epoch: u64,
        batch_id: u64,
        fragment_id: usize,
        fragment_payload: Data,
        maybe_expiration: Option<LogicalTime>,
        peer_id: PeerId,
        validator_signer: Arc<ValidatorSigner>,
    ) -> Self {
        let fragment_info = FragmentInfo::new(
            epoch,
            batch_id,
            fragment_id,
            fragment_payload,
            maybe_expiration,
        );
        let signature = validator_signer.sign(&fragment_info);
        Self {
            source: peer_id,
            fragment_info,
            signature,
        }
    }

    pub(crate) fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        // TODO: check fragment size.
        if let Some(expiration) = &self.fragment_info.maybe_expiration {
            if expiration.epoch() != self.fragment_info.epoch {
                bail!("Incorrect expiration epoch");
            }
        }
        Ok(validator.verify(self.source, &self.fragment_info, &self.signature)?)
    }

    pub(crate) fn epoch(&self) -> u64 {
        self.fragment_info.epoch
    }

    pub(crate) fn take_transactions(self) -> Data {
        self.fragment_info.take_transactions()
    }

    pub(crate) fn source(&self) -> PeerId {
        self.source
    }

    pub(crate) fn fragment_id(&self) -> usize {
        self.fragment_info.fragment_id()
    }

    pub(crate) fn batch_id(&self) -> BatchId {
        self.fragment_info.batch_id()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, CryptoHasher, BCSCryptoHash)]
pub struct BatchInfo {
    pub(crate) epoch: u64,
    pub(crate) digest: HashValue,
}

#[derive(Clone, Debug, Deserialize, Serialize, CryptoHasher, BCSCryptoHash)]
pub struct Batch {
    pub(crate) source: PeerId,
    // None is a request, Some(payload) is a response.
    pub(crate) maybe_payload: Option<Data>,
    pub(crate) batch_info: BatchInfo,
    pub(crate) maybe_signature: Option<Ed25519Signature>,
}

// TODO: make epoch, source, signature fields treatment consistent across structs.
impl Batch {
    pub fn new(
        epoch: u64,
        source: PeerId,
        digest_hash: HashValue,
        maybe_payload: Option<Data>,
        signer: Arc<ValidatorSigner>,
    ) -> Self {
        let batch_info = BatchInfo {
            epoch,
            digest: digest_hash,
        };
        let signature = if maybe_payload.is_some() {
            Some(signer.sign(&batch_info))
        } else {
            None
        };
        Self {
            source,
            maybe_payload,
            batch_info,
            maybe_signature: signature,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.batch_info.epoch
    }

    //TODO: maybe we should verify signature anyway
    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        if self.maybe_payload.is_some() {
            let mut hasher = DefaultHasher::new(b"QuorumStoreBatch");
            let serialized_payload: Vec<u8> = self
                .maybe_payload
                .as_ref()
                .unwrap()
                .iter()
                .map(|txn| to_bytes(txn).unwrap())
                .flatten()
                .collect();
            hasher.update(&serialized_payload);
            if hasher.finish() == self.batch_info.digest {
                Ok(())
            } else {
                bail!("Payload does not fit digest")
            }
        } else {
            if let Some(signature) = &self.maybe_signature {
                Ok(validator.verify(self.source, &self.batch_info, signature)?)
            } else {
                bail!("Missing signature");
            }
        }
    }

    pub fn get_payload(self) -> Data {
        assert!(self.maybe_payload.is_some());
        self.maybe_payload.unwrap()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, CryptoHasher, BCSCryptoHash)]
pub struct SignedDigestInfo {
    pub(crate) digest: HashValue,
    pub(crate) expiration: LogicalTime,
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
    pub(crate) peer_id: PeerId,
    pub(crate) info: SignedDigestInfo,
    pub(crate) signature: Ed25519Signature,
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
#[derive(Debug)]
#[allow(dead_code)]
pub struct ProofOfStore {
    info: SignedDigestInfo,
    aggregated_signature: BTreeMap<PeerId, Ed25519Signature>,
    // TODO: should we add sender + signature(digest + sender)?
}

#[allow(dead_code)]
impl ProofOfStore {
    pub fn new(info: SignedDigestInfo) -> Self {
        Self {
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

    pub fn verify(&self, validator_verifier: &ValidatorVerifier) -> bool {
        validator_verifier
            .verify_aggregated_struct_signature(&self.info, &self.aggregated_signature)
            .is_ok()
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
}
