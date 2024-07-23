use crate::{
    common::{DataStatus, PayloadExecutionLimit},
    proof_of_store::{BatchInfo, ProofOfStore},
};
use aptos_infallible::Mutex;
use aptos_types::PeerId;
use core::fmt;
use serde::{Deserialize, Serialize};
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

pub trait TDataInfo {
    fn num_txns(&self) -> u64;

    fn num_bytes(&self) -> u64;

    fn info(&self) -> &BatchInfo;

    fn signers(&self, ordered_authors: &Vec<PeerId>) -> Vec<PeerId>;
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CachedDataPointer<T> {
    pub pointer: Vec<T>,
    #[serde(skip)]
    pub status: Arc<Mutex<Option<DataStatus>>>,
}

impl<T> CachedDataPointer<T>
where
    T: TDataInfo,
{
    pub fn new(metadata: Vec<T>) -> Self {
        Self {
            pointer: metadata,
            status: Arc::new(Mutex::new(None)),
        }
    }

    pub fn extend(&mut self, other: CachedDataPointer<T>) {
        let other_data_status = other.status.lock().as_mut().unwrap().take();
        self.pointer.extend(other.pointer);
        let mut status = self.status.lock();
        if status.is_none() {
            *status = Some(other_data_status);
        } else {
            status.as_mut().unwrap().extend(other_data_status);
        }
    }

    pub fn num_txns(&self) -> usize {
        self.pointer
            .iter()
            .map(|info| info.num_txns() as usize)
            .sum()
    }

    pub fn num_bytes(&self) -> usize {
        self.pointer
            .iter()
            .map(|info| info.num_bytes() as usize)
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.pointer.is_empty()
    }
}

impl<T: PartialEq> PartialEq for CachedDataPointer<T> {
    fn eq(&self, other: &Self) -> bool {
        self.pointer == other.pointer && Arc::as_ptr(&self.status) == Arc::as_ptr(&other.status)
    }
}

impl<T: Eq> Eq for CachedDataPointer<T> {}

impl<T> Deref for CachedDataPointer<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.pointer
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct OptQuorumStorePayloadV1 {
    opt_batches: CachedDataPointer<BatchInfo>,
    proofs: CachedDataPointer<ProofOfStore>,
    execution_limits: PayloadExecutionLimit,
}

impl OptQuorumStorePayloadV1 {
    pub fn get_all_batch_infos(&self) -> Vec<BatchInfo> {
        self.opt_batches
            .deref()
            .iter()
            .chain(self.proofs.iter().map(|proof| proof.info()))
            .cloned()
            .collect()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum OptQuorumStorePayload {
    V1(OptQuorumStorePayloadV1),
}

impl OptQuorumStorePayload {
    pub(crate) fn num_txns(&self) -> usize {
        self.opt_batches.num_txns() + self.proofs.num_txns()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.opt_batches.is_empty() && self.proofs.is_empty()
    }

    pub(crate) fn extend(mut self, other: Self) -> Self {
        let other: OptQuorumStorePayloadV1 = other.into_inner();
        self.opt_batches.extend(other.opt_batches);
        self.proofs.extend(other.proofs);
        self.execution_limits.extend(other.execution_limits);
        self
    }

    pub(crate) fn num_bytes(&self) -> usize {
        self.opt_batches.num_bytes() + self.proofs.num_bytes()
    }

    fn into_inner(self) -> OptQuorumStorePayloadV1 {
        match self {
            OptQuorumStorePayload::V1(opt_qs_payload) => opt_qs_payload,
        }
    }

    pub fn proof_with_data(&self) -> &CachedDataPointer<ProofOfStore> {
        &self.proofs
    }

    pub fn opt_batches(&self) -> &CachedDataPointer<BatchInfo> {
        &self.opt_batches
    }
}

impl Deref for OptQuorumStorePayload {
    type Target = OptQuorumStorePayloadV1;

    fn deref(&self) -> &Self::Target {
        match self {
            OptQuorumStorePayload::V1(opt_qs_payload) => opt_qs_payload,
        }
    }
}

impl DerefMut for OptQuorumStorePayload {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            OptQuorumStorePayload::V1(opt_qs_payload) => opt_qs_payload,
        }
    }
}

impl fmt::Display for OptQuorumStorePayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "OptQuorumStorePayload(opt_batches: {}, proofs: {}, limits: {:?})",
            self.opt_batches.num_txns(),
            self.proofs.num_txns(),
            self.execution_limits,
        )
    }
}
