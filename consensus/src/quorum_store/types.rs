// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use aptos_consensus_types::proof_of_store::LogicalTime;
use aptos_crypto::HashValue;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::{transaction::SignedTransaction, PeerId};
use bcs::to_bytes;
use serde::{Deserialize, Serialize};
use std::mem;

pub(crate) type BatchId = u64;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SerializedTransaction {
    // pub(crate) for testing purposes
    pub(crate) bytes: Vec<u8>,
}

impl SerializedTransaction {
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
pub(crate) struct PersistedValue {
    pub(crate) maybe_payload: Option<Vec<SignedTransaction>>,
    pub(crate) expiration: LogicalTime,
    pub(crate) author: PeerId,
    pub(crate) num_bytes: usize,
}

impl PersistedValue {
    pub(crate) fn new(
        maybe_payload: Option<Vec<SignedTransaction>>,
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
    payload: Vec<SerializedTransaction>,
    maybe_expiration: Option<LogicalTime>,
}

impl FragmentInfo {
    fn new(
        epoch: u64,
        batch_id: u64,
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

    pub(crate) fn take_transactions(self) -> Vec<SerializedTransaction> {
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
    // pub signature: Ed25519Signature,
}

impl Fragment {
    pub fn new(
        epoch: u64,
        batch_id: u64,
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

    pub(crate) fn verify(&self, peer_id: PeerId) -> anyhow::Result<()> {
        if let Some(expiration) = &self.fragment_info.maybe_expiration {
            if expiration.epoch() != self.fragment_info.epoch {
                bail!("Incorrect expiration epoch");
            }
        }
        if self.source == peer_id {
            Ok(())
        } else {
            bail!("wrong sender");
        }
    }

    pub(crate) fn epoch(&self) -> u64 {
        self.fragment_info.epoch
    }

    pub(crate) fn take_transactions(self) -> Vec<SerializedTransaction> {
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, CryptoHasher, BCSCryptoHash)]
pub struct BatchInfo {
    pub(crate) epoch: u64,
    pub(crate) digest: HashValue,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, CryptoHasher, BCSCryptoHash)]
pub struct Batch {
    pub(crate) source: PeerId,
    // None is a request, Some(payload) is a response.
    pub(crate) maybe_payload: Option<Vec<SignedTransaction>>,
    pub(crate) batch_info: BatchInfo,
}

// TODO: make epoch, source, signature fields treatment consistent across structs.
impl Batch {
    pub fn new(
        epoch: u64,
        source: PeerId,
        digest_hash: HashValue,
        maybe_payload: Option<Vec<SignedTransaction>>,
    ) -> Self {
        let batch_info = BatchInfo {
            epoch,
            digest: digest_hash,
        };
        Self {
            source,
            maybe_payload,
            batch_info,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.batch_info.epoch
    }

    // Check the source == the sender. To protect from DDoS we check is Payload matches digest later.
    pub fn verify(&self, peer_id: PeerId) -> anyhow::Result<()> {
        if self.source == peer_id {
            Ok(())
        } else {
            bail!("wrong sender");
        }
    }

    pub fn get_payload(self) -> Vec<SignedTransaction> {
        self.maybe_payload.expect("Batch contains no payload")
    }
}
