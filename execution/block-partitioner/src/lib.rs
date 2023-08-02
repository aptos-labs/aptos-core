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
use aptos_crypto::hash::{CryptoHash, TestOnlyHash};
use aptos_crypto::HashValue;
use aptos_logger::info;
use aptos_types::state_store::state_key::StateKey;
use move_core_types::account_address::AccountAddress;
use crate::v2::V2Partitioner;
use crate::sharded_block_partitioner::ShardedBlockPartitioner;

pub trait BlockPartitioner: Send {
    fn partition(&self, transactions: Vec<AnalyzedTransaction>, num_shards: usize)
        -> Vec<SubBlocksForShard<AnalyzedTransaction>>;
}

/// An implementation of partitioner that splits the transactions into equal-sized chunks.
pub struct UniformPartitioner {}

impl BlockPartitioner for UniformPartitioner {
    fn partition(
        &self,
        transactions: Vec<AnalyzedTransaction>,
        num_shards: usize,
    ) -> Vec<SubBlocksForShard<AnalyzedTransaction>> {
        let total_txns = transactions.len();
        if total_txns == 0 {
            return vec![];
        }
        let txns_per_shard = (total_txns as f64 / num_shards as f64).ceil() as usize;

        let mut result: Vec<SubBlocksForShard<AnalyzedTransaction>> = Vec::new();
        let mut global_txn_counter: usize = 0;
        for (shard_id, chunk) in transactions.chunks(txns_per_shard).enumerate() {
            let twds: Vec<TransactionWithDependencies<AnalyzedTransaction>> = chunk.iter().map(|t|TransactionWithDependencies::new(t.clone(), CrossShardDependencies::default())).collect();
            let sub_block = SubBlock::new(global_txn_counter, twds);
            global_txn_counter += sub_block.num_txns();
            result.push(SubBlocksForShard::new(shard_id, vec![sub_block]));
        }
        result
    }
}

fn get_anchor_shard_id(storage_location: &StorageLocation, num_shards: usize) -> ShardId {
    let mut hasher = DefaultHasher::new();
    storage_location.hash(&mut hasher);
    (hasher.finish() % num_shards as u64) as usize
}

type Sender = Option<AccountAddress>;

pub fn assertions(before_partition: &Vec<AnalyzedTransaction>, after_partition: &Vec<SubBlocksForShard<AnalyzedTransaction>>) {
    let old_txn_id_by_txn_hash: HashMap<HashValue, usize> = HashMap::from_iter(before_partition.iter().enumerate().map(|(tid,txn)|{
        (txn.hash, tid)
    }));

    let mut total_comm_cost = 0;
    let num_txns = before_partition.len();
    let num_rounds = after_partition.first().unwrap().sub_blocks.len();
    let num_shards = after_partition.len();
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
                let old_tid = *old_txn_id_by_txn_hash.get(&td.txn().hash).unwrap();
                old_tids_seen.insert(old_tid);
                old_tids_by_sender.entry(sender).or_insert_with(Vec::new).push(old_tid);
                let tid = sub_block.start_index + local_tid;
                for loc in td.txn.write_hints().iter() {
                    let key = loc.clone().into_state_key();
                    let key_str = CryptoHash::hash(&key).to_hex();
                    info!("MATRIX_REPORT - round={}, shard={}, old_tid={}, new_tid={}, write_hint={}", round_id, shard_id, old_tid, tid, key_str);
                }
                for (src_tid, locs) in td.cross_shard_dependencies.required_edges().iter() {
                    for loc in locs.iter() {
                        let key = loc.clone().into_state_key();
                        let key_str = CryptoHash::hash(&key).to_hex();
                        info!("MATRIX_REPORT - round={}, shard={}, old_tid={}, new_tid={}, recv key={} from round={}, shard={}, new_tid={}", round_id, shard_id, old_tid, tid, key_str, src_tid.round_id, src_tid.shard_id, src_tid.txn_index);
                        // if (round_id != num_rounds - 1) {
                        //     assert_ne!(src_tid.round_id, round_id);
                        // }
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
                        info!("MATRIX_REPORT - round={}, shard={}, old_tid={}, new_tid={}, send key={} to round={}, shard={}, new_tid={}", round_id, shard_id, old_tid, tid, key_str, dst_tid.round_id, dst_tid.shard_id, dst_tid.txn_index);
                        // if (round_id != num_rounds - 1) {
                        //     assert_ne!(dst_tid.round_id, round_id);
                        // }
                        assert!((round_id, shard_id) < (dst_tid.round_id, dst_tid.shard_id));
                        edge_set_from_src_view.insert((round_id, shard_id, tid, CryptoHash::hash(&key), dst_tid.round_id, dst_tid.shard_id, dst_tid.txn_index));
                        let value = cur_sub_block_connectivity_by_key_dst_pair.entry((dst_tid.round_id, dst_tid.shard_id, key)).or_insert_with(||0);
                        *value += 1;
                    }
                }
            }
            let inbound_cost: u64 = cur_sub_block_inbound_costs_by_key_src_pair.iter().map(|(_,b)| *b).sum();
            let outbound_cost: u64 = cur_sub_block_connectivity_by_key_dst_pair.iter().map(|(_,b)| *b).sum();
            info!("MATRIX_REPORT: round={}, shard={}, sub_block_size={}, inbound_cost={}, outbound_cost={}", round_id, shard_id, sub_block.num_txns(), inbound_cost, outbound_cost);
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
    assert_eq!(0, total_comm_cost % 2);
}

fn is_sorted(arr: &Vec<usize>) -> bool {
    let num = arr.len();
    for i in 1..num {
        if arr[i-1] >= arr[i] {return false;}
    }
    return true;
}
