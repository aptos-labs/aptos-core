// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod sharded_block_partitioner; //TODO: maybe v1 is a better name.
pub mod v2;

pub mod test_utils;

use aptos_types::{
    block_executor::partitioner::{PartitionedTransactions, ShardId},
    transaction::analyzed_transaction::{AnalyzedTransaction, StorageLocation},
};
use move_core_types::account_address::AccountAddress;
use sharded_block_partitioner::config::PartitionerV1Config;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};
use v2::config::PartitionerV2Config;
mod pre_partition;

pub trait BlockPartitioner: Send {
    fn partition(
        &self,
        transactions: Vec<AnalyzedTransaction>,
        num_shards: usize, //TODO: rethink about whether this is needed as part of `BlockPartitioner` API.
    ) -> PartitionedTransactions;
}

/// When multiple transactions access the same storage location,
/// use this function to pick a shard as the anchor/leader and resolve conflicts.
/// Used by `ShardedBlockPartitioner` and `V2Partitioner`.
fn get_anchor_shard_id(storage_location: &StorageLocation, num_shards: usize) -> ShardId {
    let mut hasher = DefaultHasher::new();
    storage_location.hash(&mut hasher);
    (hasher.finish() % num_shards as u64) as usize
}

type Sender = Option<AccountAddress>;

#[derive(Clone, Copy, Debug)]
pub enum PartitionerConfig {
    V1(PartitionerV1Config),
    V2(PartitionerV2Config),
}

impl Default for PartitionerConfig {
    fn default() -> Self {
        PartitionerConfig::V2(PartitionerV2Config::default())
    }
}

impl PartitionerConfig {
    pub fn build(self) -> Box<dyn BlockPartitioner> {
        match self {
            PartitionerConfig::V1(c) => c.build(),
            PartitionerConfig::V2(c) => c.build(),
        }
    }
}
