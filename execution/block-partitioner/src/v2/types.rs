// Copyright © Aptos Foundation

use aptos_types::block_executor::partitioner::{
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
        (self.round_id, self.shard_id).partial_cmp(&(other.round_id, other.shard_id))
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

/// The position of a txn in the block after `pre_partition` and before `partition_to_matrix`.
pub type PrePartitionedTxnIdx = usize;

/// Represent a specific storage location in a partitioning session.
/// TODO: ensure this type can support max num of unique state keys in a block.
pub type StorageKeyIdx = usize;

/// Represent a sender in a partitioning session.
pub type SenderIdx = usize;

/// Represents positions of a txn after it is assigned to a sub-block.
///
/// Different from `aptos_types::block_executor::partitioner::ShardedTxnIndex`,
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ShardedTxnIndexV2 {
    pub sub_block_idx: SubBlockIdx,
    pub ori_txn_idx: PrePartitionedTxnIdx,
}

impl Ord for ShardedTxnIndexV2 {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        (self.sub_block_idx, self.ori_txn_idx).cmp(&(other.sub_block_idx, other.ori_txn_idx))
    }
}

impl PartialOrd for ShardedTxnIndexV2 {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        (self.sub_block_idx, self.ori_txn_idx)
            .partial_cmp(&(other.sub_block_idx, other.ori_txn_idx))
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
    pub fn new(round_id: RoundId, shard_id: ShardId, ori_txn_idx: PrePartitionedTxnIdx) -> Self {
        Self {
            sub_block_idx: SubBlockIdx::new(round_id, shard_id),
            ori_txn_idx,
        }
    }
}
