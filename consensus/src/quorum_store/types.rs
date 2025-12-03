// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

use anyhow::ensure;
use aptos_consensus_types::{
    common::{BatchPayload, TxnSummaryWithExpiration},
    proof_of_store::{BatchInfo, BatchInfoExt, BatchKind, TBatchInfo},
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures, quorum_store::BatchId, transaction::SignedTransaction,
    validator_verifier::ValidatorVerifier, PeerId,
};
use serde::{Deserialize, Serialize};
use serde_name::{DeserializeNameAdapter, SerializeNameAdapter};
use std::{
    fmt::{Display, Formatter},
    ops::Deref,
};

#[derive(Clone, Eq, Deserialize, Serialize, PartialEq, Debug)]
pub struct PersistedValue<T> {
    info: T,
    maybe_payload: Option<Vec<SignedTransaction>>,
}

#[derive(PartialEq, Debug)]
pub(crate) enum StorageMode {
    PersistedOnly,
    MemoryAndPersisted,
}

impl<T: TBatchInfo> PersistedValue<T> {
    pub(crate) fn new(info: T, maybe_payload: Option<Vec<SignedTransaction>>) -> Self {
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

    pub fn batch_info(&self) -> &T {
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

    pub fn unpack(self) -> (T, Option<Vec<SignedTransaction>>) {
        (self.info, self.maybe_payload)
    }
}

impl<T: TBatchInfo> Deref for PersistedValue<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

impl<T: TBatchInfo> TryFrom<PersistedValue<T>> for Batch<T> {
    type Error = anyhow::Error;

    fn try_from(value: PersistedValue<T>) -> Result<Self, Self::Error> {
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

impl From<PersistedValue<BatchInfo>> for PersistedValue<BatchInfoExt> {
    fn from(value: PersistedValue<BatchInfo>) -> Self {
        let (batch_info, payload) = value.unpack();
        PersistedValue::new(batch_info.into(), payload)
    }
}

impl TryFrom<PersistedValue<BatchInfoExt>> for PersistedValue<BatchInfo> {
    type Error = anyhow::Error;

    fn try_from(value: PersistedValue<BatchInfoExt>) -> Result<Self, Self::Error> {
        let (batch_info, payload) = value.unpack();
        ensure!(!batch_info.is_v2(), "Expected Batch Info V1");
        Ok(PersistedValue::new(batch_info.unpack_info(), payload))
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
#[serde(remote = "Batch")]
pub struct Batch<T: TBatchInfo> {
    batch_info: T,
    payload: BatchPayload,
}

impl<'de, T> Deserialize<'de> for Batch<T>
where
    T: Deserialize<'de> + TBatchInfo,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        Batch::deserialize(DeserializeNameAdapter::new(
            deserializer,
            std::any::type_name::<Self>(),
        ))
    }
}

impl<T> Serialize for Batch<T>
where
    T: Serialize + TBatchInfo,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        Batch::serialize(
            self,
            SerializeNameAdapter::new(serializer, std::any::type_name::<Self>()),
        )
    }
}

impl Batch<BatchInfo> {
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
        Self::new_generic(batch_info, payload)
    }
}

impl Batch<BatchInfoExt> {
    pub fn new_v2(
        batch_id: BatchId,
        payload: Vec<SignedTransaction>,
        epoch: u64,
        expiration: u64,
        batch_author: PeerId,
        gas_bucket_start: u64,
        batch_kind: BatchKind,
    ) -> Self {
        let payload = BatchPayload::new(batch_author, payload);
        let batch_info = BatchInfoExt::new_v2(
            batch_author,
            batch_id,
            epoch,
            expiration,
            payload.hash(),
            payload.num_txns() as u64,
            payload.num_bytes() as u64,
            gas_bucket_start,
            batch_kind,
        );
        Self::new_generic(batch_info, payload)
    }

    pub fn new_v1(
        batch_id: BatchId,
        payload: Vec<SignedTransaction>,
        epoch: u64,
        expiration: u64,
        batch_author: PeerId,
        gas_bucket_start: u64,
    ) -> Self {
        let payload = BatchPayload::new(batch_author, payload);
        let batch_info = BatchInfoExt::new_v1(
            batch_author,
            batch_id,
            epoch,
            expiration,
            payload.hash(),
            payload.num_txns() as u64,
            payload.num_bytes() as u64,
            gas_bucket_start,
        );
        Self::new_generic(batch_info, payload)
    }
}

impl<T: TBatchInfo> Batch<T> {
    pub fn new_generic(batch_info: T, payload: BatchPayload) -> Self {
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
            );
            ensure!(
                !txn.payload().is_encrypted_variant(),
                "Encrypted transaction is not supported yet"
            );
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

    pub fn batch_info(&self) -> &T {
        &self.batch_info
    }
}

impl<T: TBatchInfo> Deref for Batch<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.batch_info
    }
}

impl From<Batch<BatchInfo>> for Batch<BatchInfoExt> {
    fn from(batch: Batch<BatchInfo>) -> Self {
        let Batch {
            batch_info,
            payload,
        } = batch;
        Self {
            batch_info: batch_info.into(),
            payload,
        }
    }
}

impl TryFrom<Batch<BatchInfoExt>> for Batch<BatchInfo> {
    type Error = anyhow::Error;

    fn try_from(batch: Batch<BatchInfoExt>) -> Result<Self, Self::Error> {
        ensure!(
            matches!(batch.batch_info(), &BatchInfoExt::V1 { .. }),
            "Batch must be V1 type"
        );
        let Batch {
            batch_info,
            payload,
        } = batch;
        Ok(Self {
            batch_info: batch_info.unpack_info(),
            payload,
        })
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

impl<T: TBatchInfo> From<Batch<T>> for PersistedValue<T> {
    fn from(value: Batch<T>) -> Self {
        let Batch {
            batch_info,
            payload,
        } = value;
        PersistedValue::new(batch_info, Some(payload.into_transactions()))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum BatchResponse {
    Batch(Batch<BatchInfo>),
    NotFound(LedgerInfoWithSignatures),
    BatchV2(Batch<BatchInfoExt>),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BatchMsg<T: TBatchInfo> {
    batches: Vec<Batch<T>>,
}

impl<T: TBatchInfo> BatchMsg<T> {
    pub fn new(batches: Vec<Batch<T>>) -> Self {
        Self { batches }
    }

    pub fn verify(
        &self,
        peer_id: PeerId,
        max_num_batches: usize,
        verifier: &ValidatorVerifier,
    ) -> anyhow::Result<()> {
        ensure!(!self.batches.is_empty(), "Empty message");
        ensure!(
            self.batches.len() <= max_num_batches,
            "Too many batches: {} > {}",
            self.batches.len(),
            max_num_batches
        );
        let epoch_authors = verifier.address_to_validator_index();
        for batch in self.batches.iter() {
            ensure!(
                epoch_authors.contains_key(&batch.author()),
                "Invalid author {} for batch {} in current epoch",
                batch.author(),
                batch.digest()
            );
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

    pub fn take(self) -> Vec<Batch<T>> {
        self.batches
    }
}

impl From<BatchMsg<BatchInfo>> for BatchMsg<BatchInfoExt> {
    fn from(msg: BatchMsg<BatchInfo>) -> Self {
        Self {
            batches: msg.batches.into_iter().map(|b| b.into()).collect(),
        }
    }
}
