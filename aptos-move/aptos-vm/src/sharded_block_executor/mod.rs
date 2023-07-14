// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::sharded_block_executor::{counters::NUM_EXECUTOR_SHARDS, executor_shard::ExecutorShard};
use aptos_logger::{info, trace};
use aptos_state_view::StateView;
use aptos_types::{
    block_executor::partitioner::SubBlocksForShard,
    transaction::{analyzed_transaction::AnalyzedTransaction, TransactionOutput},
};
use move_core_types::vm_status::VMStatus;
use std::{marker::PhantomData, sync::Arc};

pub mod block_executor_client;
mod counters;
mod cross_shard_client;
mod cross_shard_state_view;
pub mod executor_shard;
pub mod local_executor_shard;
pub mod messages;
pub mod sharded_executor_service;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod test_utils;

/// Coordinator for sharded block executors that manages multiple shards and aggregates the results.
pub struct ShardedBlockExecutor<S: StateView + Sync + Send + 'static, E: ExecutorShard<S>> {
    executor_shards: Vec<E>,
    phantom: PhantomData<S>,
}

pub enum ExecutorShardCommand<S> {
    ExecuteSubBlocks(
        Arc<S>,
        SubBlocksForShard<AnalyzedTransaction>,
        usize,
        Option<u64>,
    ),
    Stop,
}

impl<S: StateView + Sync + Send + 'static, E: ExecutorShard<S>> ShardedBlockExecutor<S, E> {
    pub fn new(mut executor_shards: Vec<E>) -> Self {
        info!(
            "Creating a new ShardedBlockExecutor with {} shards",
            executor_shards.len()
        );
        executor_shards.iter_mut().for_each(|shard| shard.start());
        Self {
            executor_shards,
            phantom: PhantomData,
        }
    }

    pub fn num_shards(&self) -> usize {
        self.executor_shards.len()
    }

    /// Execute a block of transactions in parallel by splitting the block into num_remote_executors partitions and
    /// dispatching each partition to a remote executor shard.
    pub fn execute_block(
        &self,
        state_view: Arc<S>,
        block: Vec<SubBlocksForShard<AnalyzedTransaction>>,
        concurrency_level_per_shard: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        let num_executor_shards = self.executor_shards.len();
        NUM_EXECUTOR_SHARDS.set(num_executor_shards as i64);
        assert_eq!(
            num_executor_shards,
            block.len(),
            "Block must be partitioned into {} sub-blocks",
            num_executor_shards
        );
        for (i, sub_blocks_for_shard) in block.into_iter().enumerate() {
            self.executor_shards[i].send_execute_command(ExecutorShardCommand::ExecuteSubBlocks(
                state_view.clone(),
                sub_blocks_for_shard,
                concurrency_level_per_shard,
                maybe_block_gas_limit,
            ))
        }
        // wait for all remote executors to send the result back and append them in order by shard id
        let mut results = vec![];
        trace!("ShardedBlockExecutor Waiting for results");
        for i in 0..num_executor_shards {
            trace!("ShardedBlockExecutor Waiting for result from shard {}", i);
            results.push(self.executor_shards[i].get_execution_result()?);
        }
        trace!("ShardedBlockExecutor Received all results");
        let num_rounds = results[0].len();
        let mut aggreate_results = vec![];
        let mut ordered_results = vec![vec![]; num_executor_shards * num_rounds];
        for (shard_id, results_from_shard) in results.into_iter().enumerate() {
            for (round, result) in results_from_shard.into_iter().enumerate() {
                ordered_results[round * num_executor_shards + shard_id] = result;
            }
        }

        for result in ordered_results.into_iter() {
            aggreate_results.extend(result);
        }

        Ok(aggreate_results)
    }
}

impl<S: StateView + Sync + Send + 'static, E: ExecutorShard<S>> Drop
    for ShardedBlockExecutor<S, E>
{
    /// Best effort stops all the executor shards and waits for the thread to finish.
    fn drop(&mut self) {
        for executor_shard in self.executor_shards.iter_mut() {
            executor_shard.stop();
        }
    }
}
