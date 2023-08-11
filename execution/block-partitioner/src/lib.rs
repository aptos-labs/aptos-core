// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod sharded_block_partitioner;
pub mod v2;

pub mod test_utils;

#[cfg(test)]
use crate::test_utils::P2PBlockGenerator;
use crate::{sharded_block_partitioner::ShardedBlockPartitioner, v2::PartitionerV2};
use aptos_crypto::{
    hash::{CryptoHash, TestOnlyHash},
    HashValue,
};
use aptos_logger::info;
use aptos_types::{
    block_executor::partitioner::{
        PartitionedTransactions, RoundId, ShardId, SubBlocksForShard, TransactionWithDependencies,
        GLOBAL_ROUND_ID, GLOBAL_SHARD_ID,
    },
    state_store::state_key::StateKey,
    transaction::analyzed_transaction::{AnalyzedTransaction, StorageLocation},
};
use move_core_types::account_address::AccountAddress;
use once_cell::sync::Lazy;
#[cfg(test)]
use rand::thread_rng;
#[cfg(test)]
use std::sync::Arc;
use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    hash::{Hash, Hasher},
    sync::RwLock,
};
use v2::config::PartitionerV2Config;
mod pre_partition;

pub trait BlockPartitioner: Send {
    fn partition(
        &self,
        transactions: Vec<AnalyzedTransaction>,
        num_shards: usize,
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
            PartitionerConfig::V1(c) => Box::new(c.build()),
            PartitionerConfig::V2(c) => Box::new(c.build()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PartitionerV1Config {
    pub num_shards: usize,
    pub max_partitioning_rounds: RoundId,
    pub cross_shard_dep_avoid_threshold: f32,
    pub partition_last_round: bool,
}

impl PartitionerV1Config {
    pub fn new() -> Self {
        PartitionerV1Config {
            num_shards: 0,
            max_partitioning_rounds: 3,
            cross_shard_dep_avoid_threshold: 0.9,
            partition_last_round: false,
        }
    }

    pub fn num_shards(mut self, num_shards: usize) -> Self {
        self.num_shards = num_shards;
        self
    }

    pub fn max_partitioning_rounds(mut self, max_partitioning_rounds: RoundId) -> Self {
        self.max_partitioning_rounds = max_partitioning_rounds;
        self
    }

    pub fn cross_shard_dep_avoid_threshold(mut self, threshold: f32) -> Self {
        self.cross_shard_dep_avoid_threshold = threshold;
        self
    }

    pub fn partition_last_round(mut self, partition_last_round: bool) -> Self {
        self.partition_last_round = partition_last_round;
        self
    }

    pub fn build(self) -> ShardedBlockPartitioner {
        ShardedBlockPartitioner::new(
            self.num_shards,
            self.max_partitioning_rounds,
            self.cross_shard_dep_avoid_threshold,
            self.partition_last_round,
        )
    }
}

impl Default for PartitionerV1Config {
    fn default() -> Self {
        Self::new()
    }
}
