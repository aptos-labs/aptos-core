// Copyright © Aptos Foundation

#![feature(is_sorted)]

use std::collections::{BTreeSet, HashMap, HashSet};
use std::iter::Chain;
use std::slice::Iter;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, RwLock};
use dashmap::{DashMap, DashSet};
use itertools::Itertools;
use once_cell::sync::Lazy;
use rayon::prelude::{IntoParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator};
use aptos_metrics_core::{HistogramVec, register_histogram_vec};
use aptos_types::block_executor::partitioner::{CrossShardDependencies, RoundId, ShardedTxnIndex, ShardId, SubBlock, SubBlocksForShard, TransactionWithDependencies};
use aptos_types::state_store::state_key::StateKey;
use aptos_types::transaction::analyzed_transaction::{AnalyzedTransaction, StorageLocation};
use aptos_types::transaction::Transaction;
use move_core_types::account_address::AccountAddress;
use crate::{add_edges, BlockPartitioner, get_anchor_shard_id};
use aptos_metrics_core::exponential_buckets;
use rayon::iter::ParallelIterator;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::cmp;
use std::ops::Deref;
use rand::rngs::OsRng;
use rand::{Rng, thread_rng};
use aptos_crypto::hash::CryptoHash;
use aptos_crypto::HashValue;
use serde::{Deserialize, Serialize};
use storage_location_helper::StorageLocationHelper;
use crate::test_utils::P2pBlockGenerator;

type Sender = Option<AccountAddress>;

mod storage_location_helper;

pub struct OmegaPartitioner {
    thread_pool: ThreadPool,
    num_rounds_limit: usize,
    avoid_pct: u64,
    dashmap_num_shards: usize,
}

impl OmegaPartitioner {
    pub fn new() -> Self {
        let num_threads = std::env::var("OMEGA_PARTITIONER__NUM_THREADS").ok().map(|s|s.parse::<usize>().ok().unwrap_or(8)).unwrap_or(8);
        let num_rounds_limit: usize = std::env::var("OMEGA_PARTITIONER__NUM_ROUNDS_LIMIT").ok().map(|s|s.parse::<usize>().ok().unwrap_or(4)).unwrap_or(4);
        let avoid_pct: u64 = std::env::var("OMEGA_PARTITIONER__STOP_DISCARDING_IF_REMAIN_PCT_LESS_THAN").ok().map(|s|s.parse::<u64>().ok().unwrap_or(10)).unwrap_or(10);
        let dashmap_num_shards = std::env::var("OMEGA_PARTITIONER__DASHMAP_NUM_SHARDS").ok().map(|v|v.parse::<usize>().unwrap_or(256)).unwrap_or(256);
        println!("OmegaPartitioner with num_threads={}, num_rounds_limit={}, avoid_pct={}, dashmap_num_shards={}", num_threads, num_rounds_limit, avoid_pct, dashmap_num_shards);
        Self {
            thread_pool: ThreadPoolBuilder::new().num_threads(num_threads).build().unwrap(),
            num_rounds_limit,
            avoid_pct,
            dashmap_num_shards,
        }
    }

    fn add_edges(
        &self,
        txns: &Vec<Mutex<Option<AnalyzedTransaction>>>,
        txn_id_matrix: &Vec<Vec<Vec<usize>>>,
        helpers: &DashMap<usize, RwLock<StorageLocationHelper>>,
    ) -> Vec<SubBlocksForShard<AnalyzedTransaction>>{
        let timer = OMEGA_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["add_edges__init"]).start_timer();

        let num_txns = txns.len();
        let num_rounds = txn_id_matrix.len();
        let num_shards = txn_id_matrix.first().unwrap().len();

        let mut global_txn_counter: usize = 0;
        let mut new_indices: Vec<usize> = vec![0; num_txns];

