// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::proof_of_store::{BatchInfo, ProofOfStore};
use anyhow::ensure;
use aptos_executor_types::ExecutorResult;
use aptos_infallible::Mutex;
use aptos_types::{transaction::SignedTransaction, PeerId};
use core::fmt;
use futures::{
    future::{BoxFuture, Shared},
    FutureExt,
};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    future::Future,
    ops::{Deref, DerefMut, Range},
    pin::Pin,
    sync::{Arc, OnceLock},
};

pub type OptBatches = BatchPointer<BatchInfo>;

pub type ProofBatches = BatchPointer<ProofOfStore>;

pub trait TDataInfo {
    fn num_txns(&self) -> u64;

    fn num_bytes(&self) -> u64;

    fn info(&self) -> &BatchInfo;

    fn signers(&self, ordered_authors: &[PeerId]) -> Vec<PeerId>;
}

pub type DataFetchFut =
    Shared<Pin<Box<dyn Future<Output = ExecutorResult<Vec<SignedTransaction>>> + Send>>>;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct BatchPointer<T> {
    pub batch_summary: Vec<T>,
    #[serde(skip)]
    pub data_fut: Arc<Mutex<Option<DataFetchFut>>>,
}

impl<T> BatchPointer<T>
where
    T: TDataInfo,
{
    pub fn new(metadata: Vec<T>) -> Self {
        Self {
            batch_summary: metadata,
            data_fut: Arc::new(Mutex::new(None)),
        }
    }

    pub fn empty() -> Self {
        Self {
            batch_summary: Vec::new(),
            data_fut: Arc::new(Mutex::new(None)),
        }
    }

    pub fn extend(&mut self, other: BatchPointer<T>) {
        self.batch_summary.extend(other.batch_summary);
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

impl Default for BatchPointer<BatchInfo> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T> From<Vec<T>> for BatchPointer<T>
where
    T: TDataInfo,
{
    fn from(value: Vec<T>) -> Self {
        Self {
            batch_summary: value,
            data_fut: Arc::new(Mutex::new(None)),
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

    pub(crate) fn max_txns_to_execute(limit: Option<u64>) -> Self {
        limit.map_or(PayloadExecutionLimit::None, |val| {
            PayloadExecutionLimit::MaxTransactionsToExecute(val)
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct InlineBatch {
    batch_info: BatchInfo,
    transactions: Vec<SignedTransaction>,
}

impl InlineBatch {
    pub fn new(batch_info: BatchInfo, transactions: Vec<SignedTransaction>) -> Self {
        Self {
            batch_info,
            transactions,
        }
    }

    pub fn info(&self) -> &BatchInfo {
        &self.batch_info
    }

    pub fn transactions(&self) -> &Vec<SignedTransaction> {
        &self.transactions
    }
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

impl From<Vec<InlineBatch>> for InlineBatches {
    fn from(value: Vec<InlineBatch>) -> Self {
        Self(value)
    }
}

impl From<Vec<(BatchInfo, Vec<SignedTransaction>)>> for InlineBatches {
    fn from(value: Vec<(BatchInfo, Vec<SignedTransaction>)>) -> Self {
        value
            .into_iter()
            .map(|(batch_info, transactions)| InlineBatch::new(batch_info, transactions))
            .collect::<Vec<_>>()
            .into()
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
    opt_batches: OptBatches,
    proofs: ProofBatches,
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

    pub fn check_epoch(&self, epoch: u64) -> anyhow::Result<()> {
        ensure!(
            self.inline_batches
                .iter()
                .all(|b| b.info().epoch() == epoch),
            "OptQS InlineBatch epoch doesn't match given epoch"
        );
        ensure!(
            self.opt_batches.iter().all(|b| b.info().epoch() == epoch),
            "OptQS OptBatch epoch doesn't match given epoch"
        );

        ensure!(
            self.proofs.iter().all(|b| b.info().epoch() == epoch),
            "OptQS Proof epoch doesn't match given epoch"
        );

        Ok(())
    }
}

pub const N_SUB_BLOCKS: usize = 8;

pub type SubBlocks = [BatchPointer<BatchInfo>; N_SUB_BLOCKS];

pub type Prefix = usize;

pub type PrefixSet = CompressedPrefixSet;

#[derive(Clone, Hash, Serialize, Deserialize)]
pub struct CompressedPrefixSet {
    #[serde(with = "serde_bytes")]
    repr: Vec<u8>,
}

impl CompressedPrefixSet {
    pub fn new(votes: impl IntoIterator<Item = (usize, usize)>) -> Self {
        let mut repr = vec![];

        for (node_id, prefix) in votes {
            assert!(prefix < 0xF);

            let byte_id = node_id / 2;

            while repr.len() < byte_id + 1 {
                // Lazy initialization with 0xFF so that each half-byte is set to 0xF.
                // 0xF is used as the special value indicating that the node is not a signer.
                repr.push(0xFF);
            }

            if node_id % 2 == 0 {
                // Least significant 4 bits are used for even node IDs.
                // Keep most significant 4 bits, set least significant 4 bits to `prefix`.
                repr[byte_id] = repr[byte_id] & 0xF0 | prefix as u8;
            } else {
                // Most significant 4 bits are used for odd node IDs.
                // Keep least significant 4 bits, set most significant 4 bits to `prefix`.
                repr[byte_id] = repr[byte_id] & 0x0F | ((prefix as u8) << 4);
            }
        }

        CompressedPrefixSet { repr }
    }

    pub fn empty() -> Self {
        Self { repr: vec![] }
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, Prefix)> + '_ {
        self.repr
            .iter()
            .copied()
            .enumerate()
            .flat_map(|(byte_id, byte)| {
                [
                    // Least significant 4 bits are used for even node IDs.
                    (byte_id * 2, (byte & 0x0F) as Prefix),
                    // Most significant 4 bits are used for odd node IDs.
                    (byte_id * 2 + 1, (byte >> 4) as Prefix),
                ]
            })
            // 0xF is used as the special value indicating that the node is not a signer.
            .filter(|(_, prefix)| *prefix != 0xF)
    }

    pub fn is_empty(&self) -> bool {
        // NB: this works only because PrefixSet is immutable once created.
        self.repr.is_empty()
    }

    pub fn signers(&self) -> impl Iterator<Item = usize> + '_ {
        self.iter().map(|(id, _)| id)
    }

    pub fn sub_block_signers(&self, sub_block: usize) -> impl Iterator<Item = usize> + '_ {
        self.iter()
            .filter(move |(_, prefix)| *prefix >= sub_block)
            .map(|(id, _)| id)
    }

    pub fn prefix(&self, storage_requirement: usize) -> Prefix {
        assert!(storage_requirement > 0, "storage_requirement cannot be 0");
        let mut counts = [0; 16];

        // Count the occurrences of each prefix.
        for (_, prefix) in self.iter() {
            counts[prefix] += 1;
        }

        // Iterate from the highest prefix downward, accumulating counts.
        let mut total = 0;
        for prefix in (0..=15).rev() {
            total += counts[prefix];
            if total >= storage_requirement {
                return prefix;
            }
        }

        panic!("Not enough votes in a PrefixSet. storage_requirement cannot exceed quorum size.");
    }

    pub fn node_ids(&self) -> impl Iterator<Item = usize> + '_ {
        self.iter().map(|(id, _)| id)
    }

    pub fn prefixes(&self) -> impl Iterator<Item = Prefix> + '_ {
        self.iter().map(|(_, prefix)| prefix)
    }

    /// Returns the k-th maximum prefix, where k is 1-indexed.
    /// For example, k=1 returns the maximum prefix, k=2 returns the second maximum, etc.
    /// Panics if k=0 is supplied.
    pub fn kth_max_prefix(&self, k: usize) -> Option<Prefix> {
        assert!(k > 0, "k must be greater than 0");

        let mut prefixes: Vec<_> = self.iter().map(|(_, prefix)| prefix).collect();
        prefixes.sort_by_key(|&prefix| std::cmp::Reverse(prefix));

        prefixes.get(k - 1).copied()
    }

    pub fn unzip(&self) -> (Vec<usize>, Vec<Prefix>) {
        self.iter().unzip()
    }
}

impl FromIterator<(usize, usize)> for CompressedPrefixSet {
    fn from_iter<T: IntoIterator<Item = (usize, usize)>>(iter: T) -> Self {
        Self::new(iter)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RaptrPayload {
    data: Arc<RaptrPayloadData>,
    include_proofs: bool,
    sub_blocks: Range<usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
struct RaptrPayloadData {
    sub_blocks: SubBlocks,
    proofs: ProofBatches,
}

impl RaptrPayloadData {
    fn new(sub_blocks: SubBlocks, proofs: ProofBatches) -> Self {
        Self { sub_blocks, proofs }
    }

    fn new_empty() -> Self {
        Self {
            sub_blocks: Default::default(),
            proofs: Vec::new().into(),
        }
    }
}

impl RaptrPayload {
    pub fn new(proofs: ProofBatches, sub_blocks: SubBlocks) -> Self {
        let sub_blocks_range = 0..sub_blocks.len();
        Self {
            data: Arc::new(RaptrPayloadData::new(sub_blocks, proofs)),
            include_proofs: true,
            sub_blocks: sub_blocks_range,
        }
    }

    pub fn new_empty() -> Self {
        RaptrPayload {
            data: Arc::new(RaptrPayloadData::new_empty()),
            include_proofs: true,
            sub_blocks: 0..N_SUB_BLOCKS,
        }
    }

    pub fn sub_blocks(&self) -> &[BatchPointer<BatchInfo>] {
        &self.data.sub_blocks[self.sub_blocks.clone()]
    }

    pub fn numbered_sub_blocks(
        &self,
    ) -> impl ExactSizeIterator<Item = (usize, &BatchPointer<BatchInfo>)> {
        self.data
            .sub_blocks
            .iter()
            .enumerate()
            .skip(self.sub_blocks.start)
            .take(self.sub_blocks.len())
    }

    pub fn proofs(&self) -> &ProofBatches {
        if self.include_proofs {
            &self.data.proofs
        } else {
            static EMPTY_BATCH_POINTER: OnceLock<ProofBatches> = OnceLock::new();
            &EMPTY_BATCH_POINTER.get_or_init(|| BatchPointer::new(vec![]))
        }
    }

    pub fn with_prefix(&self, prefix: usize) -> Self {
        assert!(prefix <= self.data.sub_blocks.len());
        assert!(self.include_proofs);

        Self {
            data: self.data.clone(),
            include_proofs: true,
            sub_blocks: 0..prefix,
        }
    }

    pub fn take_sub_blocks(&self, range: Range<usize>) -> Self {
        assert!(range.end <= self.data.sub_blocks.len());

        Self {
            data: self.data.clone(),
            include_proofs: false,
            sub_blocks: range,
        }
    }

    pub fn num_sub_block_txns(&self) -> usize {
        self.sub_blocks()
            .iter()
            .map(|inner| inner.num_txns())
            .sum::<usize>()
    }

    pub fn num_txns(&self) -> usize {
        self.num_sub_block_txns() + self.proofs().num_txns()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.sub_blocks().iter().all(|inner| inner.is_empty()) && self.proofs().is_empty()
    }

    pub fn num_sub_block_batches(&self) -> usize {
        self.sub_blocks().iter().map(|inner| inner.len()).sum()
    }

    pub fn num_proof_batches(&self) -> usize {
        self.proofs().len()
    }

    pub fn num_batches(&self) -> usize {
        self.num_sub_block_batches() + self.num_proof_batches()
    }

    pub(crate) fn num_bytes(&self) -> usize {
        self.sub_blocks()
            .iter()
            .map(|inner| inner.num_bytes())
            .sum::<usize>()
            + self.proofs().num_bytes()
    }

    pub fn proof_with_data(&self) -> &BatchPointer<ProofOfStore> {
        &self.proofs()
    }

    pub fn all_sub_block_batches(&self) -> Vec<BatchInfo> {
        self.sub_blocks()
            .iter()
            .flat_map(|inner| inner.deref())
            .cloned()
            .collect()
    }

    pub fn get_all_batch_infos(&self) -> impl Iterator<Item = &BatchInfo> {
        self.proofs()
            .iter()
            .map(|p| p.info())
            .chain(self.sub_blocks().iter().flat_map(|sb| &sb.batch_summary))
    }
}

impl fmt::Display for RaptrPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RaikouPayload(sub_blocks: {}, proofs: {})",
            self.num_sub_block_txns(),
            self.proofs().num_txns(),
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum OptQuorumStorePayload {
    V1(OptQuorumStorePayloadV1),
}

impl OptQuorumStorePayload {
    pub fn new(
        inline_batches: InlineBatches,
        opt_batches: OptBatches,
        proofs: ProofBatches,
        execution_limits: PayloadExecutionLimit,
    ) -> Self {
        Self::V1(OptQuorumStorePayloadV1 {
            inline_batches,
            opt_batches,
            proofs,
            execution_limits,
        })
    }

    pub fn new_empty() -> Self {
        Self::V1(OptQuorumStorePayloadV1 {
            inline_batches: Vec::<InlineBatch>::new().into(),
            opt_batches: Vec::new().into(),
            proofs: Vec::new().into(),
            execution_limits: PayloadExecutionLimit::None,
        })
    }

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

    pub(crate) fn num_opt_batches(&self) -> usize {
        self.opt_batches.len()
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

    pub fn set_execution_limit(&mut self, execution_limits: PayloadExecutionLimit) {
        self.execution_limits = execution_limits;
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

// Write tests for PrefixSet
#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;

    #[test]
    fn test_prefix_set() {
        let votes = vec![(0, 3), (2, 14), (5, 0)];
        let prefix_set = PrefixSet::new(votes);
        let mut iter = prefix_set.iter();
        assert_eq!(iter.next(), Some((0, 3)));
        assert_eq!(iter.next(), Some((2, 14)));
        assert_eq!(iter.next(), Some((5, 0)));
        assert_eq!(iter.next(), None);
    }

    #[test]
    #[should_panic]
    fn test_compressed_prefix_set_prefix_too_large_1() {
        let votes = vec![(0, 3), (2, 14), (5, 15)];
        let _prefix_set = CompressedPrefixSet::new(votes);
    }

    #[test]
    #[should_panic]
    fn test_compressed_prefix_set_prefix_too_large_2() {
        let votes = vec![(0, 3), (2, 14), (5, 24)];
        let _prefix_set = CompressedPrefixSet::new(votes);
    }

    #[test]
    fn test_sub_block_signers() {
        let votes = vec![(0, 3), (2, 13), (5, 0)];
        let prefix_set = PrefixSet::new(votes);
        assert_eq!(prefix_set.sub_block_signers(3).collect_vec(), vec![0, 2]);
        assert_eq!(prefix_set.sub_block_signers(10).collect_vec(), vec![2]);
        assert_eq!(prefix_set.sub_block_signers(13).collect_vec(), vec![2]);
        assert_eq!(prefix_set.sub_block_signers(14).collect_vec(), vec![]
            as Vec<usize>);
        assert_eq!(prefix_set.sub_block_signers(0).collect_vec(), vec![0, 2, 5]);
    }

    #[test]
    fn test_prefix() {
        let votes = vec![(0, 3), (2, 14), (5, 0)];
        let prefix_set = PrefixSet::new(votes);
        assert_eq!(prefix_set.prefix(1), 14);
        assert_eq!(prefix_set.prefix(2), 3);
        assert_eq!(prefix_set.prefix(3), 0);
    }

    #[test]
    #[should_panic]
    fn test_prefix_panic() {
        let votes = vec![(0, 3), (2, 14), (5, 0)];
        let prefix_set = PrefixSet::new(votes);
        prefix_set.prefix(4);
    }
}
