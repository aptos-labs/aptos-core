// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::ensure;
use aptos_consensus_types::{
    common::{BatchPayload, TxnSummaryWithExpiration},
    proof_of_store::BatchInfo,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures, quorum_store::BatchId, transaction::SignedTransaction,
    PeerId,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    ops::Deref,
};

#[derive(Clone, Eq, Deserialize, Serialize, PartialEq, Debug)]
pub struct PersistedValue {
    info: BatchInfo,
    maybe_payload: Option<Vec<SignedTransaction>>,
}

#[derive(PartialEq, Debug)]
pub(crate) enum StorageMode {
    PersistedOnly,
    MemoryAndPersisted,
}

impl PersistedValue {
    pub(crate) fn new(info: BatchInfo, maybe_payload: Option<Vec<SignedTransaction>>) -> Self {
        Self {
            info,
            maybe_payload,
        }
    }

    pub(crate) fn payload_storage_mode(&self) -> StorageMode {
        match self.maybe_payload {
            Some(_) => StorageMode::MemoryAndPersisted,
            None => StorageMode::PersistedOnly,
        }
    }

    pub(crate) fn take_payload(&mut self) -> Option<Vec<SignedTransaction>> {
        self.maybe_payload.take()
    }

    #[allow(dead_code)]
    pub(crate) fn remove_payload(&mut self) {
        self.maybe_payload = None;
    }

    pub fn batch_info(&self) -> &BatchInfo {
        &self.info
    }

    pub fn payload(&self) -> &Option<Vec<SignedTransaction>> {
        &self.maybe_payload
    }

    pub fn summary(&self) -> Vec<TxnSummaryWithExpiration> {
        if let Some(payload) = &self.maybe_payload {
            return payload
                .iter()
                .map(|txn| {
                    TxnSummaryWithExpiration::new(
                        txn.sender(),
                        txn.replay_protector(),
                        txn.expiration_timestamp_secs(),
                        txn.committed_hash(),
                    )
                })
                .collect();
        }
        vec![]
    }

    pub fn unpack(self) -> (BatchInfo, Option<Vec<SignedTransaction>>) {
        (self.info, self.maybe_payload)
    }
}

impl Deref for PersistedValue {
    type Target = BatchInfo;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

impl TryFrom<PersistedValue> for Batch {
    type Error = anyhow::Error;

    fn try_from(value: PersistedValue) -> Result<Self, Self::Error> {
        let author = value.author();
        Ok(Batch {
            batch_info: value.info,
            payload: BatchPayload::new(
                author,
                value
                    .maybe_payload
                    .ok_or_else(|| anyhow::anyhow!("Payload not exist"))?,
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use aptos_config::config;

    #[test]
    fn test_batch_payload_padding() {
        use super::*;
        let empty_batch_payload = BatchPayload::new(PeerId::random(), vec![]);
        // We overestimate the ULEB128 encoding of the number of transactions as 128 bytes.
        assert_eq!(
            empty_batch_payload.num_bytes() + 127,
            config::BATCH_PADDING_BYTES
        );
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Batch {
    batch_info: BatchInfo,
    payload: BatchPayload,
}

impl Batch {
    pub fn new(
        batch_id: BatchId,
        payload: Vec<SignedTransaction>,
        epoch: u64,
        expiration: u64,
        batch_author: PeerId,
        gas_bucket_start: u64,
    ) -> Self {
        let payload = BatchPayload::new(batch_author, payload);
        let batch_info = BatchInfo::new(
            batch_author,
            batch_id,
            epoch,
            expiration,
            payload.hash(),
            payload.num_txns() as u64,
            payload.num_bytes() as u64,
            gas_bucket_start,
        );
        Self {
            batch_info,
            payload,
        }
    }

    pub fn verify(&self) -> anyhow::Result<()> {
        ensure!(
            self.payload.author() == self.author(),
            "Payload author doesn't match the info"
        );
        ensure!(
            self.payload.hash() == *self.digest(),
            "Payload hash doesn't match the digest"
        );
        ensure!(
            self.payload.num_txns() as u64 == self.num_txns(),
            "Payload num txns doesn't match batch info"
        );
        ensure!(
            self.payload.num_bytes() as u64 == self.num_bytes(),
            "Payload num bytes doesn't match batch info"
        );
        for txn in self.payload.txns() {
            ensure!(
                txn.gas_unit_price() >= self.gas_bucket_start(),
                "Payload gas unit price doesn't match batch info"
            )
        }
        Ok(())
    }

    /// Verify the batch, and that it matches the requested digest
    pub fn verify_with_digest(&self, requested_digest: HashValue) -> anyhow::Result<()> {
        ensure!(
            requested_digest == *self.digest(),
            "Response digest doesn't match the request"
        );
        self.verify()?;
        Ok(())
    }

    pub fn into_transactions(self) -> Vec<SignedTransaction> {
        self.payload.into_transactions()
    }

    pub fn txns(&self) -> &[SignedTransaction] {
        self.payload.txns()
    }

    pub fn batch_info(&self) -> &BatchInfo {
        &self.batch_info
    }
}

impl Deref for Batch {
    type Target = BatchInfo;

    fn deref(&self) -> &Self::Target {
        &self.batch_info
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BatchRequest {
    epoch: u64,
    source: PeerId,
    digest: HashValue,
}

impl Display for BatchRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BatchRequest: epoch: {}, source: {}, digest {}",
            self.epoch, self.source, self.digest
        )
    }
}

impl BatchRequest {
    pub fn new(source: PeerId, epoch: u64, digest: HashValue) -> Self {
        Self {
            epoch,
            source,
            digest,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
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
        self.digest
    }
}

impl From<Batch> for PersistedValue {
    fn from(value: Batch) -> Self {
        let Batch {
            batch_info,
            payload,
        } = value;
        PersistedValue::new(batch_info, Some(payload.into_transactions()))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum BatchResponse {
    Batch(Batch),
    NotFound(LedgerInfoWithSignatures),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BatchMsg {
    batches: Vec<Batch>,
}

impl BatchMsg {
    pub fn new(batches: Vec<Batch>) -> Self {
        Self { batches }
    }

    pub fn verify(&self, peer_id: PeerId, max_num_batches: usize) -> anyhow::Result<()> {
        ensure!(!self.batches.is_empty(), "Empty message");
        ensure!(
            self.batches.len() <= max_num_batches,
            "Too many batches: {} > {}",
            self.batches.len(),
            max_num_batches
        );
        for batch in self.batches.iter() {
            ensure!(
                batch.author() == peer_id,
                "Batch author doesn't match sender"
            );
            batch.verify()?
        }
        Ok(())
    }

    pub fn epoch(&self) -> anyhow::Result<u64> {
        ensure!(!self.batches.is_empty(), "Empty message");
        let epoch = self.batches[0].epoch();
        for batch in self.batches.iter() {
            ensure!(
                batch.epoch() == epoch,
                "Epoch mismatch: {} != {}",
                batch.epoch(),
                epoch
            );
        }
        Ok(epoch)
    }

    pub fn author(&self) -> Option<PeerId> {
        self.batches.first().map(|batch| batch.author())
    }

    pub fn take(self) -> Vec<Batch> {
        self.batches
    }
}
