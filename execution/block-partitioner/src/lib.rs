// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod sharded_block_partitioner;
pub mod v2;

pub mod test_utils;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use aptos_types::block_executor::partitioner::{CrossShardDependencies, RoundId, ShardId, SubBlock, SubBlocksForShard, TransactionWithDependencies};
use aptos_types::transaction::analyzed_transaction::{AnalyzedTransaction, StorageLocation};
use aptos_types::transaction::Transaction;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use rand::thread_rng;
use aptos_crypto::hash::{CryptoHash, TestOnlyHash};
use aptos_crypto::HashValue;
use aptos_logger::info;
use aptos_types::state_store::state_key::StateKey;
use move_core_types::account_address::AccountAddress;
use crate::v2::PartitionerV2;
use crate::sharded_block_partitioner::ShardedBlockPartitioner;
use crate::test_utils::P2PBlockGenerator;

pub trait BlockPartitioner: Send {
    fn partition(&self, transactions: Vec<AnalyzedTransaction>, num_shards: usize)
        -> Vec<SubBlocksForShard<AnalyzedTransaction>>;
}

pub fn build_partitioner_from_envvar(maybe_num_shards: Option<usize>) -> Box<dyn BlockPartitioner> {
    match std::env::var("APTOS_BLOCK_PARTITIONER_IMPL").ok() {
        Some(v) if v.to_uppercase().as_str() == "V2" => {
            let num_threads = std::env::var("APTOS_BLOCK_PARTITIONER_V2__NUM_THREADS").ok().map(|s|s.parse::<usize>().ok().unwrap_or(8)).unwrap_or(8);
            let num_rounds_limit: usize = std::env::var("APTOS_BLOCK_PARTITIONER_V2__NUM_ROUNDS_LIMIT").ok().map(|s|s.parse::<usize>().ok().unwrap_or(4)).unwrap_or(4);
            let avoid_pct: u64 = std::env::var("APTOS_BLOCK_PARTITIONER_V2__STOP_DISCARDING_IF_REMAIN_PCT_LESS_THAN").ok().map(|s|s.parse::<u64>().ok().unwrap_or(10)).unwrap_or(10);
            let dashmap_num_shards = std::env::var("APTOS_BLOCK_PARTITIONER_V2__DASHMAP_NUM_SHARDS").ok().map(|v|v.parse::<usize>().unwrap_or(256)).unwrap_or(256);
            info!("Creating V2Partitioner with num_threads={}, num_rounds_limit={}, avoid_pct={}, dashmap_num_shards={}", num_threads, num_rounds_limit, avoid_pct, dashmap_num_shards);
            Box::new(PartitionerV2::new(num_threads, num_rounds_limit, avoid_pct, dashmap_num_shards))
        }
        _ => {
            Box::new(ShardedBlockPartitioner::new(maybe_num_shards.unwrap()))
        }
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
pub fn verify_partitioner_output(before_partition: &Vec<AnalyzedTransaction>, after_partition: &Vec<SubBlocksForShard<AnalyzedTransaction>>) {
    let old_txn_id_by_txn_hash: HashMap<HashValue, usize> = HashMap::from_iter(before_partition.iter().enumerate().map(|(tid,txn)|{
        (txn.test_only_hash(), tid)
    }));

    let mut total_comm_cost = 0;
    let num_txns = before_partition.len();
    let num_shards = after_partition.len();
    let num_rounds = after_partition.first().map(|sbs|sbs.sub_blocks.len()).unwrap_or(0);
    for shard_id in 1..num_shards {
        assert_eq!(num_rounds, after_partition[shard_id].sub_blocks.len());
    }
    let mut old_tids_by_sender: HashMap<Sender, Vec<usize>> = HashMap::new();
    let mut old_tids_seen: HashSet<usize> = HashSet::new();
    let mut edge_set_from_src_view: HashSet<(usize, usize, usize, HashValue, usize, usize, usize)> = HashSet::new();
    let mut edge_set_from_dst_view: HashSet<(usize, usize, usize, HashValue, usize, usize, usize)> = HashSet::new();
    for round_id in 0..num_rounds {
        for (shard_id, sub_block_list) in after_partition.iter().enumerate() {
            let sub_block = sub_block_list.get_sub_block(round_id).unwrap();
            let mut cur_sub_block_inbound_costs_by_key_src_pair: HashMap<(RoundId, ShardId, StateKey), u64> = HashMap::new();
            let mut cur_sub_block_connectivity_by_key_dst_pair: HashMap<(RoundId, ShardId, StateKey), u64> = HashMap::new();
            for (local_tid, td) in sub_block.transactions.iter().enumerate() {
                let sender = td.txn.sender();
                let old_tid = *old_txn_id_by_txn_hash.get(&td.txn().test_only_hash()).unwrap();
                old_tids_seen.insert(old_tid);
                old_tids_by_sender.entry(sender).or_insert_with(Vec::new).push(old_tid);
                let tid = sub_block.start_index + local_tid;
                for loc in td.txn.write_hints().iter() {
                    let key = loc.clone().into_state_key();
                    let key_str = CryptoHash::hash(&key).to_hex();
                    println!("MATRIX_REPORT - round={}, shard={}, old_tid={}, new_tid={}, write_hint={}", round_id, shard_id, old_tid, tid, key_str);
                }
                for (src_tid, locs) in td.cross_shard_dependencies.required_edges().iter() {
                    for loc in locs.iter() {
                        let key = loc.clone().into_state_key();
                        let key_str = CryptoHash::hash(&key).to_hex();
                        println!("MATRIX_REPORT - round={}, shard={}, old_tid={}, new_tid={}, recv key={} from round={}, shard={}, new_tid={}", round_id, shard_id, old_tid, tid, key_str, src_tid.round_id, src_tid.shard_id, src_tid.txn_index);
                        if (round_id != num_rounds - 1) {
                            assert_ne!(src_tid.round_id, round_id);
                        }
                        assert!((src_tid.round_id, src_tid.shard_id) < (round_id, shard_id));
                        edge_set_from_dst_view.insert((src_tid.round_id, src_tid.shard_id, src_tid.txn_index, CryptoHash::hash(&key), round_id, shard_id, tid));
                        let value = cur_sub_block_inbound_costs_by_key_src_pair.entry((src_tid.round_id, src_tid.shard_id, key)).or_insert_with(||0);
                        *value += 1;
                    }
                }
                for (dst_tid, locs) in td.cross_shard_dependencies.dependent_edges().iter() {
                    for loc in locs.iter() {
                        let key = loc.clone().into_state_key();
                        let key_str = CryptoHash::hash(&key).to_hex();
                        println!("MATRIX_REPORT - round={}, shard={}, old_tid={}, new_tid={}, send key={} to round={}, shard={}, new_tid={}", round_id, shard_id, old_tid, tid, key_str, dst_tid.round_id, dst_tid.shard_id, dst_tid.txn_index);
                        if (round_id != num_rounds - 1) {
                            assert_ne!(dst_tid.round_id, round_id);
                        }
                        assert!((round_id, shard_id) < (dst_tid.round_id, dst_tid.shard_id));
                        edge_set_from_src_view.insert((round_id, shard_id, tid, CryptoHash::hash(&key), dst_tid.round_id, dst_tid.shard_id, dst_tid.txn_index));
                        let value = cur_sub_block_connectivity_by_key_dst_pair.entry((dst_tid.round_id, dst_tid.shard_id, key)).or_insert_with(||0);
                        *value += 1;
                    }
                }
            }
            let inbound_cost: u64 = cur_sub_block_inbound_costs_by_key_src_pair.iter().map(|(_,b)| *b).sum();
            let outbound_cost: u64 = cur_sub_block_connectivity_by_key_dst_pair.iter().map(|(_,b)| *b).sum();
            println!("MATRIX_REPORT: round={}, shard={}, sub_block_size={}, inbound_cost={}, outbound_cost={}", round_id, shard_id, sub_block.num_txns(), inbound_cost, outbound_cost);
            if round_id == 0 {
                assert_eq!(0, inbound_cost);
            }
            total_comm_cost += inbound_cost + outbound_cost;
        }
    }
    assert_eq!(HashSet::from_iter(0..num_txns), old_tids_seen);
    assert_eq!(edge_set_from_src_view, edge_set_from_dst_view);
    for (sender, old_tids) in old_tids_by_sender {
        assert!(is_sorted(&old_tids));
    }
    info!("MATRIX_REPORT: total_comm_cost={}", total_comm_cost);
}

fn is_sorted(arr: &Vec<usize>) -> bool {
    let num = arr.len();
    for i in 1..num {
        if arr[i-1] >= arr[i] {return false;}
    }
    return true;
}

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
