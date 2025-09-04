// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(unused_variables)]

use crate::{
    Sender,
    v2::{
        conflicting_txn_tracker::ConflictingTxnTracker,
        counters::MISC_TIMERS_SECONDS,
        types::{
            FinalTxnIdx, OriginalTxnIdx, PrePartitionedTxnIdx, SenderIdx, ShardedTxnIndexV2,
            StorageKeyIdx, SubBlockIdx,
        },
    },
};
use aptos_metrics_core::TimerHelper;
use aptos_types::{
    block_executor::partitioner::{
        CrossShardDependencies, RoundId, ShardId, ShardedTxnIndex, SubBlock,
        TransactionWithDependencies,
    },
    state_store::state_key::StateKey,
    transaction::analyzed_transaction::{AnalyzedTransaction, StorageLocation},
};
use dashmap::DashMap;
use rayon::{
    ThreadPool,
    iter::{IntoParallelIterator, ParallelIterator},
};
use std::{
    collections::HashSet,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicUsize, Ordering},
    },
};

/// All the parameters, indexes, temporary states needed in a `PartitionerV2` session,
/// wrapped in a single struct, so we don't forget to async drop any large states.
pub struct PartitionState {
    //
    // Initial params/utils begin.
    //
    pub(crate) num_executor_shards: ShardId,
    pub(crate) num_rounds_limit: usize,
    pub(crate) dashmap_num_shards: usize,
    pub(crate) cross_shard_dep_avoid_threshold: f32,
    pub(crate) partition_last_round: bool,
    pub(crate) thread_pool: Arc<ThreadPool>,
    /// OriginalTxnIdx -> the actual txn.
    /// Wrapped in `RwLock` to allow being taking in parallel in `add_edges` phase and parallel reads in other phases.
    pub(crate) txns: Vec<RwLock<Option<AnalyzedTransaction>>>,
    //
    // Initial params/utils ends.
    //
    /// A `ConflictingTxnTracker` for each key that helps resolve conflicts and speed-up edge creation.
    /// Updated in multiple stages of partitioning.
    pub(crate) trackers: DashMap<StorageKeyIdx, RwLock<ConflictingTxnTracker>>,

    //
    // States computed in `init()` begin.
    //
    /// For txn of OriginalTxnIdx i, the sender index.
    pub(crate) sender_idxs: Vec<RwLock<Option<SenderIdx>>>,

    /// For txn of OriginalTxnIdx i, the writer set.
    pub(crate) write_sets: Vec<RwLock<HashSet<StorageKeyIdx>>>,

    /// For txn of OriginalTxnIdx i, the read set.
    pub(crate) read_sets: Vec<RwLock<HashSet<StorageKeyIdx>>>,

    pub(crate) sender_counter: AtomicUsize,
    pub(crate) sender_idx_table: DashMap<Sender, SenderIdx>,

    pub(crate) storage_key_counter: AtomicUsize,
    pub(crate) key_idx_table: DashMap<StateKey, StorageKeyIdx>,

    //
    // States computed in `init()` end.
    // States computed in `pre_partition()` begin.
    //
    /// For shard i, the `PrePartitionedTxnIdx`s of the txns pre-partitioned into shard i.
    pub(crate) pre_partitioned: Vec<Vec<PrePartitionedTxnIdx>>,

    /// For shard i, the num of txns pre-partitioned into shard 0..=i-1.
    pub(crate) start_txn_idxs_by_shard: Vec<PrePartitionedTxnIdx>,

    /// Map the `PrePartitionedTxnIdx` of a transaction to its `OriginalTxnIdx`.
    pub(crate) ori_idxs_by_pre_partitioned: Vec<OriginalTxnIdx>,

    //
    // States computed in `pre_partition()` end.
    // States computed in `remove_cross_shard_dependencies()` begin.
    //
    pub(crate) finalized_txn_matrix: Vec<Vec<Vec<PrePartitionedTxnIdx>>>,
    pub(crate) start_index_matrix: Vec<Vec<PrePartitionedTxnIdx>>,

    /// Map the PrePartitionedTxnIdx of a transaction to its FinalTxnIdx.
    pub(crate) final_idxs_by_pre_partitioned: Vec<RwLock<FinalTxnIdx>>,
    //
    // States computed in `remove_cross_shard_dependencies()` end.
    //

    // Temporary sub-block matrix used in `add_edges()`.
    pub(crate) sub_block_matrix: Vec<Vec<Mutex<Option<SubBlock<AnalyzedTransaction>>>>>,
}

