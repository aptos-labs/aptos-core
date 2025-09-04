// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_types::block_executor::partitioner::{
    RoundId, ShardId, GLOBAL_ROUND_ID, GLOBAL_SHARD_ID,
};
use serde::{Deserialize, Serialize};
use std::cmp;

/// Represent which sub-block a txn is assigned to.
/// TODO: switch to enum to better represent the sub-block assigned to the global executor.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SubBlockIdx {
    pub round_id: RoundId,
    pub shard_id: ShardId,
}

impl Ord for SubBlockIdx {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        (self.round_id, self.shard_id).cmp(&(other.round_id, other.shard_id))
    }
}

impl PartialOrd for SubBlockIdx {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl SubBlockIdx {
    pub fn new(round_id: RoundId, shard_id: ShardId) -> Self {
        SubBlockIdx { round_id, shard_id }
    }

    pub fn global() -> SubBlockIdx {
        SubBlockIdx::new(GLOBAL_ROUND_ID, GLOBAL_SHARD_ID)
    }
}

/// The txn positions in the original block.
pub type OriginalTxnIdx = usize;

/// The txn positions after pre-partitioning but before discarding.
pub type PrePartitionedTxnIdx = usize;

/// The txn positions after discarding.
pub type FinalTxnIdx = usize;

/// Represent a specific storage location in a partitioning session.
/// TODO: ensure this type can support max num of unique state keys in a block.
pub type StorageKeyIdx = usize;

/// Represent a sender in a partitioning session.
pub type SenderIdx = usize;

/// Represents positions of a txn after it is assigned to a sub-block.
///
/// Different from `velor_types::block_executor::partitioner::ShardedTxnIndex`,
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ShardedTxnIndexV2 {
    pub sub_block_idx: SubBlockIdx,
    pub pre_partitioned_txn_idx: PrePartitionedTxnIdx,
}

impl Ord for ShardedTxnIndexV2 {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        (self.sub_block_idx, self.pre_partitioned_txn_idx)
            .cmp(&(other.sub_block_idx, other.pre_partitioned_txn_idx))
    }
}

impl PartialOrd for ShardedTxnIndexV2 {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl ShardedTxnIndexV2 {
    pub fn round_id(&self) -> RoundId {
        self.sub_block_idx.round_id
    }

    pub fn shard_id(&self) -> ShardId {
        self.sub_block_idx.shard_id
    }
}

impl ShardedTxnIndexV2 {
    pub fn new(round_id: RoundId, shard_id: ShardId, txn_idx1: PrePartitionedTxnIdx) -> Self {
        Self {
            sub_block_idx: SubBlockIdx::new(round_id, shard_id),
            pre_partitioned_txn_idx: txn_idx1,
        }
    }
}
