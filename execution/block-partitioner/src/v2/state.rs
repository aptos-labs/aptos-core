// Copyright © Aptos Foundation

#![allow(unused_variables)]

use crate::{
    pre_partition::start_txn_idxs,
    v2::{
        conflicting_txn_tracker::ConflictingTxnTracker,
        counters::MISC_TIMERS_SECONDS,
        types::{PrePartitionedTxnIdx, SenderIdx, ShardedTxnIndexV2, StorageKeyIdx, SubBlockIdx},
    },
    Sender,
};
use aptos_types::{
    block_executor::partitioner::{
        CrossShardDependencies, RoundId, ShardId, ShardedTxnIndex, SubBlock,
        TransactionWithDependencies, TxnIndex,
    },
    state_store::state_key::StateKey,
    transaction::analyzed_transaction::{AnalyzedTransaction, StorageLocation},
};
use dashmap::DashMap;
use rayon::{
    iter::{IntoParallelIterator, ParallelIterator},
    ThreadPool,
};
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, RwLock,
    },
};

/// All the parameters, indexes, temporary states needed in a `PartitionerV2` session wrapped in a single struct
/// to make async drop easy.
pub struct PartitionState {
    // Params/utils from partitioner.
    pub(crate) num_executor_shards: ShardId,
    pub(crate) num_rounds_limit: usize,
    pub(crate) dashmap_num_shards: usize,
    pub(crate) cross_shard_dep_avoid_threshold: f32,
    pub(crate) partition_last_round: bool,
    pub(crate) thread_pool: Arc<ThreadPool>,

    /// Holding all the txns.
    /// Wrapped in `RwLock` to allow being taking in parallel in `add_edges` phase and parallel reads in other phases.
    pub(crate) txns: Vec<RwLock<Option<AnalyzedTransaction>>>,

    // Pre-partitioning results.
    pub(crate) pre_partitioned: Vec<Vec<PrePartitionedTxnIdx>>,
    pub(crate) start_txn_idxs_by_shard: Vec<PrePartitionedTxnIdx>,

    // The discretized txn info, populated in `init()`.
    /// Sender index by `PreParedTxnIdx`.
    pub(crate) sender_idxs: Vec<RwLock<Option<SenderIdx>>>,
    /// Write key indices by `PreParedTxnIdx`.
    pub(crate) write_sets: Vec<RwLock<HashSet<StorageKeyIdx>>>,
    /// Read key indices by `PreParedTxnIdx`.
    pub(crate) read_sets: Vec<RwLock<HashSet<StorageKeyIdx>>>,

    // Used in `init()` to discretize senders.
    pub(crate) sender_counter: AtomicUsize,
    pub(crate) sender_idx_table: DashMap<Sender, SenderIdx>,

    // Used in `init()` to discretize storage locations.
    pub(crate) storage_key_counter: AtomicUsize,
    pub(crate) key_idx_table: DashMap<StateKey, StorageKeyIdx>,

    // A `ConflictingTxnTracker` for each key that helps resolve conflicts and speed-up edge creation.
    pub(crate) trackers: DashMap<StorageKeyIdx, RwLock<ConflictingTxnTracker>>,

    // Results of `remove_cross_shard_dependencies()`.
    pub(crate) finalized_txn_matrix: Vec<Vec<Vec<PrePartitionedTxnIdx>>>,
    pub(crate) start_index_matrix: Vec<Vec<PrePartitionedTxnIdx>>,
    pub(crate) new_txn_idxs: Vec<RwLock<TxnIndex>>,

    // Temporary sub-block matrix used in `add_edges()`.
    pub(crate) sub_block_matrix: Vec<Vec<Mutex<Option<SubBlock<AnalyzedTransaction>>>>>,
}

/// Some small operations.
impl PartitionState {
    pub fn new(
        thread_pool: Arc<ThreadPool>,
        dashmap_num_shards: usize,
        txns: Vec<AnalyzedTransaction>,
        num_executor_shards: ShardId,
        pre_partitioned: Vec<Vec<PrePartitionedTxnIdx>>,
        num_rounds_limit: usize,
        cross_shard_dep_avoid_threshold: f32,
        merge_discarded: bool,
    ) -> Self {
        let _timer = MISC_TIMERS_SECONDS
            .with_label_values(&["new"])
            .start_timer();
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
        let start_txn_idxs_by_shard = start_txn_idxs(&pre_partitioned);
        Self {
            dashmap_num_shards,
            partition_last_round: merge_discarded,
            thread_pool,
            num_executor_shards,
            pre_partitioned,
            start_txn_idxs_by_shard,
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
            new_txn_idxs: vec![],
            start_index_matrix: vec![],
            txns: takable_txns,
            sub_block_matrix: vec![],
        }
    }

    pub(crate) fn num_txns(&self) -> usize {
        self.txns.len()
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

    pub(crate) fn sender_idx(&self, txn_idx: PrePartitionedTxnIdx) -> SenderIdx {
        *self.sender_idxs[txn_idx].read().unwrap().as_ref().unwrap()
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
        ori_txn_idx: PrePartitionedTxnIdx,
        round_id: RoundId,
        shard_id: ShardId,
    ) {
        let write_set = self.write_sets[ori_txn_idx].read().unwrap();
        let read_set = self.read_sets[ori_txn_idx].read().unwrap();
        for &key_idx in write_set.iter().chain(read_set.iter()) {
            self.trackers
                .get(&key_idx)
                .unwrap()
                .write()
                .unwrap()
                .mark_txn_ordered(ori_txn_idx, round_id, shard_id);
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
            .map(|t| t.ori_txn_idx);
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
        ori_txn_idx: PrePartitionedTxnIdx,
    ) -> TransactionWithDependencies<AnalyzedTransaction> {
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
                    txn_index: *self.new_txn_idxs[txn_idx.ori_txn_idx].read().unwrap(),
                    shard_id: txn_idx.shard_id(),
                    round_id: txn_idx.round_id(),
                };
                deps.add_required_edge(src_txn_idx, tracker.storage_location.clone());
            }
        }

        // Build dependent edges.
        for &key_idx in self.write_sets[ori_txn_idx].read().unwrap().iter() {
            if Some(ori_txn_idx) == self.last_writer(key_idx, SubBlockIdx { round_id, shard_id }) {
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
                        txn_index: *self.new_txn_idxs[follower_txn_idx.ori_txn_idx]
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
