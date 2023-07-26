// Copyright © Aptos Foundation

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::RwLock;
use dashmap::DashMap;
use itertools::Itertools;
use once_cell::sync::Lazy;
use rayon::prelude::IntoParallelRefMutIterator;
use aptos_metrics_core::{HistogramVec, register_histogram_vec};
use aptos_types::block_executor::partitioner::{SubBlock, SubBlocksForShard};
use aptos_types::state_store::state_key::StateKey;
use aptos_types::transaction::analyzed_transaction::{AnalyzedTransaction, StorageLocationHelper};
use aptos_types::transaction::Transaction;
use move_core_types::account_address::AccountAddress;
use crate::{add_edges, BlockPartitioner, get_anchor_shard_id};
use aptos_metrics_core::exponential_buckets;
use rayon::iter::ParallelIterator;
use rayon::{ThreadPool, ThreadPoolBuilder};

type Sender = Option<AccountAddress>;

pub struct OmegaPartitioner {
    thread_pool: ThreadPool,
}

impl OmegaPartitioner {
    pub fn new(num_threads: usize) -> Self {
        Self {
            thread_pool: ThreadPoolBuilder::new().num_threads(num_threads).build().unwrap()
        }
    }
}

impl BlockPartitioner for OmegaPartitioner {
    fn partition(&self, mut txns: Vec<AnalyzedTransaction>, num_executor_shards: usize) -> Vec<SubBlocksForShard<AnalyzedTransaction>> {
        let timer = OMEGA_PARTITIONER_MISC_TIMERS_SECONDS.with_label_values(&["preprocess"]).start_timer();
        let num_txns = txns.len();
        let mut num_senders = AtomicUsize::new(0);
        let mut num_keys = AtomicUsize::new(0);
        let shard_amount = std::env::var("OMEGA_PARTITIONER__DASHMAP_NUM_SHARDS").ok().map(|v|v.parse::<usize>().unwrap_or(256)).unwrap_or(256);
        let mut sender_ids_by_sender: DashMap<Sender, usize> = DashMap::with_shard_amount(shard_amount);
        let mut txn_counts_by_sender_id: DashMap<usize, AtomicUsize> = DashMap::with_shard_amount(shard_amount);
        let mut key_ids_by_key: DashMap<StateKey, usize> = DashMap::with_shard_amount(shard_amount);
        let mut helpers_by_key_id: DashMap<usize, RwLock<StorageLocationHelper>> = DashMap::with_shard_amount(shard_amount);
        for (txn_id, txn) in txns.iter_mut().enumerate() {
            txn.maybe_txn_id_in_partition_session = Some(txn_id);
        }
        txns.par_iter_mut().for_each(|mut txn| {
            let txn_id = *txn.maybe_txn_id_in_partition_session.as_ref().unwrap();
            let sender = txn.sender();
            let sender_id = *sender_ids_by_sender.entry(sender).or_insert_with(||{
                num_senders.fetch_add(1, Ordering::SeqCst)
            });
            txn_counts_by_sender_id.entry(sender_id).or_insert_with(|| AtomicUsize::new(0)).fetch_add(1, Ordering::SeqCst);
            txn.maybe_sender_id_in_partition_session = Some(sender_id);
            let num_writes = txn.write_hints.len();
            for (i, storage_location) in txn.write_hints.iter_mut().chain(txn.read_hints.iter_mut()).enumerate() {
                let key = storage_location.maybe_state_key().unwrap().clone();
                let key_id = *key_ids_by_key.entry(key).or_insert_with(||{
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
        let num_senders = num_senders.load(Ordering::SeqCst);
        let num_keys = num_keys.load(Ordering::SeqCst);
        let duration = timer.stop_and_record();
        println!("omega_par/preprocess={duration:?}");

        let mut remaining_txns = uniform_partition(num_txns, num_executor_shards);
        let mut start_txn_ids_by_shard_id = vec![0; num_executor_shards];
        {
            for shard_id in 1..num_executor_shards {
                start_txn_ids_by_shard_id[shard_id] = start_txn_ids_by_shard_id[shard_id - 1] + remaining_txns[shard_id - 1].len();
            }
            println!("start_txn_ids_by_shard_id={start_txn_ids_by_shard_id:?}");
        }

        let num_rounds: usize = 4;
        let mut txn_id_matrix: Vec<Vec<Vec<usize>>> = Vec::new();
        for round_id in 0..(num_rounds - 1) {
            let (accepted, discarded) = discarding_round_v2(&txns, remaining_txns, &helpers_by_key_id, &start_txn_ids_by_shard_id);
            txn_id_matrix.push(accepted);
            remaining_txns = discarded;
        }
        let last_round_txns: Vec<usize> = remaining_txns.into_iter().flatten().collect();
        remaining_txns = vec![vec![]; num_executor_shards];
        remaining_txns[num_executor_shards - 1] = last_round_txns;
        txn_id_matrix.push(remaining_txns);
        let num_actual_rounds = txn_id_matrix.len();

        let mut txns: Vec<Option<AnalyzedTransaction>> = txns.into_iter().map(|t| Some(t)).collect();
        let mut txn_matrix = vec![vec![vec![]; num_executor_shards]; num_actual_rounds];
        for (round_id, row) in txn_id_matrix.into_iter().enumerate() {
            for (shard_id, txn_ids) in row.into_iter().enumerate() {
                let sub_block: Vec<AnalyzedTransaction> = txn_ids.into_iter().map(|txn_id| txns[txn_id].take().unwrap()).collect();
                txn_matrix[round_id][shard_id] = sub_block;
            }
        }
        let ret = add_edges(num_executor_shards, txn_matrix, Some(num_keys));
        ret
    }
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

fn discarding_round_v2(
    txns: &Vec<AnalyzedTransaction>,
    txn_id_vecs: Vec<Vec<usize>>,
    loc_helpers: &DashMap<usize, RwLock<StorageLocationHelper>>,
    start_txn_id_by_shard_id: &Vec<usize>,
) -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
    let num_shards = txn_id_vecs.len();
    let mut accepted = vec![vec![]; num_shards];
    let mut discarded = vec![vec![]; num_shards];
    let min_discarded_seq_nums_by_sender_id: DashMap<usize, AtomicUsize> = DashMap::new();
    let acc_dsc_pairs: Vec<(Vec<usize>, Vec<usize>)> = txn_id_vecs.into_iter().enumerate().map(|(my_shard_id, txn_ids)|{
        let mut potentially_accepted: BTreeSet<usize> = BTreeSet::new();
        let mut discarded: BTreeSet<usize> = BTreeSet::new();
        for txn_id in txn_ids {
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
                discarded.insert(txn_id);
            } else {
                potentially_accepted.insert(txn_id);
            }
        }

        let mut accepted = vec![];
        for txn_id in potentially_accepted {
            let txn = txns.get(txn_id).unwrap();
            let sender_id = txn.maybe_sender_id_in_partition_session.unwrap();
            let min_discarded_txn_id = min_discarded_seq_nums_by_sender_id.entry(sender_id).or_insert_with(|| AtomicUsize::new(usize::MAX)).value().load(Ordering::SeqCst);
            if txn_id < min_discarded_txn_id {
                for loc in txn.write_hints.iter() {
                    let loc_id = *loc.maybe_id_in_partition_session.as_ref().unwrap();
                    loc_helpers.get(&loc_id).unwrap().write().unwrap().promote_txn_id(txn_id);
                }
                accepted.push(txn_id);
            } else {
                discarded.insert(txn_id);
            }
        }
        (accepted, discarded.into_iter().collect_vec())
    }).collect();

    for (shard_id, (acc, dsc)) in acc_dsc_pairs.into_iter().enumerate() {
        accepted[shard_id] = acc;
        discarded[shard_id] = dsc;
    }
    (accepted, discarded)
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