        let mut start_index_matrix: Vec<Vec<usize>> = vec![vec![0; num_shards]; num_rounds];
        for (round_id, row) in txn_id_matrix.iter().enumerate() {
            for (shard_id, txn_ids) in row.iter().enumerate() {
                let num_txns_in_cur_sub_block = txn_ids.len();
                for (pos_inside_sub_block, txn_id) in txn_ids.iter().enumerate() {
                    let new_index = global_txn_counter + pos_inside_sub_block;
                    new_indices[*txn_id] = new_index;
                }
                start_index_matrix[round_id][shard_id] = global_txn_counter;
                global_txn_counter += num_txns_in_cur_sub_block;
            }
        }

        let mut sub_block_matrix: Vec<Vec<Mutex<Option<SubBlock<AnalyzedTransaction>>>>> = Vec::with_capacity(num_rounds);
        for _round_id in 0..num_rounds {
            let mut row = Vec::with_capacity(num_shards);
            for shard_id in 0..num_shards {
                row.push(Mutex::new(None));
            }
            sub_block_matrix.push(row);
        }
        let duration = timer.stop_and_record();
        println!("add_edges__init={duration}");
        let timer = OMEGA_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["add_edges__main"]).start_timer();
        self.thread_pool.install(||{
            (0..num_rounds).into_par_iter().for_each(|round_id| {
                (0..num_shards).into_par_iter().for_each(|shard_id| {
                    let cur_sub_block_size = txn_id_matrix[round_id][shard_id].len();
                    let mut twds: Vec<TransactionWithDependencies<AnalyzedTransaction>> = Vec::with_capacity(cur_sub_block_size);
                    (0..cur_sub_block_size).into_iter().for_each(|pos_in_sub_block|{
                        let txn_id = txn_id_matrix[round_id][shard_id][pos_in_sub_block];
                        let txn = txns[txn_id].lock().unwrap().take().unwrap();
                        let mut deps = CrossShardDependencies::default();
                        for loc in txn.write_hints.iter().chain(txn.read_hints.iter()) {
                            let loc_id = *loc.maybe_id_in_partition_session.as_ref().unwrap();
                            let helper_ref = helpers.get(&loc_id).unwrap();
                            let helper = helper_ref.read().unwrap();
                            if let Some(fat_id) = helper.promoted_writer_ids.range(..TxnFatId::new(round_id, shard_id, 0)).last() {
                                let src_txn_idx_fat = ShardedTxnIndex {
                                    txn_index: new_indices[fat_id.old_txn_idx],
                                    shard_id: fat_id.shard_id,
                                    round_id: fat_id.round_id,
                                };
                                deps.add_required_edge(src_txn_idx_fat, loc.clone());
                            }
                        }
                        for loc in txn.write_hints.iter() {
                            let loc_id = *loc.maybe_id_in_partition_session.as_ref().unwrap();
                            let helper_ref = helpers.get(&loc_id).unwrap();
                            let helper = helper_ref.read().unwrap();
                            let is_last_writer_in_cur_sub_block = helper.promoted_writer_ids.range(TxnFatId::new(round_id, shard_id, txn_id + 1)..TxnFatId::new(round_id, shard_id + 1, 0)).next().is_none();
                            if is_last_writer_in_cur_sub_block {
                                let mut end_id = TxnFatId::new(num_rounds, num_shards, 0); // Guaranteed to be invalid.
                                for follower_id in helper.promoted_txn_ids.range(TxnFatId::new(round_id, shard_id + 1, 0)..) {
                                    if *follower_id > end_id {
                                        break;
                                    }
                                    let dst_txn_idx_fat = ShardedTxnIndex {
                                        txn_index: new_indices[follower_id.old_txn_idx],
                                        shard_id: follower_id.shard_id,
                                        round_id: follower_id.round_id,
                                    };
                                    deps.add_dependent_edge(dst_txn_idx_fat, vec![loc.clone()]);
                                    if helper.writer_set.contains(&follower_id.old_txn_idx) {
                                        end_id = TxnFatId::new(follower_id.round_id, follower_id.shard_id + 1, 0);
                                    }
                                }
                            }
                        }
                        let twd = TransactionWithDependencies::new(txn, deps);
                        twds.push(twd);
                    });
                    let sub_block = SubBlock::new(start_index_matrix[round_id][shard_id], twds);
                    *sub_block_matrix[round_id][shard_id].lock().unwrap() = Some(sub_block);
                });
            });
        });
        let duration = timer.stop_and_record();
        println!("add_edges__main={duration}");

        let timer = OMEGA_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["add_edges__return_obj"]).start_timer();
        let ret: Vec<SubBlocksForShard<AnalyzedTransaction>> = (0..num_shards).map(|shard_id|{
            let sub_blocks: Vec<SubBlock<AnalyzedTransaction>> = (0..num_rounds).map(|round_id|{
                sub_block_matrix[round_id][shard_id].lock().unwrap().take().unwrap()
            }).collect();
            SubBlocksForShard::new(shard_id, sub_blocks)
        }).collect();
        let duration = timer.stop_and_record();
        println!("add_edges__return_obj={duration}");
        self.thread_pool.install(move||{
            drop(sub_block_matrix);
            drop(start_index_matrix);
        });
        ret
    }

    fn discarding_round(
        &self,
        round_id: usize,
        txns: &Vec<AnalyzedTransaction>,
        txn_id_vecs: Vec<Vec<usize>>,
        loc_helpers: &DashMap<usize, RwLock<StorageLocationHelper>>,
        start_txn_id_by_shard_id: &Vec<usize>,
    ) -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
        let timer = OMEGA_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&[format!("round_{round_id}__init").as_str()]).start_timer();
        let num_shards = txn_id_vecs.len();
        let mut discarded: Vec<RwLock<Vec<usize>>> = Vec::with_capacity(num_shards);
        let mut potentially_accepted: Vec<RwLock<Vec<usize>>> = Vec::with_capacity(num_shards);
        let mut finally_accepted: Vec<RwLock<Vec<usize>>> = Vec::with_capacity(num_shards);
        for shard_id in 0..num_shards {
            potentially_accepted.push(RwLock::new(Vec::with_capacity(txn_id_vecs[shard_id].len())));
            finally_accepted.push(RwLock::new(Vec::with_capacity(txn_id_vecs[shard_id].len())));
            discarded.push(RwLock::new(Vec::with_capacity(txn_id_vecs[shard_id].len())));
        }

        let min_discarded_seq_nums_by_sender_id: DashMap<usize, AtomicUsize> = DashMap::new();
        let shard_id_and_txn_id_vec_pairs: Vec<(usize, Vec<usize>)> = txn_id_vecs.into_iter().enumerate().collect();
        let duration = timer.stop_and_record();
        println!("round_{}__init={}", round_id, duration);
        let timer = OMEGA_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&[format!("round_{round_id}__discard_by_key").as_str()]).start_timer();
        self.thread_pool.install(|| {
            shard_id_and_txn_id_vec_pairs.into_par_iter().for_each(|(my_shard_id, txn_ids)| {
                txn_ids.into_par_iter().for_each(|txn_id| {
                    let txn = txns.get(txn_id).unwrap();
                    let in_round_conflict_detected = txn.write_hints.iter().chain(txn.read_hints.iter()).any(|loc| {
                        let loc_id = *loc.maybe_id_in_partition_session.as_ref().unwrap();
                        let loc_helper = loc_helpers.get(&loc_id).unwrap();
                        let loc_helper_read = loc_helper.read().unwrap();
                        let anchor_shard_id = loc_helper_read.anchor_shard_id;
                        loc_helper_read.has_write_in_range(start_txn_id_by_shard_id[anchor_shard_id], start_txn_id_by_shard_id[my_shard_id])
                    });
                    if in_round_conflict_detected {
                        let sender_id = txn.maybe_sender_id_in_partition_session.unwrap();
                        min_discarded_seq_nums_by_sender_id.entry(sender_id).or_insert_with(|| AtomicUsize::new(usize::MAX)).value().fetch_min(txn_id, Ordering::SeqCst);
                        discarded[my_shard_id].write().unwrap().push(txn_id);
                    } else {
                        potentially_accepted[my_shard_id].write().unwrap().push(txn_id);
                    }
                });
            });
        });
        let duration = timer.stop_and_record();
        println!("round_{}__discard_by_key={}", round_id, duration);
        let timer = OMEGA_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&[format!("round_{round_id}__discard_by_sender").as_str()]).start_timer();
        self.thread_pool.install(||{
            (0..num_shards).into_par_iter().for_each(|shard_id|{
                potentially_accepted[shard_id].read().unwrap().par_iter().for_each(|txn_id|{
                    let txn = txns.get(*txn_id).unwrap();
                    let sender_id = txn.maybe_sender_id_in_partition_session.unwrap();
                    let min_discarded_txn_id = min_discarded_seq_nums_by_sender_id.entry(sender_id).or_insert_with(|| AtomicUsize::new(usize::MAX)).load(Ordering::SeqCst);
                    if *txn_id < min_discarded_txn_id {
                        for loc in txn.write_hints.iter().chain(txn.read_hints.iter()) {
                            let loc_id = *loc.maybe_id_in_partition_session.as_ref().unwrap();
                            loc_helpers.get(&loc_id).unwrap().write().unwrap().promote_txn_id(*txn_id, round_id, shard_id);
                        }
                        finally_accepted[shard_id].write().unwrap().push(*txn_id);
                    } else {
                        discarded[shard_id].write().unwrap().push(*txn_id);
                    }
                });
            });
        });
        let duration = timer.stop_and_record();
        println!("round_{}__discard_by_sender={}", round_id, duration);
        let timer = OMEGA_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&[format!("round_{round_id}__return_obj").as_str()]).start_timer();
        let ret = (extract_and_sort(finally_accepted), extract_and_sort(discarded));
        let duration = timer.stop_and_record();
        println!("round_{}__return_obj={}", round_id, duration);
        self.thread_pool.install(move||{
            drop(potentially_accepted);
            drop(min_discarded_seq_nums_by_sender_id);

        });
        ret
    }

}