/// Some utils.
impl PartitionState {
    pub fn new(
        thread_pool: Arc<ThreadPool>,
        dashmap_num_shards: usize,
        txns: Vec<AnalyzedTransaction>,
        num_executor_shards: ShardId,
        num_rounds_limit: usize,
        cross_shard_dep_avoid_threshold: f32,
        partition_last_round: bool,
    ) -> Self {
        let _timer = MISC_TIMERS_SECONDS.timer_with(&["new"]);
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
        let takable_txns = thread_pool.install(|| {
            txns.into_par_iter()
                .map(|txn| RwLock::new(Some(txn)))
                .collect()
        });

        Self {
            dashmap_num_shards,
            partition_last_round,
            thread_pool,
            num_executor_shards,
            pre_partitioned: vec![],
            start_txn_idxs_by_shard: vec![0; num_executor_shards],
            sender_counter,
            storage_key_counter: key_counter,
            sender_idxs: senders,
            write_sets: wsets,
            read_sets: rsets,
            sender_idx_table,
            key_idx_table,
            trackers,
            cross_shard_dep_avoid_threshold,
            num_rounds_limit,
            finalized_txn_matrix: Vec::with_capacity(num_rounds_limit),
            final_idxs_by_pre_partitioned: vec![],
            start_index_matrix: vec![],
            txns: takable_txns,
            sub_block_matrix: vec![],
            ori_idxs_by_pre_partitioned: vec![0; num_txns],
        }
    }

    pub(crate) fn num_txns(&self) -> usize {
        self.txns.len()
    }

    pub(crate) fn num_keys(&self) -> usize {
        self.storage_key_counter.load(Ordering::SeqCst)
    }

    pub(crate) fn num_senders(&self) -> usize {
        self.sender_counter.load(Ordering::SeqCst)
    }

    pub(crate) fn add_key(&self, key: &StateKey) -> StorageKeyIdx {
        *self
            .key_idx_table
            .entry(key.clone())
            .or_insert_with(|| self.storage_key_counter.fetch_add(1, Ordering::SeqCst))
    }

    pub(crate) fn storage_location(&self, key_idx: StorageKeyIdx) -> StorageLocation {
        let tracker_ref = self.trackers.get(&key_idx).unwrap();
        let tracker = tracker_ref.read().unwrap();
        tracker.storage_location.clone()
    }

    pub(crate) fn sender_idx(&self, ori_txn_idx: OriginalTxnIdx) -> SenderIdx {
        *self.sender_idxs[ori_txn_idx]
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
    }

    pub(crate) fn add_sender(&self, sender: Sender) -> SenderIdx {
        *self
            .sender_idx_table
            .entry(sender)
            .or_insert_with(|| self.sender_counter.fetch_add(1, Ordering::SeqCst))
    }

    /// For a key, check if there is any write between the anchor shard and a given shard.
    pub(crate) fn key_owned_by_another_shard(&self, shard_id: ShardId, key: StorageKeyIdx) -> bool {
        let tracker_ref = self.trackers.get(&key).unwrap();
        let tracker = tracker_ref.read().unwrap();
        let range_start = self.start_txn_idxs_by_shard[tracker.anchor_shard_id];
        let range_end = self.start_txn_idxs_by_shard[shard_id];
        tracker.has_write_in_range(range_start, range_end)
    }

    pub(crate) fn update_trackers_on_accepting(
        &self,
        txn_idx: PrePartitionedTxnIdx,
        round_id: RoundId,
        shard_id: ShardId,
    ) {
        let ori_txn_idx = self.ori_idxs_by_pre_partitioned[txn_idx];
        let write_set = self.write_sets[ori_txn_idx].read().unwrap();
        let read_set = self.read_sets[ori_txn_idx].read().unwrap();
        for &key_idx in write_set.iter().chain(read_set.iter()) {
            self.trackers
                .get(&key_idx)
                .unwrap()
                .write()
                .unwrap()
                .mark_txn_ordered(txn_idx, round_id, shard_id);
        }
    }

    /// Get the last txn inside `sub_block` that writes a given key.
    pub(crate) fn last_writer(
        &self,
        key: StorageKeyIdx,
        sub_block: SubBlockIdx,
    ) -> Option<PrePartitionedTxnIdx> {
        let tracker_ref = self.trackers.get(&key).unwrap();
        let tracker = tracker_ref.read().unwrap();
        let start = ShardedTxnIndexV2::new(sub_block.round_id, sub_block.shard_id, 0);
        let end = ShardedTxnIndexV2::new(sub_block.round_id, sub_block.shard_id + 1, 0);
        let ret = tracker
            .finalized_writes
            .range(start..end)
            .last()
            .map(|t| t.pre_partitioned_txn_idx);
        ret
    }

