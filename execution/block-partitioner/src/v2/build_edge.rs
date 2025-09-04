// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::v2::{PartitionerV2, counters::MISC_TIMERS_SECONDS, state::PartitionState};
use aptos_metrics_core::TimerHelper;
use aptos_types::{
    block_executor::partitioner::{
        PartitionedTransactions, SubBlock, SubBlocksForShard, TransactionWithDependencies,
    },
    transaction::analyzed_transaction::AnalyzedTransaction,
};
use rayon::{
    iter::ParallelIterator,
    prelude::{IntoParallelIterator, IntoParallelRefIterator},
};
use std::sync::Mutex;

impl PartitionerV2 {
    pub(crate) fn add_edges(state: &mut PartitionState) -> PartitionedTransactions {
        let _timer = MISC_TIMERS_SECONDS.timer_with(&["add_edges"]);

        state.sub_block_matrix = state.thread_pool.install(|| {
            (0..state.num_rounds())
                .into_par_iter()
                .map(|_round_id| {
                    (0..state.num_executor_shards)
                        .into_par_iter()
                        .map(|_shard_id| Mutex::new(None))
                        .collect()
                })
                .collect()
        });

        state.thread_pool.install(|| {
            (0..state.num_rounds())
                .into_par_iter()
                .for_each(|round_id| {
                    (0..state.num_executor_shards)
                        .into_par_iter()
                        .for_each(|shard_id| {
                            let twds = state.finalized_txn_matrix[round_id][shard_id]
                                .par_iter()
                                .map(|&txn_idx1| {
                                    state.take_txn_with_dep(round_id, shard_id, txn_idx1)
                                })
                                .collect();
                            let sub_block =
                                SubBlock::new(state.start_index_matrix[round_id][shard_id], twds);
                            *state.sub_block_matrix[round_id][shard_id].lock().unwrap() =
                                Some(sub_block);
                        });
                });
        });

        let global_txns: Vec<TransactionWithDependencies<AnalyzedTransaction>> =
            if !state.partition_last_round {
                state
                    .sub_block_matrix
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

        let final_num_rounds = state.sub_block_matrix.len();
        let sharded_txns = (0..state.num_executor_shards)
            .map(|shard_id| {
                let sub_blocks: Vec<SubBlock<AnalyzedTransaction>> = (0..final_num_rounds)
                    .map(|round_id| {
                        state.sub_block_matrix[round_id][shard_id]
                            .lock()
                            .unwrap()
                            .take()
                            .unwrap()
                    })
                    .collect();
                SubBlocksForShard::new(shard_id, sub_blocks)
            })
            .collect();

        PartitionedTransactions::new(sharded_txns, global_txns)
    }
}
