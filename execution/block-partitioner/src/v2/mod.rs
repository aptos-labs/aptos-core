// Copyright © Aptos Foundation

use crate::{pre_partition::PrePartitioner, v2::counters::MISC_TIMERS_SECONDS, BlockPartitioner};
use aptos_types::{
    block_executor::partitioner::{PartitionedTransactions, RoundId},
    transaction::analyzed_transaction::AnalyzedTransaction,
};
use rayon::{ThreadPool, ThreadPoolBuilder};
use state::PartitionState;
use std::sync::{Arc, RwLock};

mod build_edge;
pub mod config;
mod conflicting_txn_tracker;
mod counters;
mod init;
pub(crate) mod load_balance;
mod partition_to_matrix;
pub(crate) mod state;
#[cfg(test)]
mod tests;
pub mod types;
pub(crate) mod union_find;

/// A block partitioner that works as follows.
/// - Use a given pre-partitioner to partition a given block into some shards.
/// - Discard some txns from each shard, so the remaining txns has 0 cross-shard dependencies.
/// - Use the discarded txns to form a new round (but keep the partitioned status), and do the same discarding work.
/// - Repeating the work until we have enough number of rounds, or the number of discarded txns is lower than a threshold. We now have a txn matrix.
/// - Optionally, merge the txns in the last round and mark them to be executed in a special global executor.
/// - Calculate cross-shard dependencies for each txn.
///
/// Note that this is essentially `ShardedBlockPartitioner` but:
/// - with configurable PrePartitioner.
/// - implemented more efficiently (~2.5x faster).
pub struct PartitionerV2 {
    pre_partitioner: Box<dyn PrePartitioner>,
    thread_pool: Arc<ThreadPool>,
    max_partitioning_rounds: RoundId,
    cross_shard_dep_avoid_threshold: f32,
    dashmap_num_shards: usize,
    partition_last_round: bool,
}

impl PartitionerV2 {
    pub fn new(
        num_threads: usize,
        num_rounds_limit: usize,
        cross_shard_dep_avoid_threshold: f32,
        dashmap_num_shards: usize,
        partition_last_round: bool,
        pre_partitioner: Box<dyn PrePartitioner>,
    ) -> Self {
        let thread_pool = Arc::new(
            ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
                .unwrap(),
        );
        Self {
            pre_partitioner,
            thread_pool,
            max_partitioning_rounds: num_rounds_limit,
            cross_shard_dep_avoid_threshold,
            dashmap_num_shards,
            partition_last_round,
        }
    }
}

impl BlockPartitioner for PartitionerV2 {
    fn partition(
        &self,
        txns: Vec<AnalyzedTransaction>,
        num_executor_shards: usize,
    ) -> PartitionedTransactions {
        let _timer = MISC_TIMERS_SECONDS
            .with_label_values(&["total"])
            .start_timer();

        let mut state = PartitionState::new(
            self.thread_pool.clone(),
            self.dashmap_num_shards,
            txns,
            num_executor_shards,
            self.max_partitioning_rounds,
            self.cross_shard_dep_avoid_threshold,
            self.partition_last_round,
        );
        // Step 1: build some necessary indices for txn senders/storage locations.
        Self::init(&mut state);

        // Step 2: pre-partition.
        (state.idx1_to_idx0, state.start_txn_idxs_by_shard, state.pre_partitioned) = self.pre_partitioner.pre_partition(&mut state);

        // Step 3: update trackers.
        for txn_idx1 in 0..state.num_txns() {
            let txn_idx0 = state.idx1_to_idx0[txn_idx1];
            let wset_guard = state.write_sets[txn_idx0].read().unwrap();
            let rset_guard = state.read_sets[txn_idx0].read().unwrap();
            let writes = wset_guard.iter().map(|key_idx| (key_idx, true));
            let reads = rset_guard.iter().map(|key_idx| (key_idx, false));
            for (key_idx, is_write) in writes.chain(reads) {
                let tracker_ref = state.trackers.get(key_idx).unwrap();
                let mut tracker = tracker_ref.write().unwrap();
                if is_write {
                    tracker.add_write_candidate(txn_idx1);
                } else {
                    tracker.add_read_candidate(txn_idx1);
                }
            }
        }

        // Step 4: remove cross-shard dependencies by move some txns into new rounds.
        // As a result, we get a txn matrix of no more than `self.max_partitioning_rounds` rows and exactly `num_executor_shards` columns.
        // It's guaranteed that inside every round other than the last round, there's no cross-shard dependency. (But cross-round dependencies are always possible.)
        Self::remove_cross_shard_dependencies(&mut state);

        // Step 5: build some additional indices of the resulting txn matrix from the previous step.
        Self::build_index_from_txn_matrix(&mut state);

        // Step 6: calculate all the cross-shard dependencies and prepare the input for sharded execution.
        let ret = Self::add_edges(&mut state);

        // Async clean-up.
        self.thread_pool.spawn(move || {
            drop(state);
        });
        ret
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
