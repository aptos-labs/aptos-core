// Copyright © Aptos Foundation

use std::collections::HashMap;
use crate::{
    pre_partition::{uniform_partitioner::UniformPartitioner, PrePartitioner},
    v2::counters::MISC_TIMERS_SECONDS,
    BlockPartitioner,
};
use aptos_types::{
    block_executor::partitioner::{PartitionedTransactions, RoundId},
    transaction::analyzed_transaction::AnalyzedTransaction,
};
use rayon::{ThreadPool, ThreadPoolBuilder};
use state::PartitionState;
use std::sync::{Arc, RwLock};
use crate::v2::load_balance::assign_tasks_to_workers;
use crate::v2::types::{TxnIdx0, TxnIdx1};
use crate::v2::union_find::UnionFind;

mod build_edge;
pub mod config;
mod conflicting_txn_tracker;
mod counters;
mod init;
mod pre_partition;
mod partition_to_matrix;
pub(crate) mod state;
pub mod types;
mod union_find;
mod load_balance;

/// Basically `ShardedBlockPartitioner` but:
/// - Not pre-partitioned by txn sender.
/// - implemented more efficiently.
pub struct PartitionerV2 {
    pre_partitioner: Box<dyn PrePartitioner>,
    thread_pool: Arc<ThreadPool>,
    max_partitioning_rounds: RoundId,
    cross_shard_dep_avoid_threshold: f32,
    dashmap_num_shards: usize,
    partition_last_round: bool,
    load_imbalance_tolerance: f32,
}

impl PartitionerV2 {
    pub fn new(
        num_threads: usize,
        num_rounds_limit: usize,
        cross_shard_dep_avoid_threshold: f32,
        dashmap_num_shards: usize,
        partition_last_round: bool,
        load_imbalance_tolerance: f32,
    ) -> Self {
        let thread_pool = Arc::new(
            ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
                .unwrap(),
        );
        Self {
            pre_partitioner: Box::new(UniformPartitioner {}), //TODO: parameterize it.
            thread_pool,
            max_partitioning_rounds: num_rounds_limit,
            cross_shard_dep_avoid_threshold,
            dashmap_num_shards,
            partition_last_round,
            load_imbalance_tolerance,
        }
    }

