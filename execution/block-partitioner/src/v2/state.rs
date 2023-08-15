// Copyright © Aptos Foundation

use crate::{
    pre_partition::start_txn_idxs,
    v2::{
        conflicting_txn_tracker::ConflictingTxnTracker,
        counters::MISC_TIMERS_SECONDS,
        types::{PreParedTxnIdx, SenderIdx, ShardedTxnIndexV2, StorageKeyIdx, SubBlockIdx},
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
    mem,
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
    pub(crate) cross_shard_dep_avoid_threshold: f32,
    pub(crate) partition_last_round: bool,
    pub(crate) thread_pool: Arc<ThreadPool>,

    // Holding all the txns.
    pub(crate) txns: Vec<RwLock<Option<AnalyzedTransaction>>>,

    // Pre-partitioning results.
    pub(crate) pre_partitioned: Vec<Vec<PreParedTxnIdx>>,
    pub(crate) start_txn_idxs_by_shard: Vec<PreParedTxnIdx>,

    // The discretized txn info, populated in `init()`.
    pub(crate) sender_idxs: Vec<RwLock<Option<SenderIdx>>>,
    pub(crate) write_sets: Vec<RwLock<HashSet<StorageKeyIdx>>>,
    pub(crate) read_sets: Vec<RwLock<HashSet<StorageKeyIdx>>>,

    // Used in `init()` to discretize senders.
    pub(crate) sender_counter: AtomicUsize,
    pub(crate) sender_idx_table: DashMap<Sender, SenderIdx>,

    // Used in `init()` to discretize storage locations.
    pub(crate) key_counter: AtomicUsize,
    pub(crate) key_idx_table: DashMap<StateKey, StorageKeyIdx>,

    // A `ConflictingTxnTracker` for each key that helps resolve conflicts and speed-up edge creation.
    pub(crate) trackers: DashMap<StorageKeyIdx, RwLock<ConflictingTxnTracker>>,

    // Used in `flatten_to_rounds()` to preserve relative txns order for the same sender.
    pub(crate) min_discards_by_sender: DashMap<SenderIdx, AtomicUsize>,

    // Results of `flatten_to_rounds()`.
    pub(crate) finalized_txn_matrix: Vec<Vec<Vec<PreParedTxnIdx>>>,
    pub(crate) start_index_matrix: Vec<Vec<PreParedTxnIdx>>,
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
        pre_partitioned: Vec<Vec<PreParedTxnIdx>>,
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
            partition_last_round: merge_discarded,
            thread_pool,
            num_executor_shards,
            pre_partitioned,
            start_txn_idxs_by_shard,
            sender_counter,
            key_counter,
            sender_idxs: senders,
            write_sets: wsets,
            read_sets: rsets,
            sender_idx_table,
            key_idx_table,
            trackers,
            min_discards_by_sender: DashMap::new(),
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
            .or_insert_with(|| self.key_counter.fetch_add(1, Ordering::SeqCst))
    }

    pub(crate) fn num_keys(&self) -> usize {
        self.key_counter.load(Ordering::SeqCst)
    }

    pub(crate) fn reset_min_discard_table(&mut self) {
        let table = mem::take(&mut self.min_discards_by_sender);
        self.thread_pool.spawn(move || {
            drop(table);
        });
    }

    pub(crate) fn storage_location(&self, key_idx: StorageKeyIdx) -> StorageLocation {
        let tracker_ref = self.trackers.get(&key_idx).unwrap();
        let tracker = tracker_ref.read().unwrap();
        tracker.storage_location.clone()
    }

    pub(crate) fn sender_idx(&self, txn_idx: PreParedTxnIdx) -> SenderIdx {
        *self.sender_idxs[txn_idx].read().unwrap().as_ref().unwrap()
    }

    pub(crate) fn add_sender(&self, sender: Sender) -> SenderIdx {
        *self
            .sender_idx_table
            .entry(sender)
            .or_insert_with(|| self.sender_counter.fetch_add(1, Ordering::SeqCst))
    }

    pub(crate) fn key_owned_by_another_shard(&self, shard_id: ShardId, key: StorageKeyIdx) -> bool {
        let tracker_ref = self.trackers.get(&key).unwrap();
        let tracker = tracker_ref.read().unwrap();
        let range_start = self.start_txn_idxs_by_shard[tracker.anchor_shard_id];
        let range_end = self.start_txn_idxs_by_shard[shard_id];
        tracker.has_write_in_range(range_start, range_end)
    }

    pub(crate) fn all_hints(&self, txn_idx: PreParedTxnIdx) -> Vec<StorageKeyIdx> {
        let wset = self.write_sets[txn_idx].read().unwrap();
        let rset = self.read_sets[txn_idx].read().unwrap();
        let all: Vec<StorageKeyIdx> = wset.iter().chain(rset.iter()).copied().collect();
        all
    }

    pub(crate) fn write_hints(&self, txn_idx: PreParedTxnIdx) -> Vec<StorageKeyIdx> {
        self.write_sets[txn_idx]
            .read()
            .unwrap()
            .iter()
            .copied()
            .collect()
    }

    pub(crate) fn min_discard(&self, sender: SenderIdx) -> Option<PreParedTxnIdx> {
        self.min_discards_by_sender
            .get(&sender)
            .as_ref()
            .map(|r| r.value().load(Ordering::SeqCst))
    }

    pub(crate) fn update_min_discarded_txn_idx(&self, sender: SenderIdx, txn_idx: PreParedTxnIdx) {
        self.min_discards_by_sender
            .entry(sender)
            .or_insert_with(|| AtomicUsize::new(usize::MAX))
            .value()
            .fetch_min(txn_idx, Ordering::SeqCst);
    }

    pub(crate) fn update_trackers_on_accepting(
        &self,
        ori_txn_idx: PreParedTxnIdx,
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

    /// Get the last txn inside `sub_block` that writes a given key.
    pub(crate) fn last_writer(
        &self,
        key: StorageKeyIdx,
        sub_block: SubBlockIdx,
    ) -> Option<PreParedTxnIdx> {
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
        tracker.finalized_all.range(start..end).copied().collect()
    }

    pub(crate) fn num_rounds(&self) -> usize {
        self.finalized_txn_matrix.len()
    }

    pub(crate) fn final_sub_block_idx(&self, sub_blk_idx: SubBlockIdx) -> SubBlockIdx {
        if !self.partition_last_round {
            if sub_blk_idx.round_id == self.num_rounds() - 1 {
                SubBlockIdx::global()
            } else {
                sub_blk_idx
            }
        } else {
            sub_blk_idx
        }
    }

    /// Take a txn out, wrap it as a `TransactionWithDependencies`.
    pub(crate) fn take_txn_with_dep(
        &self,
        round_id: RoundId,
        shard_id: ShardId,
        ori_txn_idx: PreParedTxnIdx,
    ) -> TransactionWithDependencies<AnalyzedTransaction> {
        let txn = self.txns[ori_txn_idx].write().unwrap().take().unwrap();
        let mut deps = CrossShardDependencies::default();

        // Build required edges.
        for key_idx in self.all_hints(ori_txn_idx) {
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
        for key_idx in self.write_hints(ori_txn_idx) {
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
