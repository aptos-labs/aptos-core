// Copyright © Aptos Foundation

#[cfg(test)]
use crate::test_utils::assert_deterministic_result;
#[cfg(test)]
use crate::test_utils::P2PBlockGenerator;
use crate::{
    get_anchor_shard_id,
    v2::{conflicting_txn_tracker::ConflictingTxnTracker, counters::MISC_TIMERS_SECONDS},
    BlockPartitioner, Sender,
};
use aptos_crypto::hash::CryptoHash;
use aptos_logger::{info, trace};
use aptos_types::{
    block_executor::partitioner::{
        CrossShardDependencies, PartitionedTransactions, RoundId, ShardId, ShardedTxnIndex,
        SubBlock, SubBlocksForShard, TransactionWithDependencies, TxnIndex, GLOBAL_ROUND_ID,
        GLOBAL_SHARD_ID,
    },
    state_store::state_key::StateKey,
    transaction::analyzed_transaction::{AnalyzedTransaction, StorageLocation},
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
use std::{
    cmp,
    collections::{btree_set::Range, HashSet},
    iter::Chain,
    mem,
    mem::swap,
    slice::Iter,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
};
use types::{OriginalTxnIdx, SenderIdx, ShardedTxnIndex2, StorageKeyIdx, SubBlockIdx};

pub mod config;
mod conflicting_txn_tracker;
mod counters;
pub mod types;

/// Basically `ShardedBlockPartitioner` but:
/// - Not pre-partitioned by txn sender.
/// - implemented more efficiently.
pub struct PartitionerV2 {
    thread_pool: Arc<ThreadPool>,
    num_rounds_limit: usize,
    avoid_pct: u64,
    dashmap_num_shards: usize,
    merge_discarded: bool,
}

impl PartitionerV2 {
    pub fn new(
        num_threads: usize,
        num_rounds_limit: usize,
        avoid_pct: u64,
        dashmap_num_shards: usize,
        merge_discarded: bool,
    ) -> Self {
        info!("Creating a PartitionerV2 instance with num_threads={num_threads}, num_rounds_limit={num_rounds_limit}, avoid_pct={avoid_pct}, dashmap_num_shards={dashmap_num_shards}, merge_discarded={merge_discarded}");
        let thread_pool = Arc::new(
            ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
                .unwrap(),
        );
        Self {
            thread_pool,
            num_rounds_limit,
            avoid_pct,
            dashmap_num_shards,
            merge_discarded,
        }
    }
}

struct WorkSession {
    thread_pool: Arc<ThreadPool>,
    num_executor_shards: ShardId,
    txns: Vec<AnalyzedTransaction>,
    pre_partitioned: Vec<Vec<OriginalTxnIdx>>,
    start_txn_idxs_by_shard: Vec<OriginalTxnIdx>,
    sender_counter: AtomicUsize,
    key_counter: AtomicUsize,
    senders: Vec<RwLock<Option<SenderIdx>>>,
    wsets: Vec<RwLock<HashSet<StorageKeyIdx>>>,
    rsets: Vec<RwLock<HashSet<StorageKeyIdx>>>,
    sender_idx_table: DashMap<Sender, SenderIdx>,
    key_idx_table: DashMap<StateKey, StorageKeyIdx>,
    trackers: DashMap<StorageKeyIdx, RwLock<ConflictingTxnTracker>>,
    min_discards_by_sender: DashMap<SenderIdx, AtomicUsize>,
    num_rounds_limit: usize,
    avoid_pct: u64,
    merge_discarded: bool,
    finalized_txn_matrix: Vec<Vec<Vec<OriginalTxnIdx>>>,
    start_index_matrix: Vec<Vec<OriginalTxnIdx>>,
    new_txn_idxs: Vec<RwLock<TxnIndex>>,
}

fn start_txn_idxs(pre_partitioned: &Vec<Vec<OriginalTxnIdx>>) -> Vec<OriginalTxnIdx> {
    let num_shards = pre_partitioned.len();
    let mut ret: Vec<OriginalTxnIdx> = vec![0; num_shards];
    for shard_id in 1..num_shards {
        ret[shard_id] = ret[shard_id - 1] + pre_partitioned[shard_id - 1].len();
    }
    ret
}

impl WorkSession {
    fn new(
        thread_pool: Arc<ThreadPool>,
        dashmap_num_shards: usize,
        txns: Vec<AnalyzedTransaction>,
        num_executor_shards: ShardId,
        pre_partitioned: Vec<Vec<OriginalTxnIdx>>,
        num_rounds_limit: usize,
        avoid_pct: u64,
        merge_discarded: bool,
    ) -> Self {
        let num_txns = txns.len();
        let sender_counter = AtomicUsize::new(0);
        let key_counter = AtomicUsize::new(0);
        let mut senders: Vec<RwLock<Option<SenderIdx>>> = Vec::with_capacity(num_txns);
        let mut wsets: Vec<RwLock<HashSet<StorageKeyIdx>>> = Vec::with_capacity(num_txns);
        let mut rsets: Vec<RwLock<HashSet<StorageKeyIdx>>> = Vec::with_capacity(num_txns);
        let sender_idx_table: DashMap<Sender, SenderIdx> =
            DashMap::with_shard_amount(dashmap_num_shards);
        let key_idx_table: DashMap<StateKey, StorageKeyIdx> =
            DashMap::with_shard_amount(dashmap_num_shards);
        let trackers: DashMap<StorageKeyIdx, RwLock<ConflictingTxnTracker>> =
            DashMap::with_shard_amount(dashmap_num_shards);
        for txn in txns.iter() {
            senders.push(RwLock::new(None));
            wsets.push(RwLock::new(HashSet::with_capacity(txn.write_hints().len())));
            rsets.push(RwLock::new(HashSet::with_capacity(txn.read_hints().len())));
        }
        let start_txn_idxs_by_shard = start_txn_idxs(&pre_partitioned);
        Self {
            merge_discarded,
            thread_pool,
            num_executor_shards,
            txns,
            pre_partitioned,
            start_txn_idxs_by_shard,
            sender_counter,
            key_counter,
            senders,
            wsets,
            rsets,
            sender_idx_table,
            key_idx_table,
            trackers,
            min_discards_by_sender: DashMap::new(),
            avoid_pct,
            num_rounds_limit,
            finalized_txn_matrix: Vec::with_capacity(num_rounds_limit),
            new_txn_idxs: Vec::with_capacity(num_txns),
            start_index_matrix: Vec::with_capacity(num_rounds_limit),
        }
    }

    fn num_txns(&self) -> usize {
        self.txns.len()
    }

    fn add_key(&self, key: &StateKey) -> StorageKeyIdx {
        *self
            .key_idx_table
            .entry(key.clone())
            .or_insert_with(|| self.key_counter.fetch_add(1, Ordering::SeqCst))
    }

    fn num_keys(&self) -> usize {
        self.key_counter.load(Ordering::SeqCst)
    }

    fn storage_location(&self, key_idx: StorageKeyIdx) -> StorageLocation {
        let tracker_ref = self.trackers.get(&key_idx).unwrap();
        let tracker = tracker_ref.read().unwrap();
        tracker.storage_location.clone()
    }

    fn sender_idx(&self, txn_idx: OriginalTxnIdx) -> SenderIdx {
        *self.senders[txn_idx].read().unwrap().as_ref().unwrap()
    }

    fn add_sender(&self, sender: Sender) -> SenderIdx {
        *self
            .sender_idx_table
            .entry(sender)
            .or_insert_with(|| self.sender_counter.fetch_add(1, Ordering::SeqCst))
    }

    fn shard_is_currently_follower_for_key(&self, shard_id: ShardId, key: StorageKeyIdx) -> bool {
        let tracker_ref = self.trackers.get(&key).unwrap();
        let tracker = tracker_ref.read().unwrap();
        let range_start = self.start_txn_idxs_by_shard[tracker.anchor_shard_id];
        let range_end = self.start_txn_idxs_by_shard[shard_id];
        tracker.has_write_in_range(range_start, range_end)
    }

    fn all_hints(&self, txn_idx: OriginalTxnIdx) -> Vec<StorageKeyIdx> {
        let wset = self.wsets[txn_idx].read().unwrap();
        let rset = self.rsets[txn_idx].read().unwrap();
        let all: Vec<StorageKeyIdx> = wset.iter().chain(rset.iter()).copied().collect();
        all
    }

    fn write_hints(&self, txn_idx: OriginalTxnIdx) -> Vec<StorageKeyIdx> {
        self.wsets[txn_idx]
            .read()
            .unwrap()
            .iter()
            .copied()
            .collect()
    }

    fn build_index(&self) {
        self.thread_pool.install(|| {
            (0..self.num_txns())
                .into_par_iter()
                .for_each(|txn_idx: OriginalTxnIdx| {
                    let txn = &self.txns[txn_idx];
                    let sender_idx = self.add_sender(txn.sender());
                    *self.senders[txn_idx].write().unwrap() = Some(sender_idx);

                    let reads = txn.read_hints.iter().map(|loc| (loc, false));
                    let writes = txn.write_hints.iter().map(|loc| (loc, true));
                    reads
                        .chain(writes)
                        .for_each(|(storage_location, is_write)| {
                            let key_idx = self.add_key(storage_location.state_key());
                            if is_write {
                                self.wsets[txn_idx].write().unwrap().insert(key_idx);
                            } else {
                                self.rsets[txn_idx].write().unwrap().insert(key_idx);
                            }
                            let tracker_ref = self.trackers.entry(key_idx).or_insert_with(|| {
                                let anchor_shard_id =
                                    get_anchor_shard_id(storage_location, self.num_executor_shards);
                                RwLock::new(ConflictingTxnTracker::new(
                                    storage_location.clone(),
                                    anchor_shard_id,
                                ))
                            });
                            let mut tracker = tracker_ref.write().unwrap();
                            if is_write {
                                tracker.add_write_candidate(txn_idx);
                            } else {
                                tracker.add_read_candidate(txn_idx);
                            }
                        });
                });
        });
    }

    fn min_discard(&self, sender: SenderIdx) -> Option<OriginalTxnIdx> {
        self.min_discards_by_sender
            .get(&sender)
            .as_ref()
            .map(|r| r.value().load(Ordering::SeqCst))
    }

    fn update_min_discarded_txn_idx(&self, sender: SenderIdx, txn_idx: OriginalTxnIdx) {
        self.min_discards_by_sender
            .entry(sender)
            .or_insert_with(|| AtomicUsize::new(usize::MAX))
            .value()
            .fetch_min(txn_idx, Ordering::SeqCst);
    }

    fn update_trackers_on_accepting(
        &self,
        ori_txn_idx: OriginalTxnIdx,
        round_id: RoundId,
        shard_id: ShardId,
    ) {
        for key_idx in self.all_hints(ori_txn_idx) {
            self.trackers
                .get(&key_idx)
                .unwrap()
                .write()
                .unwrap()
                .mark_txn_ordered(ori_txn_idx, round_id, shard_id);
        }
    }

    fn build_new_index_tables(
        &self,
        accepted_txn_matrix: &Vec<Vec<Vec<OriginalTxnIdx>>>,
    ) -> (Vec<Vec<TxnIndex>>, Vec<RwLock<TxnIndex>>) {
        let num_rounds = accepted_txn_matrix.len();
        let mut start_index_matrix: Vec<Vec<TxnIndex>> =
            vec![vec![0; self.num_executor_shards]; num_rounds];
        let mut global_counter: TxnIndex = 0;
        for (round_id, row) in accepted_txn_matrix.iter().enumerate() {
            for (shard_id, txns) in row.iter().enumerate() {
                start_index_matrix[round_id][shard_id] = global_counter;
                global_counter += txns.len();
            }
        }

        let finalized_indexs: Vec<RwLock<TxnIndex>> =
            (0..self.num_txns()).map(|_tid| RwLock::new(0)).collect();

        self.thread_pool.install(|| {
            (0..num_rounds).into_par_iter().for_each(|round_id| {
                (0..self.num_executor_shards)
                    .into_par_iter()
                    .for_each(|shard_id| {
                        let sub_block_size = accepted_txn_matrix[round_id][shard_id].len();
                        (0..sub_block_size)
                            .into_par_iter()
                            .for_each(|pos_in_sub_block| {
                                let txn_idx =
                                    accepted_txn_matrix[round_id][shard_id][pos_in_sub_block];
                                *finalized_indexs[txn_idx].write().unwrap() =
                                    start_index_matrix[round_id][shard_id] + pos_in_sub_block;
                            });
                    });
            });
        });

        (start_index_matrix, finalized_indexs)
    }

    /// Given some pre-partitioned txns, pull some off from each shard to avoid cross-shard conflict.
    /// The pulled off txns become the pre-partitioned txns for the next round.
    fn discarding_round(
        &mut self,
        round_id: RoundId,
        remaining_txns: Vec<Vec<OriginalTxnIdx>>,
    ) -> (Vec<Vec<OriginalTxnIdx>>, Vec<Vec<OriginalTxnIdx>>) {
        let timer = MISC_TIMERS_SECONDS
            .with_label_values(&[format!("multi_rounds__round_{round_id}__init").as_str()])
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

        self.min_discards_by_sender = DashMap::new();
        let _duration = timer.stop_and_record();

        self.thread_pool.install(|| {
            (0..self.num_executor_shards)
                .into_par_iter()
                .for_each(|shard_id| {
                    remaining_txns[shard_id].par_iter().for_each(|&txn_idx| {
                        let in_round_conflict_detected =
                            self.all_hints(txn_idx).iter().any(|&key_idx| {
                                self.shard_is_currently_follower_for_key(shard_id, key_idx)
                            });
                        if in_round_conflict_detected {
                            let sender = self.sender_idx(txn_idx);
                            self.update_min_discarded_txn_idx(sender, txn_idx);
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

        self.thread_pool.install(|| {
            (0..num_shards).into_par_iter().for_each(|shard_id| {
                potentially_accepted[shard_id]
                    .read()
                    .unwrap()
                    .par_iter()
                    .for_each(|&ori_txn_idx| {
                        let sender_idx = self.sender_idx(ori_txn_idx);
                        if ori_txn_idx < self.min_discard(sender_idx).unwrap_or(OriginalTxnIdx::MAX)
                        {
                            self.update_trackers_on_accepting(ori_txn_idx, round_id, shard_id);
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

        let ret = (
            extract_and_sort(finally_accepted),
            extract_and_sort(discarded),
        );
        let min_discards_by_sender = mem::take(&mut self.min_discards_by_sender);
        self.thread_pool.spawn(move || {
            drop(remaining_txns);
            drop(potentially_accepted);
            drop(min_discards_by_sender);
        });
        ret
    }

    fn flatten_to_rounds(&mut self) {
        let mut remaining_txns = mem::take(&mut self.pre_partitioned);
        assert_eq!(self.num_executor_shards, remaining_txns.len());

        let mut num_remaining_txns = usize::MAX;
        for round_id in 0..(self.num_rounds_limit - 1) {
            let timer = MISC_TIMERS_SECONDS
                .with_label_values(&[format!("multi_rounds__round_{round_id}").as_str()])
                .start_timer();
            let (accepted, discarded) = self.discarding_round(round_id, remaining_txns);
            self.finalized_txn_matrix.push(accepted);
            remaining_txns = discarded;
            num_remaining_txns = remaining_txns.iter().map(|ts| ts.len()).sum();
            let _duration = timer.stop_and_record();

            if num_remaining_txns < self.avoid_pct as usize * self.num_txns() / 100 {
                break;
            }
        }

        if self.merge_discarded {
            trace!("Merging txns after discarding stopped.");
            let last_round_txns: Vec<OriginalTxnIdx> =
                remaining_txns.into_iter().flatten().collect();
            remaining_txns = vec![vec![]; self.num_executor_shards];
            remaining_txns[self.num_executor_shards - 1] = last_round_txns;
        }

        let last_round_id = self.finalized_txn_matrix.len();
        self.thread_pool.install(|| {
            (0..self.num_executor_shards)
                .into_par_iter()
                .for_each(|shard_id| {
                    remaining_txns[shard_id]
                        .par_iter()
                        .for_each(|&ori_txn_idx| {
                            self.update_trackers_on_accepting(ori_txn_idx, last_round_id, shard_id);
                        });
                });
        });
        self.finalized_txn_matrix.push(remaining_txns);

        (self.start_index_matrix, self.new_txn_idxs) =
            self.build_new_index_tables(&self.finalized_txn_matrix);
    }

    fn last_writer(&self, key: StorageKeyIdx, sub_block: SubBlockIdx) -> Option<OriginalTxnIdx> {
        let tracker_ref = self.trackers.get(&key).unwrap();
        let tracker = tracker_ref.read().unwrap();
        let start = ShardedTxnIndex2::new(sub_block.round_id, sub_block.shard_id, 0);
        let end = ShardedTxnIndex2::new(sub_block.round_id, sub_block.shard_id + 1, 0);
        let ret = tracker
            .finalized_writes
            .range(start..end)
            .last()
            .map(|t| t.ori_txn_idx);
        ret
    }

    fn first_writer(
        &self,
        key: StorageKeyIdx,
        since: ShardedTxnIndex2,
    ) -> Option<ShardedTxnIndex2> {
        let tracker_ref = self.trackers.get(&key).unwrap();
        let tracker = tracker_ref.read().unwrap();
        let ret = tracker.finalized_writes.range(since..).next().copied();
        ret
    }

    fn all_accepted_txns(
        &self,
        key: StorageKeyIdx,
        start: ShardedTxnIndex2,
        end: ShardedTxnIndex2,
    ) -> Vec<ShardedTxnIndex2> {
        let tracker_ref = self.trackers.get(&key).unwrap();
        let tracker = tracker_ref.read().unwrap();
        let ret = tracker.finalized_all.range(start..end).copied().collect();
        ret
    }

    fn num_rounds(&self) -> usize {
        self.finalized_txn_matrix.len()
    }

    fn actual_sub_block_idx(&self, sub_blk_idx: SubBlockIdx) -> SubBlockIdx {
        if self.merge_discarded {
            if sub_blk_idx.round_id == self.num_rounds() - 1 {
                SubBlockIdx::global()
            } else {
                sub_blk_idx
            }
        } else {
            sub_blk_idx
        }
    }

    fn add_edges(&mut self) -> PartitionedTransactions {
        let txns: Vec<Mutex<Option<AnalyzedTransaction>>> = self.thread_pool.install(|| {
            mem::take(&mut self.txns)
                .into_par_iter()
                .map(|t| Mutex::new(Some(t)))
                .collect()
        });
        let actual_sub_block_position = |round_id: usize, shard_id: usize| -> (usize, usize) {
            if self.merge_discarded {
                if round_id == self.num_rounds() - 1 {
                    (GLOBAL_ROUND_ID, GLOBAL_SHARD_ID)
                } else {
                    (round_id, shard_id)
                }
            } else {
                (round_id, shard_id)
            }
        };

        let mut sub_block_matrix: Vec<Vec<Mutex<Option<SubBlock<AnalyzedTransaction>>>>> =
            self.thread_pool.install(|| {
                (0..self.num_rounds())
                    .into_par_iter()
                    .map(|_round_id| {
                        (0..self.num_executor_shards)
                            .into_par_iter()
                            .map(|_shard_id| Mutex::new(None))
                            .collect()
                    })
                    .collect()
            });

        self.thread_pool.install(|| {
            (0..self.num_rounds()).into_par_iter().for_each(|round_id| {
                (0..self.num_executor_shards)
                    .into_par_iter()
                    .for_each(|shard_id| {
                        let cur_sub_block_size =
                            self.finalized_txn_matrix[round_id][shard_id].len();
                        let mut twds: Vec<TransactionWithDependencies<AnalyzedTransaction>> =
                            Vec::with_capacity(cur_sub_block_size);
                        (0..cur_sub_block_size).for_each(|pos_in_sub_block| {
                            let ori_txn_idx =
                                self.finalized_txn_matrix[round_id][shard_id][pos_in_sub_block];
                            let txn = txns[ori_txn_idx].lock().unwrap().take().unwrap();
                            let mut deps = CrossShardDependencies::default();
                            for key_idx in self.all_hints(ori_txn_idx) {
                                let tracker_ref = self.trackers.get(&key_idx).unwrap();
                                let tracker = tracker_ref.read().unwrap();
                                if let Some(txn_idx) = tracker
                                    .finalized_writes
                                    .range(..ShardedTxnIndex2::new(round_id, shard_id, 0))
                                    .last()
                                {
                                    let src_txn_idx = ShardedTxnIndex {
                                        txn_index: *self.new_txn_idxs[txn_idx.ori_txn_idx]
                                            .read()
                                            .unwrap(),
                                        shard_id: txn_idx.sub_block_idx.shard_id,
                                        round_id: txn_idx.sub_block_idx.round_id,
                                    };
                                    deps.add_required_edge(
                                        src_txn_idx,
                                        tracker.storage_location.clone(),
                                    );
                                }
                            }
                            for key_idx in self.write_hints(ori_txn_idx) {
                                if Some(ori_txn_idx)
                                    == self.last_writer(key_idx, SubBlockIdx { round_id, shard_id })
                                {
                                    let start_of_next_sub_block =
                                        ShardedTxnIndex2::new(round_id, shard_id + 1, 0);
                                    let next_writer =
                                        self.first_writer(key_idx, start_of_next_sub_block);
                                    let end_follower = match next_writer {
                                        None => ShardedTxnIndex2::new(
                                            self.num_rounds(),
                                            self.num_executor_shards,
                                            0,
                                        ), // Guaranteed to be greater than any invalid idx...
                                        Some(idx) => ShardedTxnIndex2::new(
                                            idx.round_id(),
                                            idx.shard_id() + 1,
                                            0,
                                        ),
                                    };
                                    for follower_txn_idx in self.all_accepted_txns(
                                        key_idx,
                                        start_of_next_sub_block,
                                        end_follower,
                                    ) {
                                        let actual_sub_blk_idx = self
                                            .actual_sub_block_idx(follower_txn_idx.sub_block_idx);
                                        let dst_txn_idx = ShardedTxnIndex {
                                            txn_index: *self.new_txn_idxs
                                                [follower_txn_idx.ori_txn_idx]
                                                .read()
                                                .unwrap(),
                                            shard_id: actual_sub_blk_idx.shard_id,
                                            round_id: actual_sub_blk_idx.round_id,
                                        };
                                        deps.add_dependent_edge(dst_txn_idx, vec![
                                            self.storage_location(key_idx)
                                        ]);
                                    }
                                }
                            }
                            let twd = TransactionWithDependencies::new(txn, deps);
                            twds.push(twd);
                        });
                        let sub_block =
                            SubBlock::new(self.start_index_matrix[round_id][shard_id], twds);
                        *sub_block_matrix[round_id][shard_id].lock().unwrap() = Some(sub_block);
                    });
            });
        });

        let global_txns: Vec<TransactionWithDependencies<AnalyzedTransaction>> =
            if self.merge_discarded {
                sub_block_matrix
                    .pop()
                    .unwrap()
                    .last()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .take()
                    .unwrap()
                    .into_transactions_with_deps()
            } else {
                vec![]
            };

        let num_rounds = sub_block_matrix.len();
        let sharded_txns: Vec<SubBlocksForShard<AnalyzedTransaction>> = (0..self
            .num_executor_shards)
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
        let ret = PartitionedTransactions::new(sharded_txns, global_txns);

        self.thread_pool.spawn(move || {
            drop(sub_block_matrix);
            drop(txns);
        });
        ret
    }

    fn run(&mut self) -> PartitionedTransactions {
        self.build_index();
        self.flatten_to_rounds();
        self.add_edges()
    }
}
impl BlockPartitioner for PartitionerV2 {
    fn partition(
        &self,
        txns: Vec<AnalyzedTransaction>,
        num_executor_shards: usize,
    ) -> PartitionedTransactions {
        let num_txns = txns.len();
        let pre_partitioned = uniform_partition(num_txns, num_executor_shards);
        let mut session = WorkSession::new(
            self.thread_pool.clone(),
            self.dashmap_num_shards,
            txns,
            num_executor_shards,
            pre_partitioned,
            self.num_rounds_limit,
            self.avoid_pct,
            self.merge_discarded,
        );
        let ret = session.run();
        self.thread_pool.spawn(move || {
            drop(session);
        });
        ret
    }
}

#[test]
fn test_partitioner_v2_correctness() {
    for merge_discarded in [false, true] {
        let block_generator = P2PBlockGenerator::new(100);
        let partitioner = PartitionerV2::new(8, 4, 10, 64, merge_discarded);
        let mut rng = thread_rng();
        for _run_id in 0..20 {
            let block_size = 10_u64.pow(rng.gen_range(0, 4)) as usize;
            let num_shards = rng.gen_range(1, 10);
            let block = block_generator.rand_block(&mut rng, block_size);
            let block_clone = block.clone();
            let partitioned = partitioner.partition(block, num_shards);
            crate::test_utils::verify_partitioner_output(&block_clone, &partitioned);
        }
    }
}

#[test]
fn test_partitioner_v2_determinism() {
    for merge_discarded in [false, true] {
        let partitioner = Arc::new(PartitionerV2::new(4, 4, 10, 64, merge_discarded));
        assert_deterministic_result(partitioner);
    }
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
