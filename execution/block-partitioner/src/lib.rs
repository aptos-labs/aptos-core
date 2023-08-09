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
    block_executor::partitioner::{RoundId, ShardId, SubBlocksForShard},
    state_store::state_key::StateKey,
    transaction::analyzed_transaction::{AnalyzedTransaction, StorageLocation},
};
use move_core_types::account_address::AccountAddress;
#[cfg(test)]
use rand::thread_rng;
#[cfg(test)]
use std::sync::Arc;
use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    hash::{Hash, Hasher},
};
use aptos_types::block_executor::partitioner::{GLOBAL_ROUND_ID, GLOBAL_SHARD_ID, PartitionedTransactions, TransactionWithDependencies};

pub trait BlockPartitioner: Send {
    fn partition(
        &self,
        transactions: Vec<AnalyzedTransaction>,
        num_shards: usize,
    ) -> PartitionedTransactions;
}

pub fn build_partitioner_from_envvar(maybe_num_shards: Option<usize>) -> Box<dyn BlockPartitioner> {
    match std::env::var("APTOS_BLOCK_PARTITIONER_IMPL").ok() {
        Some(v) if v.to_uppercase().as_str() == "V2" => {
            let num_threads = std::env::var("APTOS_BLOCK_PARTITIONER_V2__NUM_THREADS")
                .ok()
                .map(|s| s.parse::<usize>().ok().unwrap_or(8))
                .unwrap_or(8);
            let num_rounds_limit: usize =
                std::env::var("APTOS_BLOCK_PARTITIONER_V2__NUM_ROUNDS_LIMIT")
                    .ok()
                    .map(|s| s.parse::<usize>().ok().unwrap_or(4))
                    .unwrap_or(4);
            let avoid_pct: u64 = std::env::var(
                "APTOS_BLOCK_PARTITIONER_V2__STOP_DISCARDING_IF_REMAIN_PCT_LESS_THAN",
            )
            .ok()
            .map(|s| s.parse::<u64>().ok().unwrap_or(10))
            .unwrap_or(10);
            let dashmap_num_shards =
                std::env::var("APTOS_BLOCK_PARTITIONER_V2__DASHMAP_NUM_SHARDS")
                    .ok()
                    .map(|v| v.parse::<usize>().unwrap_or(256))
                    .unwrap_or(256);
            info!("Creating V2Partitioner with num_threads={}, num_rounds_limit={}, avoid_pct={}, dashmap_num_shards={}", num_threads, num_rounds_limit, avoid_pct, dashmap_num_shards);
            Box::new(PartitionerV2::new(
                num_threads,
                num_rounds_limit,
                avoid_pct,
                dashmap_num_shards,
            ))
        },
        _ => {
            let max_partitioning_rounds = std::env::var("APTOS_BLOCK_PARTITIONER_V2__MAX_PARTITIONING_ROUNDS").ok().map(|v|v.parse::<usize>().unwrap_or(4)).unwrap_or(4);
            let cross_shard_dep_avoid_threshold = std::env::var("APTOS_BLOCK_PARTITIONER_V2__CROSS_SHARD_DEP_AVOID_THRESHOLD").ok().map(|v|v.parse::<f32>().unwrap_or(0.9)).unwrap_or(0.9);
            let partition_last_round = std::env::var("APTOS_BLOCK_PARTITIONER_V2__PARTITION_LAST_ROUND").ok().map(|v|v.parse::<usize>().unwrap_or(0)).unwrap_or(0);
            info!("Creating V1Partitioner with max_partitioning_rounds={}, cross_shard_dep_avoid_threshold={}, partition_last_round={}", max_partitioning_rounds, cross_shard_dep_avoid_threshold, partition_last_round);
            Box::new(ShardedBlockPartitioner::new(maybe_num_shards.unwrap(), max_partitioning_rounds, cross_shard_dep_avoid_threshold, partition_last_round!=0))
        },
    }
}

pub mod uniform_partitioner;

/// When multiple transactions access the same storage location,
/// use this function to pick a shard as the anchor/leader and resolve conflicts.
/// Used by `ShardedBlockPartitioner` and `V2Partitioner`.
fn get_anchor_shard_id(storage_location: &StorageLocation, num_shards: usize) -> ShardId {
    let mut hasher = DefaultHasher::new();
    storage_location.hash(&mut hasher);
    (hasher.finish() % num_shards as u64) as usize
}

