// Copyright © Aptos Foundation

#[cfg(test)]
use crate::assert_deterministic_result;
#[cfg(test)]
use crate::test_utils::P2PBlockGenerator;
use crate::{
    get_anchor_shard_id,
    v2::{conflicting_txn_tracker::ConflictingTxnTracker, counters::MISC_TIMERS_SECONDS},
    BlockPartitioner, Sender,
};
use aptos_logger::info;
use aptos_types::{
    block_executor::partitioner::{
        CrossShardDependencies, RoundId, ShardId, ShardedTxnIndex, SubBlock, SubBlocksForShard,
        TransactionWithDependencies, TxnIndex,
    },
    state_store::state_key::StateKey,
    transaction::analyzed_transaction::AnalyzedTransaction,
};
use dashmap::DashMap;
#[cfg(test)]
use rand::thread_rng;
#[cfg(test)]
use rand::Rng;
use rayon::{
    iter::ParallelIterator,
    prelude::{IntoParallelIterator, IntoParallelRefIterator},
    ThreadPool, ThreadPoolBuilder,
};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::sync::Arc;
use std::{
    cmp,
    collections::HashSet,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, RwLock,
    },
};

/// The position of a txn in the block *before partitioning*.
type OriginalTxnIdx = usize;

/// Represent a specific storage location in a partitioning session.
type StorageKeyIdx = usize;

/// Represent a sender in a partitioning session.
type SenderIdx = usize;

/// Represent a txn after its position is finalized.
/// Different from `aptos_types::block_executor::partitioner::ShardedTxnIndex`,
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ShardedTxnIndex2 {
    pub round_id: RoundId,
    pub shard_id: ShardId,
    pub ori_txn_idx: OriginalTxnIdx,
}

impl Ord for ShardedTxnIndex2 {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        (self.round_id, self.shard_id, self.ori_txn_idx).cmp(&(
            other.round_id,
            other.shard_id,
            other.ori_txn_idx,
        ))
    }
}

impl PartialOrd for ShardedTxnIndex2 {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        (self.round_id, self.shard_id, self.ori_txn_idx).partial_cmp(&(
            other.round_id,
            other.shard_id,
            other.ori_txn_idx,
        ))
    }
}

impl ShardedTxnIndex2 {
    pub fn new(round_id: RoundId, shard_id: ShardId, pre_par_tid: OriginalTxnIdx) -> Self {
        Self {
            round_id,
            shard_id,
            ori_txn_idx: pre_par_tid,
        }
    }
}

mod conflicting_txn_tracker;
mod counters;

/// Basically `ShardedBlockPartitioner` but:
/// - Not pre-partitioned by txn sender.
/// - implemented more efficiently.
pub struct PartitionerV2 {
    thread_pool: ThreadPool,
    num_rounds_limit: usize,
    avoid_pct: u64,
    dashmap_num_shards: usize,
}

impl PartitionerV2 {
    pub fn new(
        num_threads: usize,
        num_rounds_limit: usize,
        avoid_pct: u64,
        dashmap_num_shards: usize,
    ) -> Self {
        Self {
            thread_pool: ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
                .unwrap(),
            num_rounds_limit,
            avoid_pct,
            dashmap_num_shards,
        }
    }