impl BlockPartitioner for OmegaPartitioner {
    fn partition(&self, mut txns: Vec<AnalyzedTransaction>, num_executor_shards: usize) -> Vec<SubBlocksForShard<AnalyzedTransaction>> {
        let timer = OMEGA_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["preprocess"]).start_timer();
        let num_txns = txns.len();
        let mut num_senders = AtomicUsize::new(0);
        let mut num_keys = AtomicUsize::new(0);
        let mut sender_ids_by_sender: DashMap<Sender, usize> = DashMap::with_shard_amount(self.dashmap_num_shards);
        let mut key_ids_by_key: DashMap<StateKey, usize> = DashMap::with_shard_amount(self.dashmap_num_shards);
        let mut helpers_by_key_id: DashMap<usize, RwLock<StorageLocationHelper>> = DashMap::with_shard_amount(self.dashmap_num_shards);
        for (txn_id, txn) in txns.iter_mut().enumerate() {
            txn.maybe_txn_id_in_partition_session = Some(txn_id);
        }
        self.thread_pool.install(||{
            txns.par_iter_mut().for_each(|mut txn| {
                let txn_id = *txn.maybe_txn_id_in_partition_session.as_ref().unwrap();
                let sender = txn.sender();
                let sender_id = *sender_ids_by_sender.entry(sender).or_insert_with(||{
                    num_senders.fetch_add(1, Ordering::SeqCst)
                });
                txn.maybe_sender_id_in_partition_session = Some(sender_id);
                let num_writes = txn.write_hints.len();
                for (i, storage_location) in txn.write_hints.iter_mut().chain(txn.read_hints.iter_mut()).enumerate() {
                    let key = storage_location.maybe_state_key().unwrap().clone();
                    let key_id = *key_ids_by_key.entry(key).or_insert_with(|| {
                        num_keys.fetch_add(1, Ordering::SeqCst)
                    });
                    storage_location.maybe_id_in_partition_session = Some(key_id);
                    let is_write = i < num_writes;
                    helpers_by_key_id.entry(key_id).or_insert_with(|| {
                        let anchor_shard_id = get_anchor_shard_id(storage_location, num_executor_shards);
                        RwLock::new(StorageLocationHelper::new(anchor_shard_id))
                    }).write().unwrap().add_candidate(txn_id, is_write);

                }
            });
        });
        let duration = timer.stop_and_record();
        println!("preprocess={duration:?}");