type Sender = Option<AccountAddress>;

/// Assert partitioner correctness for `ShardedBlockPartitioner` and `V2Partitioner`:
/// - Transaction set remains the same after partitioning.
/// - The relative order of the txns from the same sender
/// - For a cross-shard dependency, the consumer txn always comes after the provider txn in the sharded block.
/// - Required edge set matches dependency edge set.
/// - Before the last round, there is no in-round cross-shard dependency.
///
/// Also print a summary of the partitioning result.
pub fn verify_partitioner_output(
    before_partition: &Vec<AnalyzedTransaction>,
    after_partition: &PartitionedTransactions,
) {
    let old_txn_id_by_txn_hash: HashMap<HashValue, usize> = HashMap::from_iter(
        before_partition
            .iter()
            .enumerate()
            .map(|(tid, txn)| (txn.test_only_hash(), tid)),
    );

    let mut total_comm_cost = 0;
    let num_txns = before_partition.len();
    let num_shards = after_partition.sharded_txns().len();
    let num_rounds = after_partition.sharded_txns()
        .first()
        .map(|sbs| sbs.sub_blocks.len())
        .unwrap_or(0);
    for sub_block_list in after_partition.sharded_txns().iter().take(num_shards).skip(1) {
        assert_eq!(num_rounds, sub_block_list.sub_blocks.len());
    }
    let mut old_tids_by_sender: HashMap<Sender, Vec<usize>> = HashMap::new();
    let mut old_tids_seen: HashSet<usize> = HashSet::new();
    let mut edge_set_from_src_view: HashSet<(usize, usize, usize, HashValue, usize, usize, usize)> =
        HashSet::new();
    let mut edge_set_from_dst_view: HashSet<(usize, usize, usize, HashValue, usize, usize, usize)> =
        HashSet::new();

    let mut for_each_sub_block = |round_id: usize, shard_id: usize, start_txn_idx: usize, sub_block_txns: &[TransactionWithDependencies<AnalyzedTransaction>]| {
        let mut cur_sub_block_inbound_costs: HashMap<
            (RoundId, ShardId, StateKey),
            u64,
        > = HashMap::new();
        let mut cur_sub_block_outbound_costs: HashMap<
            (RoundId, ShardId, StateKey),
            u64,
        > = HashMap::new();
        for (local_tid, td) in sub_block_txns.iter().enumerate() {
            let sender = td.txn.sender();
            let old_tid = *old_txn_id_by_txn_hash
                .get(&td.txn().test_only_hash())
                .unwrap();
            old_tids_seen.insert(old_tid);
            old_tids_by_sender
                .entry(sender)
                .or_insert_with(Vec::new)
                .push(old_tid);
            let tid = start_txn_idx + local_tid;
            for loc in td.txn.write_hints().iter() {
                let key = loc.clone().into_state_key();
                let key_str = CryptoHash::hash(&key).to_hex();
                println!(
                    "MATRIX_REPORT - round={}, shard={}, old_tid={}, new_tid={}, write_hint={}",
                    round_id, shard_id, old_tid, tid, key_str
                );
            }
            for (src_tid, locs) in td.cross_shard_dependencies.required_edges().iter() {
                for loc in locs.iter() {
                    let key = loc.clone().into_state_key();
                    let key_str = CryptoHash::hash(&key).to_hex();
                    println!("MATRIX_REPORT - round={}, shard={}, old_tid={}, new_tid={}, recv key={} from round={}, shard={}, new_tid={}", round_id, shard_id, old_tid, tid, key_str, src_tid.round_id, src_tid.shard_id, src_tid.txn_index);
                    if round_id != num_rounds - 1 {
                        assert_ne!(src_tid.round_id, round_id);
                    }
                    assert!((src_tid.round_id, src_tid.shard_id) < (round_id, shard_id));
                    edge_set_from_dst_view.insert((
                        src_tid.round_id,
                        src_tid.shard_id,
                        src_tid.txn_index,
                        CryptoHash::hash(&key),
                        round_id,
                        shard_id,
                        tid,
                    ));
                    let value = cur_sub_block_inbound_costs
                        .entry((src_tid.round_id, src_tid.shard_id, key))
                        .or_insert_with(|| 0);
                    *value += 1;
                }
            }
            for (dst_tid, locs) in td.cross_shard_dependencies.dependent_edges().iter() {
                for loc in locs.iter() {
                    let key = loc.clone().into_state_key();
                    let key_str = CryptoHash::hash(&key).to_hex();
                    println!("MATRIX_REPORT - round={}, shard={}, old_tid={}, new_tid={}, send key={} to round={}, shard={}, new_tid={}", round_id, shard_id, old_tid, tid, key_str, dst_tid.round_id, dst_tid.shard_id, dst_tid.txn_index);
                    if round_id != num_rounds - 1 {
                        assert_ne!(dst_tid.round_id, round_id);
                    }
                    assert!((round_id, shard_id) < (dst_tid.round_id, dst_tid.shard_id));
                    edge_set_from_src_view.insert((
                        round_id,
                        shard_id,
                        tid,
                        CryptoHash::hash(&key),
                        dst_tid.round_id,
                        dst_tid.shard_id,
                        dst_tid.txn_index,
                    ));
                    let value = cur_sub_block_outbound_costs
                        .entry((dst_tid.round_id, dst_tid.shard_id, key))
                        .or_insert_with(|| 0);
                    *value += 1;
                }
            }
        }
        let inbound_cost: u64 = cur_sub_block_inbound_costs
            .values()
            .copied()
            .sum();
        let outbound_cost: u64 = cur_sub_block_outbound_costs
            .values()
            .copied()
            .sum();
        println!("MATRIX_REPORT: round={}, shard={}, sub_block_size={}, inbound_cost={}, outbound_cost={}", round_id, shard_id, sub_block_txns.len(), inbound_cost, outbound_cost);
        if round_id == 0 {
            assert_eq!(0, inbound_cost);
        }
        total_comm_cost += inbound_cost + outbound_cost;
    };

    for round_id in 0..num_rounds {
        for (shard_id, sub_block_list) in after_partition.sharded_txns().iter().enumerate() {
            let sub_block = sub_block_list.get_sub_block(round_id).unwrap();
            for_each_sub_block(round_id, shard_id, sub_block.start_index, sub_block.transactions_with_deps().as_slice())
        }
    }
    for_each_sub_block(GLOBAL_ROUND_ID, GLOBAL_SHARD_ID, after_partition.num_sharded_txns(), after_partition.global_txns.as_slice());

    assert_eq!(HashSet::from_iter(0..num_txns), old_tids_seen);
    assert_eq!(edge_set_from_src_view.len(), edge_set_from_dst_view.len());
    assert_eq!(edge_set_from_src_view, edge_set_from_dst_view);
    for (_sender, old_tids) in old_tids_by_sender {
        assert!(is_sorted(&old_tids));
    }
    info!("MATRIX_REPORT: total_comm_cost={}", total_comm_cost);
}

fn is_sorted(arr: &Vec<usize>) -> bool {
    let num = arr.len();
    for i in 1..num {
        if arr[i - 1] >= arr[i] {
            return false;
        }
    }
    true
}

#[cfg(test)]
fn assert_deterministic_result(partitioner: Arc<dyn BlockPartitioner>) {
    let mut rng = thread_rng();
    let block_gen = P2PBlockGenerator::new(1000);
    for _ in 0..100 {
        let txns = block_gen.rand_block(&mut rng, 100);
        let result_0 = partitioner.partition(txns.clone(), 10);
        let result_1 = partitioner.partition(txns, 10);
        assert_eq!(result_1, result_0);
    }
}

pub struct BlockPartitionerConfig {
    num_shards: usize,
    max_partitioning_rounds: RoundId,
    cross_shard_dep_avoid_threshold: f32,
    partition_last_round: bool,
}

impl BlockPartitionerConfig {
    pub fn new() -> Self {
        BlockPartitionerConfig {
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

impl Default for BlockPartitionerConfig {
    fn default() -> Self {
        Self::new()
    }
}