    fn pre_partition(
        state: &mut PartitionState,
    ) {
        // Union-find.
        let num_senders = state.num_senders();
        let num_keys = state.num_keys();
        let mut uf = UnionFind::new(num_senders + num_keys);
        for txn_idx0 in 0..state.num_txns() {
            let sender_idx = state.sender_idx(txn_idx0);
            let write_set = state.write_sets[txn_idx0].read().unwrap();
            let read_set = state.read_sets[txn_idx0].read().unwrap();
            for &key_idx in write_set.iter().chain(read_set.iter()) {
                let key_idx_in_uf = num_senders + key_idx;
                uf.union(key_idx_in_uf, sender_idx);
            }
        }

        let mut set_sizes: HashMap<usize, usize> = HashMap::new();
        for txn_idx0 in 0..state.num_txns() {
            let sender_idx = state.sender_idx(txn_idx0);
            let set_idx = uf.find(sender_idx);
            let set_size_entry = set_sizes.entry(set_idx).or_insert_with(||0);
            *set_size_entry += 1;
        }


        // Create groups following the size limit.
        let group_size_limit = ((state.num_txns() as f32) * state.load_imbalance_tolerance / (state.num_executor_shards as f32)) as usize;
        let mut route_table: HashMap<usize, usize> = HashMap::new(); // Keep track of the idx of the group that members of a set should go to.
        let mut groups: Vec<Vec<TxnIdx0>> = vec![];
        for txn_idx0 in 0..state.num_txns() {
            let sender_idx = state.sender_idx(txn_idx0);
            let set_idx = uf.find(sender_idx);
            let group_idx_entry = route_table.entry(set_idx).or_insert_with(||{
                groups.push(vec![]);
                groups.len() - 1
            });
            if groups[*group_idx_entry].len() >= group_size_limit {
                groups.push(vec![]);
                *group_idx_entry = groups.len() - 1;
            }
            groups[*group_idx_entry].push(txn_idx0);
        }

        // Assign groups to shards in a way that minimize the longest pole.
        let tasks: Vec<u64> = groups.iter().map(|g|g.len() as u64).collect();
        let (_longest_pole, group_destinations) = assign_tasks_to_workers(&tasks, state.num_executor_shards);
        state.pre_partitioned_idx0s = vec![vec![]; state.num_executor_shards];
        for (group_idx, group) in groups.into_iter().enumerate() {
            let shard_id = group_destinations[group_idx];
            state.pre_partitioned_idx0s[shard_id].extend(group);
        }

        // Prepare `state.i1_to_i0` and `state.start_txn_idxs_by_shard`.
        let mut i1 = 0;
        for (shard_id, txn_idxs) in state.pre_partitioned_idx0s.iter().enumerate() {
            state.start_txn_idxs_by_shard[shard_id] = i1;
            for &i0 in txn_idxs {
                state.i1_to_i0[i1] = i0;
                i1 += 1;
            }
        }

        // Prepare `state.pre_partitioned`.
        state.pre_partitioned = (0..state.num_executor_shards).into_iter().map(|shard_id|{
            let start = state.start_txn_idxs_by_shard[shard_id];
            let end: TxnIdx1 = if shard_id == state.num_executor_shards - 1 { state.num_txns() } else { state.start_txn_idxs_by_shard[shard_id + 1] };
            (start..end).collect()
        }).collect();
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
            self.load_imbalance_tolerance,
        );
        // Step 1: build some necessary indices for txn senders/storage locations.
        Self::init(&mut state);

        // Step 1.5: pre-partition.
        Self::pre_partition(&mut state);

        // Step 1.8: update trackers.
        for txn_idx1 in 0..state.num_txns() {
            let txn_idx0 = state.i1_to_i0[txn_idx1];
            let wset_guard = state.write_sets[txn_idx0].read().unwrap();
            let rset_guard = state.read_sets[txn_idx0].read().unwrap();
            let writes = wset_guard.iter().map(|key_idx|(key_idx, true));
            let reads = rset_guard.iter().map(|key_idx|(key_idx, false));;
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

        // Step 2: remove cross-shard dependencies by move some txns into new rounds.
        // As a result, we get a txn matrix of no more than `self.max_partitioning_rounds` rows and exactly `num_executor_shards` columns.
        // It's guaranteed that inside every round other than the last round, there's no cross-shard dependency. (But cross-round dependencies are always possible.)
        Self::remove_cross_shard_dependencies(&mut state);
        println!("matrix={:?}", state.finalized_txn_matrix);
        // Step 3: build some additional indices of the resulting txn matrix from Step 2.
        Self::build_index_from_txn_matrix(&mut state);

        // Step 4: calculate all the cross-shard dependencies and prepare the input for sharded execution.
        let ret = Self::add_edges(&mut state);

        // Async clean-up.
        self.thread_pool.spawn(move || {
            drop(state);
        });
        ret
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        test_utils::{assert_deterministic_result, P2PBlockGenerator},
        v2::PartitionerV2,
        BlockPartitioner,
    };
    use rand::{thread_rng, Rng};
    use std::sync::Arc;

    #[test]
    fn test_partitioner_v2_correctness() {
        for merge_discarded in [false, true] {
            let block_generator = P2PBlockGenerator::new(100);
            let partitioner = PartitionerV2::new(8, 4, 0.9, 64, merge_discarded, 2.0);
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
            let partitioner = Arc::new(PartitionerV2::new(4, 4, 0.9, 64, merge_discarded, 2.0));
            assert_deterministic_result(partitioner);
        }
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