    /// Get the 1st txn after `since` that writes a given key.
    pub(crate) fn first_writer(
        &self,
        key: StorageKeyIdx,
        since: ShardedTxnIndexV2,
    ) -> Option<ShardedTxnIndexV2> {
        let tracker_ref = self.trackers.get(&key).unwrap();
        let tracker = tracker_ref.read().unwrap();
        tracker.finalized_writes.range(since..).next().copied()
    }

    /// Get all txns that access a certain key in a sub-block range.
    pub(crate) fn all_txns_in_sub_block_range(
        &self,
        key: StorageKeyIdx,
        start: ShardedTxnIndexV2,
        end: ShardedTxnIndexV2,
    ) -> Vec<ShardedTxnIndexV2> {
        let tracker_ref = self.trackers.get(&key).unwrap();
        let tracker = tracker_ref.read().unwrap();
        tracker.finalized.range(start..end).copied().collect()
    }

    pub(crate) fn num_rounds(&self) -> usize {
        self.finalized_txn_matrix.len()
    }

    pub(crate) fn final_sub_block_idx(&self, sub_blk_idx: SubBlockIdx) -> SubBlockIdx {
        if !self.partition_last_round && sub_blk_idx.round_id == self.num_rounds() - 1 {
            SubBlockIdx::global()
        } else {
            sub_blk_idx
        }
    }

    /// Take a txn out, wrap it as a `TransactionWithDependencies`.
    pub(crate) fn take_txn_with_dep(
        &self,
        round_id: RoundId,
        shard_id: ShardId,
        txn_idx: PrePartitionedTxnIdx,
    ) -> TransactionWithDependencies<AnalyzedTransaction> {
        let ori_txn_idx = self.ori_idxs_by_pre_partitioned[txn_idx];
        let txn = self.txns[ori_txn_idx].write().unwrap().take().unwrap();
        let mut deps = CrossShardDependencies::default();

        // Build required edges.
        let write_set = self.write_sets[ori_txn_idx].read().unwrap();
        let read_set = self.read_sets[ori_txn_idx].read().unwrap();
        for &key_idx in write_set.iter().chain(read_set.iter()) {
            let tracker_ref = self.trackers.get(&key_idx).unwrap();
            let tracker = tracker_ref.read().unwrap();
            if let Some(txn_idx) = tracker
                .finalized_writes
                .range(..ShardedTxnIndexV2::new(round_id, shard_id, 0))
                .last()
            {
                let src_txn_idx = ShardedTxnIndex {
                    txn_index: *self.final_idxs_by_pre_partitioned[txn_idx.pre_partitioned_txn_idx]
                        .read()
                        .unwrap(),
                    shard_id: txn_idx.shard_id(),
                    round_id: txn_idx.round_id(),
                };
                deps.add_required_edge(src_txn_idx, tracker.storage_location.clone());
            }
        }

        // Build dependent edges.
        for &key_idx in self.write_sets[ori_txn_idx].read().unwrap().iter() {
            if Some(txn_idx) == self.last_writer(key_idx, SubBlockIdx { round_id, shard_id }) {
                let start_of_next_sub_block = ShardedTxnIndexV2::new(round_id, shard_id + 1, 0);
                let next_writer = self.first_writer(key_idx, start_of_next_sub_block);
                let end_follower = match next_writer {
                    None => ShardedTxnIndexV2::new(self.num_rounds(), self.num_executor_shards, 0), // Guaranteed to be greater than any invalid idx...
                    Some(idx) => ShardedTxnIndexV2::new(idx.round_id(), idx.shard_id() + 1, 0),
                };
                for follower_txn_idx in
                    self.all_txns_in_sub_block_range(key_idx, start_of_next_sub_block, end_follower)
                {
                    let final_sub_blk_idx =
                        self.final_sub_block_idx(follower_txn_idx.sub_block_idx);
                    let dst_txn_idx = ShardedTxnIndex {
                        txn_index: *self.final_idxs_by_pre_partitioned
                            [follower_txn_idx.pre_partitioned_txn_idx]
                            .read()
                            .unwrap(),
                        shard_id: final_sub_blk_idx.shard_id,
                        round_id: final_sub_blk_idx.round_id,
                    };
                    deps.add_dependent_edge(dst_txn_idx, vec![self.storage_location(key_idx)]);
                }
            }
        }

        TransactionWithDependencies::new(txn, deps)
    }
}