    /// Given some pre-partitioned txns, pull some off from each shard to avoid cross-shard conflict.
    /// The pulled off txns become the pre-partitioned txns for the next round.
    fn discarding_round(
        &self,
        round_id: RoundId,
        rsets: &[RwLock<HashSet<StorageKeyIdx>>],
        wsets: &[RwLock<HashSet<StorageKeyIdx>>],
        senders: &[RwLock<Option<SenderIdx>>],
        trackers: &DashMap<StorageKeyIdx, RwLock<ConflictingTxnTracker>>,
        start_txns: &[OriginalTxnIdx],
        remaining_txns: Vec<Vec<OriginalTxnIdx>>,
    ) -> (Vec<Vec<OriginalTxnIdx>>, Vec<Vec<OriginalTxnIdx>>) {
        let timer = MISC_TIMERS_SECONDS
            .with_label_values(&[format!("round_{round_id}__init").as_str()])
            .start_timer();
        let num_shards = remaining_txns.len();
        let mut discarded: Vec<RwLock<Vec<OriginalTxnIdx>>> = Vec::with_capacity(num_shards);
        let mut potentially_accepted: Vec<RwLock<Vec<OriginalTxnIdx>>> =
            Vec::with_capacity(num_shards);
        let mut finally_accepted: Vec<RwLock<Vec<OriginalTxnIdx>>> = Vec::with_capacity(num_shards);
        for txns in remaining_txns.iter() {
            potentially_accepted.push(RwLock::new(Vec::with_capacity(txns.len())));
            finally_accepted.push(RwLock::new(Vec::with_capacity(txns.len())));
            discarded.push(RwLock::new(Vec::with_capacity(txns.len())));
        }

        let min_discarded_by_sender: DashMap<SenderIdx, AtomicUsize> = DashMap::new();
        let duration = timer.stop_and_record();
        info!("round_{}__init={}", round_id, duration);

        let timer = MISC_TIMERS_SECONDS
            .with_label_values(&[format!("round_{round_id}__resolve_conflict").as_str()])
            .start_timer();
        self.thread_pool.install(|| {
            remaining_txns
                .into_iter()
                .enumerate()
                .collect::<Vec<_>>()
                .into_par_iter()
                .for_each(|(shard_id, txn_idxs)| {
                    txn_idxs.into_par_iter().for_each(|txn_idx| {
                        let in_round_conflict_detected = wsets[txn_idx]
                            .read()
                            .unwrap()
                            .iter()
                            .chain(rsets[txn_idx].read().unwrap().iter())
                            .any(|&key_idx| {
                                let tracker_ref = trackers.get(&key_idx).unwrap();
                                let tracker = tracker_ref.read().unwrap();
                                tracker.has_write_in_range(
                                    start_txns[tracker.anchor_shard_id],
                                    start_txns[shard_id],
                                )
                            });
                        if in_round_conflict_detected {
                            let sender = *senders[txn_idx].read().unwrap().as_ref().unwrap();
                            min_discarded_by_sender
                                .entry(sender)
                                .or_insert_with(|| AtomicUsize::new(usize::MAX))
                                .value()
                                .fetch_min(txn_idx, Ordering::SeqCst);
                            discarded[shard_id].write().unwrap().push(txn_idx);
                        } else {
                            potentially_accepted[shard_id]
                                .write()
                                .unwrap()
                                .push(txn_idx);
                        }
                    });
                });
        });
        let duration = timer.stop_and_record();
        info!("round_{}__resolve_conflict={}", round_id, duration);

        let timer = MISC_TIMERS_SECONDS
            .with_label_values(&[format!("round_{round_id}__keep_relative_order").as_str()])
            .start_timer();
        self.thread_pool.install(|| {
            (0..num_shards).into_par_iter().for_each(|shard_id| {
                potentially_accepted[shard_id]
                    .read()
                    .unwrap()
                    .par_iter()
                    .for_each(|&ori_txn_idx| {
                        let sender_id = *senders[ori_txn_idx].read().unwrap().as_ref().unwrap();
                        let min_discarded_txn_idx = min_discarded_by_sender
                            .entry(sender_id)
                            .or_insert_with(|| AtomicUsize::new(usize::MAX))
                            .load(Ordering::SeqCst);
                        if ori_txn_idx < min_discarded_txn_idx {
                            for &key_idx in wsets[ori_txn_idx]
                                .read()
                                .unwrap()
                                .iter()
                                .chain(rsets[ori_txn_idx].read().unwrap().iter())
                            {
                                trackers
                                    .get(&key_idx)
                                    .unwrap()
                                    .write()
                                    .unwrap()
                                    .mark_txn_ordered(ori_txn_idx, round_id, shard_id);
                            }
                            finally_accepted[shard_id]
                                .write()
                                .unwrap()
                                .push(ori_txn_idx);
                        } else {
                            discarded[shard_id].write().unwrap().push(ori_txn_idx);
                        }
                    });
            });
        });
        let duration = timer.stop_and_record();
        info!("round_{}__keep_relative_order={}", round_id, duration);

        let timer = MISC_TIMERS_SECONDS
            .with_label_values(&[format!("round_{round_id}__return_obj").as_str()])
            .start_timer();
        let ret = (
            extract_and_sort(finally_accepted),
            extract_and_sort(discarded),
        );
        let duration = timer.stop_and_record();
        info!("round_{}__return_obj={}", round_id, duration);
        self.thread_pool.spawn(move || {
            drop(potentially_accepted);
            drop(min_discarded_by_sender);
        });
        ret
    }