        // print_storage_location_helper_summary(&key_ids_by_key, &helpers_by_key_id);

        let timer = OMEGA_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["pre_partition_uniform"]).start_timer();
        let mut remaining_txns = uniform_partition(num_txns, num_executor_shards);
        let mut start_txn_ids_by_shard_id = vec![0; num_executor_shards];
        {
            for shard_id in 1..num_executor_shards {
                start_txn_ids_by_shard_id[shard_id] = start_txn_ids_by_shard_id[shard_id - 1] + remaining_txns[shard_id - 1].len();
            }
        }
        let duration = timer.stop_and_record();
        println!("pre_partition_uniform={duration:?}");

        let mut txn_id_matrix: Vec<Vec<Vec<usize>>> = Vec::new();
        for round_id in 0..(self.num_rounds_limit - 1) {
            let timer = OMEGA_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&[format!("round_{round_id}").as_str()]).start_timer();
            let (accepted, discarded) = self.discarding_round(round_id, &txns, remaining_txns, &helpers_by_key_id, &start_txn_ids_by_shard_id);
            txn_id_matrix.push(accepted);
            remaining_txns = discarded;
            let num_remaining_txns: usize = remaining_txns.iter().map(|ts|ts.len()).sum();
            let duration = timer.stop_and_record();
            println!("round_{round_id}={duration:?}");
            if num_remaining_txns < self.avoid_pct as usize * num_txns / 100 {
                break;
            }
        }

        if remaining_txns.len() >= 1 {
            let timer = OMEGA_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["last_round"]).start_timer();
            let last_round_txns: Vec<usize> = remaining_txns.into_iter().flatten().collect();
            for txn_id in last_round_txns.iter() {
                let txn = &txns[*txn_id];
                for loc in txn.read_hints.iter().chain(txn.write_hints.iter()) {
                    let loc_id = *loc.maybe_id_in_partition_session.as_ref().unwrap();
                    let helper = helpers_by_key_id.get(&loc_id).unwrap();
                    helper.write().unwrap().promote_txn_id(*txn_id, self.num_rounds_limit - 1, num_executor_shards - 1);
                }
            }

            remaining_txns = vec![vec![]; num_executor_shards];
            remaining_txns[num_executor_shards - 1] = last_round_txns;
            txn_id_matrix.push(remaining_txns);
            let duration = timer.stop_and_record();
            println!("last_round={duration:?}");
        }

        // print_storage_location_helper_summary(&key_ids_by_key, &helpers_by_key_id);

        let timer = OMEGA_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["add_edges"]).start_timer();
        let txns: Vec<Mutex<Option<AnalyzedTransaction>>> = txns.into_iter().map(|t|Mutex::new(Some(t))).collect();
        let ret = self.add_edges(&txns, &txn_id_matrix, &helpers_by_key_id);
        let duration = timer.stop_and_record();
        println!("add_edges={duration:?}");
        self.thread_pool.install(move||{
            drop(sender_ids_by_sender);
            drop(key_ids_by_key);
            drop(helpers_by_key_id);
            drop(start_txn_ids_by_shard_id);
            drop(txn_id_matrix);
            drop(txns);
        });
        ret
    }
}

