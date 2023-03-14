// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::proof_of_store::{BatchId, LogicalTime};
use aptos_crypto::{hash::DefaultHasher, HashValue};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::{transaction::SignedTransaction, PeerId};
use bcs::to_bytes;
use serde::{Deserialize, Serialize};
use std::mem;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SerializedTransaction {
    // pub for testing purposes
    #[serde(with = "serde_bytes")]
    pub bytes: Vec<u8>,
}

impl SerializedTransaction {
    #[cfg(test)]
    pub(crate) fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    pub fn from_signed_txn(txn: &SignedTransaction) -> Self {
        Self {
            bytes: to_bytes(&txn).unwrap(),
        }
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn bytes(&self) -> &Vec<u8> {
        &self.bytes
    }

    pub fn take_bytes(&mut self) -> Vec<u8> {
        mem::take(&mut self.bytes)
    }
}

#[derive(Clone, Eq, Deserialize, Serialize, PartialEq, Debug)]
pub struct PersistedValue {
    pub maybe_payload: Option<Vec<SignedTransaction>>,
    pub expiration: LogicalTime,
    pub author: PeerId,
    pub batch_id: BatchId,
    pub num_bytes: usize,
}

impl PersistedValue {
    pub(crate) fn new(
        maybe_payload: Option<Vec<SignedTransaction>>,
        expiration: LogicalTime,
        author: PeerId,
        batch_id: BatchId,
        num_bytes: usize,
    ) -> Self {
        Self {
            maybe_payload,
            expiration,
            author,
            batch_id,
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
    batch_id: BatchId,
    fragment_id: usize,
    payload: Vec<SerializedTransaction>,
    maybe_expiration: Option<LogicalTime>,
}

impl FragmentInfo {
    fn new(
        epoch: u64,
        batch_id: BatchId,
        fragment_id: usize,
        fragment_payload: Vec<SerializedTransaction>,
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

    pub fn into_transactions(self) -> Vec<SerializedTransaction> {
        self.payload
    }

    pub fn fragment_id(&self) -> usize {
        self.fragment_id
    }

    pub fn batch_id(&self) -> BatchId {
        self.batch_id
    }

    pub fn maybe_expiration(&self) -> Option<LogicalTime> {
        self.maybe_expiration
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Fragment {
    source: PeerId,
    fragment_info: FragmentInfo,
}

impl Fragment {
    pub fn new(
        epoch: u64,
        batch_id: BatchId,
        fragment_id: usize,
        fragment_payload: Vec<SerializedTransaction>,
        maybe_expiration: Option<LogicalTime>,
        peer_id: PeerId,
    ) -> Self {
        let fragment_info = FragmentInfo::new(
            epoch,
            batch_id,
            fragment_id,
            fragment_payload,
            maybe_expiration,
        );
        Self {
            source: peer_id,
            fragment_info,
        }
    }

    pub fn verify(&self, peer_id: PeerId) -> anyhow::Result<()> {
        if let Some(expiration) = &self.fragment_info.maybe_expiration() {
            if expiration.epoch() != self.fragment_info.epoch {
                return Err(anyhow::anyhow!(
                    "Epoch mismatch: info: {}, expiration: {}",
                    expiration.epoch(),
                    self.fragment_info.epoch
                ));
            }
        }
        if self.source == peer_id {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Sender mismatch: peer_id: {}, source: {}",
                self.source,
                peer_id
            ))
        }
    }

    pub fn epoch(&self) -> u64 {
        self.fragment_info.epoch
    }

    pub fn into_transactions(self) -> Vec<SerializedTransaction> {
        self.fragment_info.into_transactions()
    }

    pub fn source(&self) -> PeerId {
        self.source
    }

    pub fn fragment_id(&self) -> usize {
        self.fragment_info.fragment_id()
    }

    pub fn batch_id(&self) -> BatchId {
        self.fragment_info.batch_id()
    }

    pub fn maybe_expiration(&self) -> Option<LogicalTime> {
        self.fragment_info.maybe_expiration()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BatchInfo {
    pub epoch: u64,
    pub digest: HashValue,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BatchRequest {
    source: PeerId,
    batch_info: BatchInfo,
}

impl BatchRequest {
    pub fn new(source: PeerId, epoch: u64, digest: HashValue) -> Self {
        let batch_info = BatchInfo { epoch, digest };
        Self { source, batch_info }
    }

    pub fn epoch(&self) -> u64 {
        self.batch_info.epoch
    }

    pub fn verify(&self, peer_id: PeerId) -> anyhow::Result<()> {
        if self.source == peer_id {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Sender mismatch: peer_id: {}, source: {}",
                self.source,
                peer_id
            ))
        }
    }

    pub fn source(&self) -> PeerId {
        self.source
    }

    pub fn digest(&self) -> HashValue {
        self.batch_info.digest
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Batch {
    source: PeerId,
    batch_info: BatchInfo,
    payload: Vec<SignedTransaction>,
}

impl Batch {
    pub fn new(
        source: PeerId,
        epoch: u64,
        digest: HashValue,
        payload: Vec<SignedTransaction>,
    ) -> Self {
        let batch_info = BatchInfo { epoch, digest };
        Self {
            source,
            batch_info,
            payload,
        }
    }

    pub fn source(&self) -> PeerId {
        self.source
    }

    pub fn epoch(&self) -> u64 {
        self.batch_info.epoch
    }

    pub fn verify(&self) -> anyhow::Result<()> {
        let mut hasher = DefaultHasher::new(b"QuorumStoreBatch");
        let serialized_payload: Vec<u8> = self
            .payload
            .iter()
            .flat_map(|txn| to_bytes(txn).unwrap())
            .collect();
        hasher.update(&serialized_payload);
        if hasher.finish() == self.digest() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Digest doesn't match"))
        }
    }

    pub fn into_payload(self) -> Vec<SignedTransaction> {
        self.payload
    }

    pub fn digest(&self) -> HashValue {
        self.batch_info.digest
    }
}