    fn add_edges(
        &self,
        txns: Vec<AnalyzedTransaction>,
        rsets: &[RwLock<HashSet<StorageKeyIdx>>],
        wsets: &[RwLock<HashSet<StorageKeyIdx>>],
        txn_id_matrix: &[Vec<Vec<OriginalTxnIdx>>],
        start_index_matrix: &[Vec<OriginalTxnIdx>],
        new_indices: &[RwLock<TxnIndex>],
        trackers: &DashMap<StorageKeyIdx, RwLock<ConflictingTxnTracker>>,
    ) -> Vec<SubBlocksForShard<AnalyzedTransaction>> {
        let timer = MISC_TIMERS_SECONDS
            .with_label_values(&["add_edges__init"])
            .start_timer();
        let txns: Vec<Mutex<Option<AnalyzedTransaction>>> = self
            .thread_pool
            .install(|| txns.into_par_iter().map(|t| Mutex::new(Some(t))).collect());
        let num_rounds = txn_id_matrix.len();
        let num_shards = txn_id_matrix.first().unwrap().len();
        let sub_block_matrix: Vec<Vec<Mutex<Option<SubBlock<AnalyzedTransaction>>>>> =
            self.thread_pool.install(|| {
                (0..num_rounds)
                    .into_par_iter()
                    .map(|_round_id| {
                        (0..num_shards)
                            .into_par_iter()
                            .map(|_shard_id| Mutex::new(None))
                            .collect()
                    })
                    .collect()
            });
        let duration = timer.stop_and_record();
        info!("add_edges__init={duration}");

        let timer = MISC_TIMERS_SECONDS
            .with_label_values(&["add_edges__main"])
            .start_timer();
        self.thread_pool.install(|| {
            (0..num_rounds).into_par_iter().for_each(|round_id| {
                (0..num_shards).into_par_iter().for_each(|shard_id| {
                    let cur_sub_block_size = txn_id_matrix[round_id][shard_id].len();
                    let mut twds: Vec<TransactionWithDependencies<AnalyzedTransaction>> =
                        Vec::with_capacity(cur_sub_block_size);
                    (0..cur_sub_block_size).for_each(|pos_in_sub_block| {
                        let ori_txn_idx = txn_id_matrix[round_id][shard_id][pos_in_sub_block];
                        let txn = txns[ori_txn_idx].lock().unwrap().take().unwrap();
                        let mut deps = CrossShardDependencies::default();
                        for &key_idx in wsets[ori_txn_idx]
                            .read()
                            .unwrap()
                            .iter()
                            .chain(rsets[ori_txn_idx].read().unwrap().iter())
                        {
                            let tracker_ref = trackers.get(&key_idx).unwrap();
                            let tracker = tracker_ref.read().unwrap();
                            if let Some(txn_idx) = tracker
                                .finalized_writes
                                .range(..ShardedTxnIndex2::new(round_id, shard_id, 0))
                                .last()
                            {
                                let src_txn_idx = ShardedTxnIndex {
                                    txn_index: *new_indices[txn_idx.ori_txn_idx].read().unwrap(),
                                    shard_id: txn_idx.shard_id,
                                    round_id: txn_idx.round_id,
                                };
                                deps.add_required_edge(
                                    src_txn_idx,
                                    tracker.storage_location.clone(),
                                );
                            }
                        }
                        for &key_idx in wsets[ori_txn_idx].read().unwrap().iter() {
                            let tracker_ref = trackers.get(&key_idx).unwrap();
                            let tracker = tracker_ref.read().unwrap();
                            let is_last_writer_in_cur_sub_block = tracker
                                .finalized_writes
                                .range(
                                    ShardedTxnIndex2::new(round_id, shard_id, ori_txn_idx + 1)
                                        ..ShardedTxnIndex2::new(round_id, shard_id + 1, 0),
                                )
                                .next()
                                .is_none();
                            if is_last_writer_in_cur_sub_block {
                                let mut end_idx = ShardedTxnIndex2::new(num_rounds, num_shards, 0); // Guaranteed to be invalid.
                                for follower_txn_idx in tracker
                                    .finalized_all
                                    .range(ShardedTxnIndex2::new(round_id, shard_id + 1, 0)..)
                                {
                                    if *follower_txn_idx > end_idx {
                                        break;
                                    }
                                    let dst_txn_idx = ShardedTxnIndex {
                                        txn_index: *new_indices[follower_txn_idx.ori_txn_idx]
                                            .read()
                                            .unwrap(),
                                        shard_id: follower_txn_idx.shard_id,
                                        round_id: follower_txn_idx.round_id,
                                    };
                                    deps.add_dependent_edge(dst_txn_idx, vec![tracker
                                        .storage_location
                                        .clone()]);
                                    if tracker.writer_set.contains(&follower_txn_idx.ori_txn_idx) {
                                        end_idx = ShardedTxnIndex2::new(
                                            follower_txn_idx.round_id,
                                            follower_txn_idx.shard_id + 1,
                                            0,
                                        );
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
        info!("add_edges__main={duration}");

        let timer = MISC_TIMERS_SECONDS
            .with_label_values(&["add_edges__return_obj"])
            .start_timer();
        let ret: Vec<SubBlocksForShard<AnalyzedTransaction>> = (0..num_shards)
            .map(|shard_id| {
                let sub_blocks: Vec<SubBlock<AnalyzedTransaction>> = (0..num_rounds)
                    .map(|round_id| {
                        sub_block_matrix[round_id][shard_id]
                            .lock()
                            .unwrap()
                            .take()
                            .unwrap()
                    })
                    .collect();
                SubBlocksForShard::new(shard_id, sub_blocks)
            })
            .collect();
        let duration = timer.stop_and_record();
        info!("add_edges__return_obj={duration}");

        self.thread_pool.spawn(move || {
            drop(sub_block_matrix);
            drop(txns);
        });
        ret
    }

    fn multi_rounds(
        &self,
        num_txns: usize,
        num_executor_shards: usize,
        rsets: &[RwLock<HashSet<StorageKeyIdx>>],
        wsets: &[RwLock<HashSet<StorageKeyIdx>>],
        senders: &[RwLock<Option<SenderIdx>>],
        trackers: &DashMap<StorageKeyIdx, RwLock<ConflictingTxnTracker>>,
        start_txns: &[OriginalTxnIdx],
        mut remaining_txns: Vec<Vec<OriginalTxnIdx>>,
    ) -> (
        Vec<Vec<Vec<OriginalTxnIdx>>>,
        Vec<Vec<OriginalTxnIdx>>,
        Vec<RwLock<TxnIndex>>,
    ) {
        let finalized_indexs: Vec<RwLock<TxnIndex>> =
            (0..num_txns).map(|_tid| RwLock::new(0)).collect();
        let mut txn_idx_matrix: Vec<Vec<Vec<OriginalTxnIdx>>> = Vec::new();
        let mut num_remaining_txns = usize::MAX;
        for round_id in 0..(self.num_rounds_limit - 1) {
            let timer = MISC_TIMERS_SECONDS
                .with_label_values(&[format!("round_{round_id}").as_str()])
                .start_timer();
            let (accepted, discarded) = self.discarding_round(
                round_id,
                rsets,
                wsets,
                senders,
                trackers,
                start_txns,
                remaining_txns,
            );
            txn_idx_matrix.push(accepted);
            remaining_txns = discarded;
            num_remaining_txns = remaining_txns.iter().map(|ts| ts.len()).sum();
            let duration = timer.stop_and_record();
            info!("round_{round_id}={duration}");

            if num_remaining_txns < self.avoid_pct as usize * num_txns / 100 {
                break;
            }
        }

        if num_remaining_txns >= 1 {
            let last_round_id = txn_idx_matrix.len();
            let timer = MISC_TIMERS_SECONDS
                .with_label_values(&["last_round"])
                .start_timer();
            let last_round_txns: Vec<OriginalTxnIdx> =
                remaining_txns.into_iter().flatten().collect();
            self.thread_pool.install(|| {
                last_round_txns.par_iter().for_each(|txn_idx_ref| {
                    let txn_idx = *txn_idx_ref;
                    for key_idx_ref in rsets[txn_idx]
                        .read()
                        .unwrap()
                        .iter()
                        .chain(wsets[txn_idx].read().unwrap().iter())
                    {
                        let key_idx = *key_idx_ref;
                        let tracker = trackers.get(&key_idx).unwrap();
                        tracker.write().unwrap().mark_txn_ordered(
                            txn_idx,
                            last_round_id,
                            num_executor_shards - 1,
                        );
                    }
                });
            });

            remaining_txns = vec![vec![]; num_executor_shards];
            remaining_txns[num_executor_shards - 1] = last_round_txns;
            txn_idx_matrix.push(remaining_txns);
            let duration = timer.stop_and_record();
            info!("last_round={duration}");
        }

        let timer = MISC_TIMERS_SECONDS
            .with_label_values(&["new_tid_table"])
            .start_timer();
        let num_rounds = txn_idx_matrix.len();
        let mut start_index_matrix: Vec<Vec<TxnIndex>> =
            vec![vec![0; num_executor_shards]; num_rounds];
        let mut global_counter: TxnIndex = 0;
        for (round_id, row) in txn_idx_matrix.iter().enumerate() {
            for (shard_id, txns) in row.iter().enumerate() {
                start_index_matrix[round_id][shard_id] = global_counter;
                global_counter += txns.len();
            }
        }

        self.thread_pool.install(|| {
            (0..num_rounds).into_par_iter().for_each(|round_id| {
                (0..num_executor_shards)
                    .into_par_iter()
                    .for_each(|shard_id| {
                        let sub_block_size = txn_idx_matrix[round_id][shard_id].len();
                        (0..sub_block_size)
                            .into_par_iter()
                            .for_each(|pos_in_sub_block| {
                                let txn_idx = txn_idx_matrix[round_id][shard_id][pos_in_sub_block];
                                *finalized_indexs[txn_idx].write().unwrap() =
                                    start_index_matrix[round_id][shard_id] + pos_in_sub_block;
                            });
                    });
            });
        });
        let duration = timer.stop_and_record();
        info!("new_tid_table={duration}");
        (txn_idx_matrix, start_index_matrix, finalized_indexs)
    }
}

impl BlockPartitioner for PartitionerV2 {
    fn partition(
        &self,
        txns: Vec<AnalyzedTransaction>,
        num_executor_shards: usize,
    ) -> Vec<SubBlocksForShard<AnalyzedTransaction>> {
        let timer = MISC_TIMERS_SECONDS
            .with_label_values(&["preprocess"])
            .start_timer();
        let num_txns = txns.len();
        let num_senders = AtomicUsize::new(0);
        let num_keys = AtomicUsize::new(0);
        let mut senders: Vec<RwLock<Option<SenderIdx>>> = Vec::with_capacity(num_txns);
        let mut wsets: Vec<RwLock<HashSet<StorageKeyIdx>>> = Vec::with_capacity(num_txns);
        let mut rsets: Vec<RwLock<HashSet<StorageKeyIdx>>> = Vec::with_capacity(num_txns);
        let sender_idx_table: DashMap<Sender, SenderIdx> =
            DashMap::with_shard_amount(self.dashmap_num_shards);
        let key_idx_table: DashMap<StateKey, StorageKeyIdx> =
            DashMap::with_shard_amount(self.dashmap_num_shards);
        let trackers: DashMap<StorageKeyIdx, RwLock<ConflictingTxnTracker>> =
            DashMap::with_shard_amount(self.dashmap_num_shards);
        for txn in txns.iter() {
            senders.push(RwLock::new(None));
            wsets.push(RwLock::new(HashSet::with_capacity(txn.write_hints().len())));
            rsets.push(RwLock::new(HashSet::with_capacity(txn.read_hints().len())));
        }
        let timer_1 = MISC_TIMERS_SECONDS
            .with_label_values(&["preprocess__main"])
            .start_timer();
        self.thread_pool.install(|| {
            (0..num_txns)
                .into_par_iter()
                .for_each(|txn_idx: OriginalTxnIdx| {
                    let txn = &txns[txn_idx];
                    let sender = txn.sender();
                    let sender_idx = *sender_idx_table
                        .entry(sender)
                        .or_insert_with(|| num_senders.fetch_add(1, Ordering::SeqCst));
                    *senders[txn_idx].write().unwrap() = Some(sender_idx);
                    let num_writes = txn.write_hints().len();
                    for (i, storage_location) in txn
                        .write_hints()
                        .iter()
                        .chain(txn.read_hints().iter())
                        .enumerate()
                    {
                        let key = storage_location.state_key().clone();
                        let key_idx = *key_idx_table
                            .entry(key)
                            .or_insert_with(|| num_keys.fetch_add(1, Ordering::SeqCst));
                        let is_write = i < num_writes;
                        if is_write {
                            wsets[txn_idx].write().unwrap().insert(key_idx);
                        } else {
                            rsets[txn_idx].write().unwrap().insert(key_idx);
                        }
                        trackers
                            .entry(key_idx)
                            .or_insert_with(|| {
                                let anchor_shard_id =
                                    get_anchor_shard_id(storage_location, num_executor_shards);
                                RwLock::new(ConflictingTxnTracker::new(
                                    storage_location.clone(),
                                    anchor_shard_id,
                                ))
                            })
                            .write()
                            .unwrap()
                            .add_candidate(txn_idx, is_write);
                    }
                });
        });
        let duration_1 = timer_1.stop_and_record();
        info!("preprocess__main={duration_1}");
        let duration = timer.stop_and_record();
        info!("preprocess={duration}");

        let timer = MISC_TIMERS_SECONDS
            .with_label_values(&["pre_partition_uniform"])
            .start_timer();
        let remaining_txn_idxs = uniform_partition(num_txns, num_executor_shards);
        let mut start_txns: Vec<OriginalTxnIdx> = vec![0; num_executor_shards];
        for shard_id in 1..num_executor_shards {
            start_txns[shard_id] =
                start_txns[shard_id - 1] + remaining_txn_idxs[shard_id - 1].len();
        }
        let duration = timer.stop_and_record();
        info!("pre_partition_uniform={duration}");

        let timer = MISC_TIMERS_SECONDS
            .with_label_values(&["multi_rounds"])
            .start_timer();
        let (finalized_txn_matrix, start_index_matrix, new_idxs) = self.multi_rounds(
            num_txns,
            num_executor_shards,
            &rsets,
            &wsets,
            &senders,
            &trackers,
            &start_txns,
            remaining_txn_idxs,
        );
        let duration = timer.stop_and_record();
        info!("multi_rounds={duration}");

        let timer = MISC_TIMERS_SECONDS
            .with_label_values(&["add_edges"])
            .start_timer();
        let ret = self.add_edges(
            txns,
            &rsets,
            &wsets,
            &finalized_txn_matrix,
            &start_index_matrix,
            &new_idxs,
            &trackers,
        );
        let duration = timer.stop_and_record();
        info!("add_edges={duration}");
        let timer = MISC_TIMERS_SECONDS
            .with_label_values(&["drop"])
            .start_timer();
        self.thread_pool.spawn(move || {
            drop(sender_idx_table);
            drop(senders);
            drop(wsets);
            drop(rsets);
            drop(trackers);
            drop(key_idx_table);
            drop(start_txns);
            drop(finalized_txn_matrix);
            drop(start_index_matrix);
            drop(new_idxs);
        });
        let duration = timer.stop_and_record();
        info!("drop={duration}");
        ret
    }
}

#[test]
fn test_partitioner_v2_correctness() {
    let block_generator = P2PBlockGenerator::new(100);
    let partitioner = PartitionerV2::new(8, 4, 10, 64);
    let mut rng = thread_rng();
    for _run_id in 0..100 {
        let block_size = 10_u64.pow(rng.gen_range(0, 4)) as usize;
        let num_shards = rng.gen_range(1, 10);
        let block = block_generator.rand_block(&mut rng, block_size);
        let block_clone = block.clone();
        let partitioned = partitioner.partition(block, num_shards);
        crate::verify_partitioner_output(&block_clone, &partitioned);
    }
}

#[test]
fn test_partitioner_v2_determinism() {
    let partitioner = Arc::new(PartitionerV2::new(4, 4, 10, 64));
    assert_deterministic_result(partitioner);
}

/// Evenly divide 0..n-1. Example: uniform_partition(11,3) == [[0,1,2,3],[4,5,6,7],[8,9,10]]
fn uniform_partition(num_items: usize, num_chunks: usize) -> Vec<Vec<OriginalTxnIdx>> {
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
    assert_eq!(
        vec![4, 4, 4, 3, 3],
        actual.iter().map(|v| v.len()).collect::<Vec<usize>>()
    );
    assert_eq!((0..18).collect::<Vec<usize>>(), actual.concat());

    let actual = uniform_partition(18, 3);
    assert_eq!(
        vec![6, 6, 6],
        actual.iter().map(|v| v.len()).collect::<Vec<usize>>()
    );
    assert_eq!((0..18).collect::<Vec<usize>>(), actual.concat());
}

fn extract_and_sort(arr_2d: Vec<RwLock<Vec<usize>>>) -> Vec<Vec<usize>> {
    arr_2d
        .into_iter()
        .map(|arr_1d| {
            let mut arr_1d_guard = arr_1d.write().unwrap();
            let mut arr_1d_value = std::mem::take(&mut *arr_1d_guard);
            arr_1d_value.sort();
            arr_1d_value
        })
        .collect::<Vec<_>>()
}
