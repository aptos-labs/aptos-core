// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::proof_of_store::{BatchInfo, BatchInfoExt, ProofOfStore, TBatchInfo};
use anyhow::ensure;
use aptos_types::{transaction::SignedTransaction, PeerId};
use core::fmt;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

pub type OptBatches<T> = BatchPointer<T>;

pub type ProofBatches<T> = BatchPointer<ProofOfStore<T>>;

pub trait TDataInfo {
    fn num_txns(&self) -> u64;

    fn num_bytes(&self) -> u64;

    fn info(&self) -> &BatchInfo;

    fn signers(&self, ordered_authors: &[PeerId]) -> Vec<PeerId>;
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct BatchPointer<T> {
    pub batch_summary: Vec<T>,
}

impl<T> BatchPointer<T>
where
    T: TDataInfo,
{
    pub fn new(metadata: Vec<T>) -> Self {
        Self {
            batch_summary: metadata,
        }
    }

    pub fn extend(&mut self, other: BatchPointer<T>) {
        self.batch_summary.extend(other.batch_summary);
    }

    pub fn num_proofs(&self) -> usize {
        self.batch_summary.len()
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

impl<T> From<Vec<T>> for BatchPointer<T>
where
    T: TDataInfo,
{
    fn from(value: Vec<T>) -> Self {
        Self {
            batch_summary: value,
        }
    }
}

impl<T: PartialEq> PartialEq for BatchPointer<T> {
    fn eq(&self, other: &Self) -> bool {
        self.batch_summary == other.batch_summary
    }
}

impl<T: Eq> Eq for BatchPointer<T> {}

impl<T> Deref for BatchPointer<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.batch_summary
    }
}

impl<T> DerefMut for BatchPointer<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.batch_summary
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
pub struct TxnAndGasLimits {
    pub transaction_limit: Option<u64>,
    pub gas_limit: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum PayloadExecutionLimit {
    None,
    MaxTransactionsToExecute(u64),
    TxnAndGasLimits(TxnAndGasLimits),
}

impl PayloadExecutionLimit {
    pub fn new(max_txns: Option<u64>, _max_gas: Option<u64>) -> Self {
        // TODO: on next release, start using TxnAndGasLimits
        match max_txns {
            Some(max_txns) => PayloadExecutionLimit::MaxTransactionsToExecute(max_txns),
            None => PayloadExecutionLimit::None,
        }
    }

    fn extend_options(o1: Option<u64>, o2: Option<u64>) -> Option<u64> {
        match (o1, o2) {
            (Some(v1), Some(v2)) => Some(v1 + v2),
            (Some(v), None) => Some(v),
            (None, Some(v)) => Some(v),
            _ => None,
        }
    }

    pub(crate) fn extend(&mut self, other: PayloadExecutionLimit) {
        *self = match (&self, &other) {
            (PayloadExecutionLimit::None, _) => other,
            (_, PayloadExecutionLimit::None) => return,
            (
                PayloadExecutionLimit::MaxTransactionsToExecute(limit1),
                PayloadExecutionLimit::MaxTransactionsToExecute(limit2),
            ) => PayloadExecutionLimit::MaxTransactionsToExecute(*limit1 + *limit2),
            (
                PayloadExecutionLimit::TxnAndGasLimits(block1_limits),
                PayloadExecutionLimit::TxnAndGasLimits(block2_limits),
            ) => PayloadExecutionLimit::TxnAndGasLimits(TxnAndGasLimits {
                transaction_limit: Self::extend_options(
                    block1_limits.transaction_limit,
                    block2_limits.transaction_limit,
                ),
                gas_limit: Self::extend_options(block1_limits.gas_limit, block2_limits.gas_limit),
            }),
            (
                PayloadExecutionLimit::MaxTransactionsToExecute(limit1),
                PayloadExecutionLimit::TxnAndGasLimits(block2_limits),
            ) => PayloadExecutionLimit::TxnAndGasLimits(TxnAndGasLimits {
                transaction_limit: Some(*limit1 + block2_limits.transaction_limit.unwrap_or(0)),
                gas_limit: block2_limits.gas_limit,
            }),
            (
                PayloadExecutionLimit::TxnAndGasLimits(block1_limits),
                PayloadExecutionLimit::MaxTransactionsToExecute(limit2),
            ) => PayloadExecutionLimit::TxnAndGasLimits(TxnAndGasLimits {
                transaction_limit: Some(*limit2 + block1_limits.transaction_limit.unwrap_or(0)),
                gas_limit: block1_limits.gas_limit,
            }),
        };
    }

    pub fn max_txns_to_execute(&self) -> Option<u64> {
        match self {
            PayloadExecutionLimit::None => None,
            PayloadExecutionLimit::MaxTransactionsToExecute(max) => Some(*max),
            PayloadExecutionLimit::TxnAndGasLimits(limits) => limits.transaction_limit,
        }
    }

