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
use aptos_logger::{debug, info, trace};
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
use state::PartitionState;
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
use types::{PreParedTxnIdx, SenderIdx, ShardedTxnIndexV2, StorageKeyIdx, SubBlockIdx};

mod build_edge;
pub mod config;
mod conflicting_txn_tracker;
mod counters;
mod flatten_to_rounds;
mod init;
pub(crate) mod state;
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
        let thread_pool = Arc::new(
            ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
                .unwrap(),
        );
        Self {
            pre_partitioner: Box::new(UniformPartitioner {}), //TODO: parameterize it.
            thread_pool,
            num_rounds_limit,
            avoid_pct,
            dashmap_num_shards,
            merge_discarded,
        }
    }
}

impl BlockPartitioner for PartitionerV2 {
    fn partition(
        &self,
        txns: Vec<AnalyzedTransaction>,
        num_executor_shards: usize,
    ) -> PartitionedTransactions {
        let pre_partitioned = self
            .pre_partitioner
            .pre_partition(txns.as_slice(), num_executor_shards);

        let mut state = PartitionState::new(
            self.thread_pool.clone(),
            self.dashmap_num_shards,
            txns,
            num_executor_shards,
            pre_partitioned,
            self.num_rounds_limit,
            self.avoid_pct,
            self.merge_discarded,
        );
        Self::init(&mut state);
        Self::flatten_to_rounds(&mut state);
        let ret = Self::add_edges(&mut state);

        self.thread_pool.spawn(move || {
            drop(state);
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
