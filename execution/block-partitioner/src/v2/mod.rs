// Copyright © Aptos Foundation

#[cfg(test)]
use crate::test_utils::assert_deterministic_result;
#[cfg(test)]
use crate::test_utils::P2PBlockGenerator;
use crate::{
    pre_partition::{uniform_partitioner::UniformPartitioner, PrePartitioner},
    v2::counters::MISC_TIMERS_SECONDS,
    BlockPartitioner,
};
use aptos_types::{
    block_executor::partitioner::{PartitionedTransactions, RoundId},
    transaction::analyzed_transaction::AnalyzedTransaction,
};
#[cfg(test)]
use rand::thread_rng;
#[cfg(test)]
use rand::Rng;
use rayon::{ThreadPool, ThreadPoolBuilder};
use state::PartitionState;
use std::sync::{Arc, RwLock};
use types::PreParedTxnIdx;

mod build_edge;
pub mod config;
mod conflicting_txn_tracker;
mod counters;
mod init;
mod partition_to_matrix;
pub(crate) mod state;
pub mod types;

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
}

impl PartitionerV2 {
    pub fn new(
        num_threads: usize,
        num_rounds_limit: usize,
        cross_shard_dep_avoid_threshold: f32,
        dashmap_num_shards: usize,
        partition_last_round: bool,
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
        }
    }

    fn pre_partition(
        &self,
        txns: &[AnalyzedTransaction],
        num_shards: usize,
    ) -> Vec<Vec<PreParedTxnIdx>> {
        let _timer = MISC_TIMERS_SECONDS
            .with_label_values(&["pre_partition"])
            .start_timer();
        self.pre_partitioner.pre_partition(txns, num_shards)
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

        let pre_partitioned = self.pre_partition(txns.as_slice(), num_executor_shards);

        let mut state = PartitionState::new(
            self.thread_pool.clone(),
            self.dashmap_num_shards,
            txns,
            num_executor_shards,
            pre_partitioned,
            self.max_partitioning_rounds,
            self.cross_shard_dep_avoid_threshold,
            self.partition_last_round,
        );
        Self::init(&mut state);
        Self::partition_to_matrix(&mut state);
        Self::build_index_from_txn_matrix(&mut state);
        let ret = Self::add_edges(&mut state);

        self.thread_pool.spawn(move || {
            drop(state);
        });
        ret
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use rand::{Rng, thread_rng};
    use crate::test_utils::{assert_deterministic_result, P2PBlockGenerator};
    use crate::v2::PartitionerV2;
    use crate::BlockPartitioner;

    #[test]
    fn test_partitioner_v2_correctness() {
        for merge_discarded in [false, true] {
            let block_generator = P2PBlockGenerator::new(100);
            let partitioner = PartitionerV2::new(8, 4, 0.9, 64, merge_discarded);
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
            let partitioner = Arc::new(PartitionerV2::new(4, 4, 0.9, 64, merge_discarded));
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