    pub fn block_gas_limit(&self) -> Option<u64> {
        match self {
            PayloadExecutionLimit::None | PayloadExecutionLimit::MaxTransactionsToExecute(_) => {
                None
            },
            PayloadExecutionLimit::TxnAndGasLimits(limits) => limits.gas_limit,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct InlineBatch<T: TBatchInfo> {
    batch_info: T,
    transactions: Vec<SignedTransaction>,
}

impl<T: TBatchInfo> InlineBatch<T> {
    pub fn new(batch_info: T, transactions: Vec<SignedTransaction>) -> Self {
        Self {
            batch_info,
            transactions,
        }
    }

    pub fn info(&self) -> &T {
        &self.batch_info
    }

    pub fn transactions(&self) -> &Vec<SignedTransaction> {
        &self.transactions
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct InlineBatches<T: TBatchInfo>(Vec<InlineBatch<T>>);

impl<T: TBatchInfo> InlineBatches<T> {
    pub fn num_batches(&self) -> usize {
        self.0.len()
    }

    pub fn num_txns(&self) -> usize {
        self.0
            .iter()
            .map(|batch| batch.batch_info.num_txns() as usize)
            .sum()
    }

    pub fn num_bytes(&self) -> usize {
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

    pub fn batch_infos(&self) -> Vec<T> {
        self.0
            .iter()
            .map(|inline_batch| inline_batch.batch_info.clone())
            .collect()
    }
}

impl<T: TBatchInfo> From<Vec<InlineBatch<T>>> for InlineBatches<T> {
    fn from(value: Vec<InlineBatch<T>>) -> Self {
        Self(value)
    }
}

impl<T: TBatchInfo> From<Vec<(T, Vec<SignedTransaction>)>> for InlineBatches<T> {
    fn from(value: Vec<(T, Vec<SignedTransaction>)>) -> Self {
        value
            .into_iter()
            .map(|(batch_info, transactions)| InlineBatch::new(batch_info, transactions))
            .collect::<Vec<_>>()
            .into()
    }
}

impl<T: TBatchInfo> Deref for InlineBatches<T> {
    type Target = Vec<InlineBatch<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: TBatchInfo> DerefMut for InlineBatches<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct OptQuorumStorePayloadV1<T: TBatchInfo> {
    inline_batches: InlineBatches<T>,
    opt_batches: OptBatches<T>,
    proofs: ProofBatches<T>,
    execution_limits: PayloadExecutionLimit,
}

impl<T> OptQuorumStorePayloadV1<T>
where
    T: TBatchInfo + Send + Sync + 'static + TDataInfo,
{
    pub fn get_all_batch_infos(self) -> Vec<BatchInfoExt> {
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
            .map(|info| info.into())
            .collect()
    }

    pub fn max_txns_to_execute(&self) -> Option<u64> {
        self.execution_limits.max_txns_to_execute()
    }

    pub fn block_gas_limit(&self) -> Option<u64> {
        self.execution_limits.block_gas_limit()
    }

    pub fn check_epoch(&self, epoch: u64) -> anyhow::Result<()> {
        ensure!(
            self.inline_batches
                .iter()
                .all(|b| b.info().epoch() == epoch),
            "OptQS InlineBatch epoch doesn't match given epoch"
        );
        ensure!(
            self.opt_batches.iter().all(|b| b.epoch() == epoch),
            "OptQS OptBatch epoch doesn't match given epoch"
        );

        ensure!(
            self.proofs.iter().all(|b| b.epoch() == epoch),
            "OptQS Proof epoch doesn't match given epoch"
        );

        Ok(())
    }

    fn extend(mut self, other: Self) -> Self {
        self.inline_batches.extend(other.inline_batches.0);
        self.opt_batches.extend(other.opt_batches);
        self.proofs.extend(other.proofs);
        self.execution_limits.extend(other.execution_limits);
        self
    }

    pub fn inline_batches(&self) -> &InlineBatches<T> {
        &self.inline_batches
    }

    pub fn proof_with_data(&self) -> &BatchPointer<ProofOfStore<T>> {
        &self.proofs
    }

    pub fn opt_batches(&self) -> &BatchPointer<T> {
        &self.opt_batches
    }

    pub fn set_execution_limit(&mut self, execution_limits: PayloadExecutionLimit) {
        self.execution_limits = execution_limits;
    }

    pub(crate) fn num_txns(&self) -> usize {
        self.opt_batches.num_txns() + self.proofs.num_txns() + self.inline_batches.num_txns()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.opt_batches.is_empty() && self.proofs.is_empty() && self.inline_batches.is_empty()
    }

    pub(crate) fn num_bytes(&self) -> usize {
        self.opt_batches.num_bytes() + self.proofs.num_bytes() + self.inline_batches.num_bytes()
    }
}

impl From<OptQuorumStorePayloadV1<BatchInfo>> for OptQuorumStorePayloadV1<BatchInfoExt> {
    fn from(p: OptQuorumStorePayloadV1<BatchInfo>) -> Self {
        OptQuorumStorePayloadV1 {
            inline_batches: p
                .inline_batches
                .0
                .into_iter()
                .map(|batch| InlineBatch::new(batch.batch_info.into(), batch.transactions))
                .collect::<Vec<_>>()
                .into(),
            opt_batches: p
                .opt_batches
                .into_iter()
                .map(|batch| batch.into())
                .collect::<Vec<_>>()
                .into(),
            proofs: p
                .proofs
                .into_iter()
                .map(|proof| proof.into())
                .collect::<Vec<_>>()
                .into(),
            execution_limits: p.execution_limits,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum OptQuorumStorePayload {
    V1(OptQuorumStorePayloadV1<BatchInfo>),
    V2(OptQuorumStorePayloadV1<BatchInfoExt>),
}

impl OptQuorumStorePayload {
    pub fn new(
        inline_batches: InlineBatches<BatchInfo>,
        opt_batches: OptBatches<BatchInfo>,
        proofs: ProofBatches<BatchInfo>,
        execution_limits: PayloadExecutionLimit,
    ) -> Self {
        Self::V1(OptQuorumStorePayloadV1 {
            inline_batches,
            opt_batches,
            proofs,
            execution_limits,
        })
    }

    pub fn new_v2(
        inline_batches: InlineBatches<BatchInfoExt>,
        opt_batches: OptBatches<BatchInfoExt>,
        proofs: ProofBatches<BatchInfoExt>,
        execution_limits: PayloadExecutionLimit,
    ) -> Self {
        Self::V2(OptQuorumStorePayloadV1 {
            inline_batches,
            opt_batches,
            proofs,
            execution_limits,
        })
    }

    pub fn empty() -> Self {
        Self::new(
            Vec::<(BatchInfo, Vec<SignedTransaction>)>::new().into(),
            Vec::<BatchInfo>::new().into(),
            Vec::<ProofOfStore<BatchInfo>>::new().into(),
            PayloadExecutionLimit::None,
        )
    }

    pub(crate) fn extend(self, other: Self) -> Self {
        match (self, other) {
            (Self::V1(p1), Self::V1(p2)) => Self::V1(p1.extend(p2)),
            (Self::V2(p1), Self::V2(p2)) => Self::V2(p1.extend(p2)),
            (Self::V1(p1), Self::V2(p2)) => {
                Self::V2(OptQuorumStorePayloadV1::<BatchInfoExt>::from(p1).extend(p2))
            },
            (Self::V2(p1), Self::V1(p2)) => Self::V2(p1.extend(p2.into())),
        }
    }

    pub fn set_execution_limit(&mut self, execution_limits: PayloadExecutionLimit) {
        match self {
            OptQuorumStorePayload::V1(p) => p.set_execution_limit(execution_limits),
            OptQuorumStorePayload::V2(p) => p.set_execution_limit(execution_limits),
        }
    }

    pub(crate) fn num_txns(&self) -> usize {
        match self {
            OptQuorumStorePayload::V1(p) => p.num_txns(),
            OptQuorumStorePayload::V2(p) => p.num_txns(),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        match self {
            OptQuorumStorePayload::V1(p) => p.is_empty(),
            OptQuorumStorePayload::V2(p) => p.is_empty(),
        }
    }

    pub(crate) fn num_bytes(&self) -> usize {
        match self {
            OptQuorumStorePayload::V1(p) => p.num_bytes(),
            OptQuorumStorePayload::V2(p) => p.num_bytes(),
        }
    }

    pub(crate) fn max_txns_to_execute(&self) -> Option<u64> {
        match self {
            OptQuorumStorePayload::V1(p) => p.max_txns_to_execute(),
            OptQuorumStorePayload::V2(p) => p.max_txns_to_execute(),
        }
    }

    pub(crate) fn check_epoch(&self, epoch: u64) -> anyhow::Result<()> {
        match self {
            OptQuorumStorePayload::V1(p) => p.check_epoch(epoch),
            OptQuorumStorePayload::V2(p) => p.check_epoch(epoch),
        }
    }

    fn num_inline_txns(&self) -> usize {
        match self {
            OptQuorumStorePayload::V1(p) => p.inline_batches().num_txns(),
            OptQuorumStorePayload::V2(p) => p.inline_batches().num_txns(),
        }
    }

    fn num_opt_batch_txns(&self) -> usize {
        match self {
            OptQuorumStorePayload::V1(p) => p.opt_batches().num_txns(),
            OptQuorumStorePayload::V2(p) => p.opt_batches().num_txns(),
        }
    }

    fn num_proof_txns(&self) -> usize {
        match self {
            OptQuorumStorePayload::V1(p) => p.proof_with_data().num_txns(),
            OptQuorumStorePayload::V2(p) => p.proof_with_data().num_txns(),
        }
    }

    fn execution_limits(&self) -> &PayloadExecutionLimit {
        match self {
            OptQuorumStorePayload::V1(p) => &p.execution_limits,
            OptQuorumStorePayload::V2(p) => &p.execution_limits,
        }
    }
}

impl fmt::Display for OptQuorumStorePayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "OptQuorumStorePayload(inline: {}, opt: {}, proofs: {}, limits: {:?})",
            self.num_inline_txns(),
            self.num_opt_batch_txns(),
            self.num_proof_txns(),
            self.execution_limits(),
        )
    }
}
