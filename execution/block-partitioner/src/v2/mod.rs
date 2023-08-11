// Copyright © Aptos Foundation

#[cfg(test)]
use crate::test_utils::assert_deterministic_result;
#[cfg(test)]
use crate::test_utils::P2PBlockGenerator;
use crate::{
    get_anchor_shard_id,
    pre_partition::{start_txn_idxs, uniform, uniform::UniformPartitioner, PrePartitioner},
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
    pre_partitioner: Box<dyn PrePartitioner>,
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
            pre_partitioner: Box::new(UniformPartitioner {}),
            thread_pool,
            num_rounds_limit,
            avoid_pct,
            dashmap_num_shards,
            merge_discarded,
        }
    }
}

mod build_edge;
mod flatten;
mod init;

impl BlockPartitioner for PartitionerV2 {
    fn partition(
        &self,
        txns: Vec<AnalyzedTransaction>,
        num_executor_shards: usize,
    ) -> PartitionedTransactions {
        let pre_partitioned = self
            .pre_partitioner
            .pre_partition(txns.as_slice(), num_executor_shards);

        let mut session = PartitionState::new(
            self.thread_pool.clone(),
            self.dashmap_num_shards,
            txns,
            num_executor_shards,
            pre_partitioned,
            self.num_rounds_limit,
            self.avoid_pct,
            self.merge_discarded,
        );
        Self::init(&mut session);
        Self::flatten_to_rounds(&mut session);
        let ret = Self::add_edges(&mut session);

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

pub struct PartitionState {
    thread_pool: Arc<ThreadPool>,
    num_executor_shards: ShardId,
    txns: Vec<RwLock<Option<AnalyzedTransaction>>>,
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

impl PartitionState {
    pub fn new(
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
        let takable_txns = thread_pool.install(|| {
            txns.into_par_iter()
                .map(|txn| RwLock::new(Some(txn)))
                .collect()
        });
        let start_txn_idxs_by_shard = start_txn_idxs(&pre_partitioned);
        Self {
            merge_discarded,
            thread_pool,
            num_executor_shards,
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
            txns: takable_txns,
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

    fn make_txn_with_dep(
        &self,
        round_id: RoundId,
        shard_id: ShardId,
        ori_txn_idx: OriginalTxnIdx,
    ) -> TransactionWithDependencies<AnalyzedTransaction> {
        let txn = self.txns[ori_txn_idx].write().unwrap().take().unwrap();
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
                    txn_index: *self.new_txn_idxs[txn_idx.ori_txn_idx].read().unwrap(),
                    shard_id: txn_idx.sub_block_idx.shard_id,
                    round_id: txn_idx.sub_block_idx.round_id,
                };
                deps.add_required_edge(src_txn_idx, tracker.storage_location.clone());
            }
        }
        for key_idx in self.write_hints(ori_txn_idx) {
            if Some(ori_txn_idx) == self.last_writer(key_idx, SubBlockIdx { round_id, shard_id }) {
                let start_of_next_sub_block = ShardedTxnIndex2::new(round_id, shard_id + 1, 0);
                let next_writer = self.first_writer(key_idx, start_of_next_sub_block);
                let end_follower = match next_writer {
                    None => ShardedTxnIndex2::new(self.num_rounds(), self.num_executor_shards, 0), // Guaranteed to be greater than any invalid idx...
                    Some(idx) => ShardedTxnIndex2::new(idx.round_id(), idx.shard_id() + 1, 0),
                };
                for follower_txn_idx in
                    self.all_accepted_txns(key_idx, start_of_next_sub_block, end_follower)
                {
                    let actual_sub_blk_idx =
                        self.actual_sub_block_idx(follower_txn_idx.sub_block_idx);
                    let dst_txn_idx = ShardedTxnIndex {
                        txn_index: *self.new_txn_idxs[follower_txn_idx.ori_txn_idx]
                            .read()
                            .unwrap(),
                        shard_id: actual_sub_blk_idx.shard_id,
                        round_id: actual_sub_blk_idx.round_id,
                    };
                    deps.add_dependent_edge(dst_txn_idx, vec![self.storage_location(key_idx)]);
                }
            }
        }
        TransactionWithDependencies::new(txn, deps)
    }
}
