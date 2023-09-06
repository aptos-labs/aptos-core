// Copyright © Aptos Foundation

use crate::{
    pre_partition::{uniform_partitioner::UniformPartitioner, PrePartitioner},
    v2::counters::{BLOCK_PARTITIONING_SECONDS, MISC_TIMERS_SECONDS},
    BlockPartitioner,
};
use aptos_types::{
    block_executor::partitioner::{PartitionedTransactions, RoundId},
    transaction::analyzed_transaction::AnalyzedTransaction,
};
use rayon::{ThreadPool, ThreadPoolBuilder};
use state::PartitionState;
use std::sync::{Arc, RwLock};
use types::PrePartitionedTxnIdx;

mod build_edge;
pub mod config;
mod conflicting_txn_tracker;
pub mod counters;
mod init;
mod partition_to_matrix;
pub(crate) mod state;
pub mod types;

/// A sharded block partitioner that partitions a block into multiple transaction chunks.
/// On a high level, the partitioning process is as follows:
/// ```plaintext
/// 1. A block is partitioned into equally sized transaction chunks and sent to each shard.
///
///    Block:
///
///    T1 {write set: A, B}
///    T2 {write set: B, C}
///    T3 {write set: C, D}
///    T4 {write set: D, E}
///    T5 {write set: E, F}
///    T6 {write set: F, G}
///    T7 {write set: G, H}
///    T8 {write set: H, I}
///    T9 {write set: I, J}
///
/// 2. Discard a bunch of transactions from the chunks and create new chunks so that
///    there is no cross-shard dependency between transactions in a chunk.
///   2.1 Following information is passed to each shard:
///      - candidate transaction chunks to be partitioned
///      - previously frozen transaction chunks (if any)
///      - read-write set index mapping from previous iteration (if any) - this contains the maximum absolute index
///        of the transaction that read/wrote to a storage location indexed by the storage location.
///   2.2 Each shard creates a read-write set for all transactions in the chunk and broadcasts it to all other shards.
///              Shard 0                          Shard 1                           Shard 2
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///    |        Read-Write Set      |  |         Read-Write Set         |  |         Read-Write Set         |
///    |                            |  |                               |  |                               |
///    |   T1 {A, B}                |  |   T4 {D, E}                   |  |   T7 {G, H}                   |
///    |   T2 {B, C}                |  |   T5 {E, F}                   |  |   T8 {H, I}                   |
///    |   T3 {C, D}                |  |   T6 {F, G}                   |  |   T9 {I, J}                   |
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///   2.3 Each shard collects read-write sets from all other shards and discards transactions that have cross-shard dependencies.
///              Shard 0                          Shard 1                           Shard 2
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///    |        Discarded Txns      |  |         Discarded Txns         |  |         Discarded Txns         |
///    |                            |  |                               |  |                               |
///    |   - T3 (cross-shard dependency with T4) |  |   - T6 (cross-shard dependency with T7) |  | No discard |
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///   2.4 Each shard broadcasts the number of transactions that it plans to put in the current chunk.
///              Shard 0                          Shard 1                           Shard 2
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///    |          Chunk Count       |  |          Chunk Count          |  |          Chunk Count          |
///    |                            |  |                               |  |                               |
///    |   2                        |  |   2                           |  |      3                        |
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///   2.5 Each shard collects the number of transactions that all other shards plan to put in the current chunk and based
///      on that, it finalizes the absolute index offset of the current chunk. It uses this information to create a read-write set
///      index, which is a mapping of all the storage location to the maximum absolute index of the transaction that read/wrote to that location.
///             Shard 0                          Shard 1                           Shard 2
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///    |          Index Offset      |  |          Index Offset         |  |          Index Offset         |
///    |                            |  |                               |  |                               |
///    |   0                        |  |   2                           |  |   4                           |
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///   2.6 It also uses the read-write set index mapping passed in previous iteration to add cross-shard dependencies to the transactions. This is
///     done by looking up the read-write set index for each storage location that a transaction reads/writes to and adding a cross-shard dependency
///   2.7 Returns two lists of transactions: one list of transactions that are discarded and another list of transactions that are kept.
/// 3. Use the discarded transactions to create new chunks and repeat the step 2 until N iterations.
/// 4. For remaining transaction chunks, add cross-shard dependencies to the transactions. This is done as follows:
///   4.1 Create a read-write set with index mapping for all the transactions in the remaining chunks.
///   4.2 Broadcast and collect read-write set with index mapping from all shards.
///   4.3 Add cross-shard dependencies to the transactions in the remaining chunks by looking up the read-write set index
///       for each storage location that a transaction reads/writes to. The idea is to find the maximum transaction index
///       that reads/writes to the same location and add that as a dependency. This can be done as follows: First look up the read-write set index
///       mapping received from other shards in current iteration in descending order of shard id. If the read-write set index is not found,
///       look up the read-write set index mapping received from other shards in previous iteration(s) in descending order of shard id.
/// ```
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
    ) -> Vec<Vec<PrePartitionedTxnIdx>> {
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
        let _timer = BLOCK_PARTITIONING_SECONDS.start_timer();
        // Step 0: pre-partition. Divide a list of transactions into `num_executor_shards` chunks.
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
        // Step 1: build some necessary indices for txn senders/storage locations.
        Self::init(&mut state);

        // Step 2: remove cross-shard dependencies by move some txns into new rounds.
        // As a result, we get a txn matrix of no more than `self.max_partitioning_rounds` rows and exactly `num_executor_shards` columns.
        // It's guaranteed that inside every round other than the last round, there's no cross-shard dependency. (But cross-round dependencies are always possible.)
        Self::remove_cross_shard_dependencies(&mut state);

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
