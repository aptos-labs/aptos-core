use crate::common::{Author, Payload, Round};
use anyhow::ensure;
use aptos_crypto::{
    hash::{CryptoHash, CryptoHasher},
    HashValue,
};
use aptos_crypto_derive::CryptoHasher;
use core::fmt;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

pub type BatchDigest = HashValue;

#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Hash, Clone, PartialOrd, Ord)]
pub struct DagBatchId {
    epoch: u64,
    round: Round,
    author: Author,
}

impl DagBatchId {
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn round(&self) -> Round {
        self.round
    }

    pub fn author(&self) -> Author {
        self.author
    }
}

impl fmt::Display for DagBatchId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PayloadId [Epoch: {}, Round: {}, Author: {}]",
            self.epoch, self.round, self.author
        )
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Eq, Hash, Clone, PartialOrd, Ord)]
pub struct DagBatchInfo {
    id: DagBatchId,
    digest: BatchDigest,
    num_txns: usize,
    size_in_bytes: usize,
}

impl DagBatchInfo {
    pub fn new_for_test(
        epoch: u64,
        round: Round,
        author: Author,
        digest: BatchDigest,
        num_txns: usize,
        size_in_bytes: usize,
    ) -> Self {
        Self {
            id: DagBatchId {
                epoch,
                round,
                author,
            },
            digest,
            num_txns,
            size_in_bytes,
        }
    }

    pub fn len(&self) -> usize {
        self.num_txns
    }

    pub fn size(&self) -> usize {
        self.size_in_bytes
    }

    pub fn id(&self) -> &DagBatchId {
        &self.id
    }

    pub fn round(&self) -> Round {
        self.id.round
    }

    pub fn author(&self) -> &Author {
        &self.id.author
    }

    pub fn epoch(&self) -> u64 {
        self.id.epoch
    }

    pub fn digest(&self) -> &BatchDigest {
        &self.digest
    }
}

impl Deref for DagBatchInfo {
    type Target = DagBatchId;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, CryptoHasher)]
pub struct DagBatch {
    info: DagBatchInfo,
    payload: Payload,
}

impl DagBatch {
    pub fn new(epoch: u64, round: Round, author: Author, payload: Payload) -> Self {
        let id = DagBatchId {
            epoch,
            round,
            author,
        };
        let digest = Self::calculate_digest(&id, &payload);
        let info = DagBatchInfo {
            id,
            digest,
            num_txns: payload.len(),
            size_in_bytes: payload.size(),
        };
        Self { info, payload }
    }

    pub fn id(&self) -> &DagBatchId {
        &self.info.id
    }

    pub fn epoch(&self) -> u64 {
        self.info.epoch()
    }

    pub fn info(&self) -> DagBatchInfo {
        self.info.clone()
    }

    pub fn round(&self) -> Round {
        self.info.round()
    }

    pub fn author(&self) -> &Author {
        self.info.author()
    }

    pub fn digest(&self) -> &BatchDigest {
        self.info.digest()
    }

    pub fn payload(&self) -> &Payload {
        &self.payload
    }

    pub fn calculate_digest(id: &DagBatchId, payload: &Payload) -> BatchDigest {
        #[derive(Serialize)]
        struct BatchWithoutDigest<'a> {
            id: &'a DagBatchId,
            payload: &'a Payload,
        }

        impl<'a> CryptoHash for BatchWithoutDigest<'a> {
            type Hasher = DagBatchHasher;

            fn hash(&self) -> HashValue {
                let mut state = Self::Hasher::new();
                let bytes = bcs::to_bytes(&self).expect("Unable to serialize node");
                state.update(&bytes);
                state.finish()
            }
        }

        BatchWithoutDigest { id, payload }.hash()
    }

    pub fn verify(&self) -> anyhow::Result<()> {
        ensure!(self.digest() == &Self::calculate_digest(&self.info.id, &self.payload));

        Ok(())
    }
}

impl Deref for DagBatch {
    type Target = Payload;

    fn deref(&self) -> &Self::Target {
        &self.payload
    }
}