fn print_storage_location_helper_summary(key_ids_by_key: &DashMap<StateKey, usize>, helpers_by_key_id: &DashMap<usize, RwLock<StorageLocationHelper>>) {
    for kv in key_ids_by_key.iter() {
        let key = kv.key();
        let key_id = *kv.value();
        let helper = helpers_by_key_id.get(&key_id).unwrap();
        println!("HELPER CHECK - key_id={}, key={}, helper={}", key_id, key.hash().to_hex(), helper.read().unwrap().brief());
    }
}

#[test]
fn test_omega_partitioner() {
    let block_generator = P2pBlockGenerator::new(100);
    let partitioner = OmegaPartitioner::new();
    let mut rng = thread_rng();
    for run_id in 0..100 {
        let block_size = 10_u64.pow(rng.gen_range(0, 4)) as usize;
        let num_shards = rng.gen_range(1, 10);
        let block = block_generator.rand_block(&mut rng, block_size);
        let block_clone = block.clone();
        let partitioned = partitioner.partition(block, num_shards);
        assertions(&block_clone, &partitioned);
    }

}

pub fn assertions(before_partition: &Vec<AnalyzedTransaction>, after_partition: &Vec<SubBlocksForShard<AnalyzedTransaction>>) {
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
                let old_tid = *td.txn.maybe_txn_id_in_partition_session.as_ref().unwrap();
                old_tids_seen.insert(old_tid);
                old_tids_by_sender.entry(sender).or_insert_with(Vec::new).push(old_tid);
                let tid = sub_block.start_index + local_tid;
                for loc in td.txn.write_hints.iter() {
                    let key = loc.clone().into_state_key();
                    let key_str = CryptoHash::hash(&key).to_hex();
                    // println!("MATRIX_REPORT - round={}, shard={}, old_tid={}, new_tid={}, write_hint={}", round_id, shard_id, old_tid, tid, key_str);
                }
                for (src_tid, locs) in td.cross_shard_dependencies.required_edges().iter() {
                    for loc in locs.iter() {
                        let key = loc.clone().into_state_key();
                        let key_str = CryptoHash::hash(&key).to_hex();
                        // println!("MATRIX_REPORT - round={}, shard={}, old_tid={}, new_tid={}, recv key={} from round={}, shard={}, new_tid={}", round_id, shard_id, old_tid, tid, key_str, src_tid.round_id, src_tid.shard_id, src_tid.txn_index);
                        if (round_id != num_rounds - 1) {
                            assert_ne!(src_tid.round_id, round_id);
                        }
                        assert!((src_tid.round_id, src_tid.shard_id) < (round_id, shard_id));
                        edge_set_from_dst_view.insert((src_tid.round_id, src_tid.shard_id, src_tid.txn_index, key.hash(), round_id, shard_id, tid));
                        let value = cur_sub_block_inbound_costs_by_key_src_pair.entry((src_tid.round_id, src_tid.shard_id, key)).or_insert_with(||0);
                        *value += 1;
                    }
                }
                for (dst_tid, locs) in td.cross_shard_dependencies.dependent_edges().iter() {
                    for loc in locs.iter() {
                        let key = loc.clone().into_state_key();
                        let key_str = CryptoHash::hash(&key).to_hex();
                        // println!("MATRIX_REPORT - round={}, shard={}, old_tid={}, new_tid={}, send key={} to round={}, shard={}, new_tid={}", round_id, shard_id, old_tid, tid, key_str, dst_tid.round_id, dst_tid.shard_id, dst_tid.txn_index);
                        if (round_id != num_rounds - 1) {
                            assert_ne!(dst_tid.round_id, round_id);
                        }
                        assert!((round_id, shard_id) < (dst_tid.round_id, dst_tid.shard_id));
                        edge_set_from_src_view.insert((round_id, shard_id, tid, key.hash(), dst_tid.round_id, dst_tid.shard_id, dst_tid.txn_index));
                        let value = cur_sub_block_connectivity_by_key_dst_pair.entry((dst_tid.round_id, dst_tid.shard_id, key)).or_insert_with(||0);
                        *value += 1;
                    }
                }
            }
            let inbound_cost: u64 = cur_sub_block_inbound_costs_by_key_src_pair.iter().map(|(_,b)| *b).sum();
            let outbound_cost: u64 = cur_sub_block_connectivity_by_key_dst_pair.iter().map(|(_,b)| *b).sum();
            if round_id == 0 {
                assert_eq!(0, inbound_cost);
            }
            if round_id == num_rounds - 1 && round_id >= 1 && shard_id < num_shards - 1 {
                assert_eq!(0, sub_block.num_txns());
            }
            // println!("MATRIX_REPORT: round={}, shard={}, sub_block_size={}, inbound_cost={}, outbound_cost={}", round_id, shard_id, sub_block.num_txns(), inbound_cost, outbound_cost);
            total_comm_cost += inbound_cost + outbound_cost;
        }
    }
    assert_eq!(HashSet::from_iter(0..num_txns), old_tids_seen);
    assert_eq!(edge_set_from_dst_view, edge_set_from_dst_view);
    for (sender, old_tids) in old_tids_by_sender {
        let num = old_tids.len();
        for i in 1..num {
            assert!(old_tids[i-1] < old_tids[i]);
        }
    }
    // assert_eq!(0, total_comm_cost % 2);
    // println!("MATRIX_REPORT: total_comm_cost={}", total_comm_cost);
}


