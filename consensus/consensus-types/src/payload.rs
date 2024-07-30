// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::DataStatus,
    proof_of_store::{BatchInfo, ProofOfStore},
};
use aptos_infallible::Mutex;
use aptos_types::{transaction::SignedTransaction, PeerId};
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

    fn signers(&self, ordered_authors: &[PeerId]) -> Vec<PeerId>;
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct BatchPointer<T> {
    pub batch_summary: Vec<T>,
    #[serde(skip)]
    pub status: Arc<Mutex<Option<DataStatus>>>,
}

impl<T> BatchPointer<T>
where
    T: TDataInfo,
{
    pub fn new(metadata: Vec<T>) -> Self {
        Self {
            batch_summary: metadata,
            status: Arc::new(Mutex::new(None)),
        }
    }

    pub fn extend(&mut self, other: BatchPointer<T>) {
        let other_data_status = other.status.lock().as_mut().unwrap().take();
        self.batch_summary.extend(other.batch_summary);
        let mut status = self.status.lock();
        *status = match &mut *status {
            None => Some(other_data_status),
            Some(status) => {
                status.extend(other_data_status);
                return;
            },
        };
    }

    pub fn num_txns(&self) -> usize {
        self.batch_summary
            .iter()
            .map(|info| info.num_txns() as usize)
            .sum()
    }

    pub fn num_bytes(&self) -> usize {
        self.batch_summary
            .iter()
            .map(|info| info.num_bytes() as usize)
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.batch_summary.is_empty()
    }
}

impl<T: PartialEq> PartialEq for BatchPointer<T> {
    fn eq(&self, other: &Self) -> bool {
        self.batch_summary == other.batch_summary
            && Arc::as_ptr(&self.status) == Arc::as_ptr(&other.status)
    }
}

impl<T: Eq> Eq for BatchPointer<T> {}

impl<T> Deref for BatchPointer<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.batch_summary
    }
}

impl<T> IntoIterator for BatchPointer<T> {
    type IntoIter = std::vec::IntoIter<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        self.batch_summary.into_iter()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum PayloadExecutionLimit {
    None,
    MaxTransactionsToExecute(u64),
}

impl PayloadExecutionLimit {
    pub(crate) fn extend(&mut self, other: PayloadExecutionLimit) {
        *self = match (&self, &other) {
            (PayloadExecutionLimit::None, _) => other,
            (_, PayloadExecutionLimit::None) => return,
            (
                PayloadExecutionLimit::MaxTransactionsToExecute(limit1),
                PayloadExecutionLimit::MaxTransactionsToExecute(limit2),
            ) => PayloadExecutionLimit::MaxTransactionsToExecute(*limit1 + *limit2),
        };
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct InlineBatch {
    batch_info: BatchInfo,
    transactions: Vec<SignedTransaction>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct InlineBatches(Vec<InlineBatch>);

impl InlineBatches {
    fn num_txns(&self) -> usize {
        self.0
            .iter()
            .map(|batch| batch.batch_info.num_txns() as usize)
            .sum()
    }

    fn num_bytes(&self) -> usize {
        self.0
            .iter()
            .map(|batch| batch.batch_info.num_bytes() as usize)
            .sum()
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn transactions(&self) -> Vec<SignedTransaction> {
        self.0
            .iter()
            .flat_map(|inline_batch| inline_batch.transactions.clone())
            .collect()
    }

    pub fn batch_infos(&self) -> Vec<BatchInfo> {
        self.0
            .iter()
            .map(|inline_batch| inline_batch.batch_info.clone())
            .collect()
    }
}

impl Deref for InlineBatches {
    type Target = Vec<InlineBatch>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for InlineBatches {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct OptQuorumStorePayloadV1 {
    inline_batches: InlineBatches,
    opt_batches: BatchPointer<BatchInfo>,
    proofs: BatchPointer<ProofOfStore>,
    execution_limits: PayloadExecutionLimit,
}

impl OptQuorumStorePayloadV1 {
    pub fn get_all_batch_infos(self) -> Vec<BatchInfo> {
        let Self {
            inline_batches,
            opt_batches,
            proofs,
            execution_limits: _,
        } = self;
        inline_batches
            .0
            .into_iter()
            .map(|batch| batch.batch_info)
            .chain(opt_batches)
            .chain(proofs.into_iter().map(|proof| proof.info().clone()))
            .collect()
    }

    pub fn max_txns_to_execute(&self) -> Option<u64> {
        match self.execution_limits {
            PayloadExecutionLimit::None => None,
            PayloadExecutionLimit::MaxTransactionsToExecute(max) => Some(max),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum OptQuorumStorePayload {
    V1(OptQuorumStorePayloadV1),
}

impl OptQuorumStorePayload {
    pub(crate) fn num_txns(&self) -> usize {
        self.opt_batches.num_txns() + self.proofs.num_txns() + self.inline_batches.num_txns()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.opt_batches.is_empty() && self.proofs.is_empty() && self.inline_batches.is_empty()
    }

    pub(crate) fn extend(mut self, other: Self) -> Self {
        let other: OptQuorumStorePayloadV1 = other.into_inner();
        self.inline_batches.extend(other.inline_batches.0);
        self.opt_batches.extend(other.opt_batches);
        self.proofs.extend(other.proofs);
        self.execution_limits.extend(other.execution_limits);
        self
    }

    pub(crate) fn num_bytes(&self) -> usize {
        self.opt_batches.num_bytes() + self.proofs.num_bytes() + self.inline_batches.num_bytes()
    }

    pub fn into_inner(self) -> OptQuorumStorePayloadV1 {
        match self {
            OptQuorumStorePayload::V1(opt_qs_payload) => opt_qs_payload,
        }
    }

    pub fn inline_batches(&self) -> &InlineBatches {
        &self.inline_batches
    }

    pub fn proof_with_data(&self) -> &BatchPointer<ProofOfStore> {
        &self.proofs
    }

    pub fn opt_batches(&self) -> &BatchPointer<BatchInfo> {
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