/// 18,5 -> [4,4,4,3,3]
fn uniform_partition(num_items: usize, num_chunks: usize) -> Vec<Vec<usize>> {
    let num_big_chunks = num_items % num_chunks;
    let small_chunk_size = num_items / num_chunks;
    let mut ret = Vec::with_capacity(num_chunks);
    let mut next_chunk_start = 0;
    for chunk_id in 0..num_chunks {
        let extra = if chunk_id < num_big_chunks { 1 } else { 0 };
        let next_chunk_end = next_chunk_start + small_chunk_size + extra;
        let chunk: Vec<usize> = (next_chunk_start..next_chunk_end).collect();
        next_chunk_start = next_chunk_end;
        ret.push(chunk);
    }
    ret
}

#[test]
fn test_uniform_partition() {
    let actual = uniform_partition(18, 5);
    assert_eq!(vec![4,4,4,3,3], actual.iter().map(|v|v.len()).collect::<Vec<usize>>());
    assert_eq!((0..18).collect::<Vec<usize>>(), actual.concat());

    let actual = uniform_partition(18, 3);
    assert_eq!(vec![6,6,6], actual.iter().map(|v|v.len()).collect::<Vec<usize>>());
    assert_eq!((0..18).collect::<Vec<usize>>(), actual.concat());
}


fn extract_and_sort(arr_2d: Vec<RwLock<Vec<usize>>>) -> Vec<Vec<usize>> {
    arr_2d.into_iter().map(|arr_1d|{
        let mut x = arr_1d.write().unwrap();
        let mut y = std::mem::replace(&mut *x, vec![]);
        y.sort();
        y
    }).collect::<Vec<_>>()
}

pub static OMEGA_PARTITIONER_MISC_TIMERS_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "omega_partitioner_misc_timers_seconds",
        // metric description
        "The time spent in seconds of miscellaneous phases of OmegaPartitioner.",
        &["name"],
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
        .unwrap()
});

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, Serialize, Deserialize)]
pub struct TxnFatId {
    pub round_id: usize,
    pub shard_id: usize,
    pub old_txn_idx: usize,
}

impl PartialOrd for TxnFatId {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        (self.round_id, self.shard_id, self.old_txn_idx).partial_cmp(&(other.round_id, other.shard_id, other.old_txn_idx))
    }
}

impl TxnFatId {
    pub fn new(round_id: usize, shard_id: usize, old_txn_idx: usize) -> Self {
        Self {
            round_id,
            shard_id,
            old_txn_idx,
        }
    }
}
