// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_state_view::StateView;
use aptos_types::{
    block_executor::partitioner::SubBlocksForShard,
    transaction::{analyzed_transaction::AnalyzedTransaction, TransactionOutput},
};
use move_core_types::vm_status::VMStatus;
use std::sync::Arc;

// Interface to communicate from the block executor coordinator to the executor shards.
pub trait ExecutorClient<S: StateView + Sync + Send + 'static>: Send + Sync {
    fn num_shards(&self) -> usize;

    // A non blocking call that sends the block to be executed by the executor shards.
    fn execute_block(
        &self,
        state_view: Arc<S>,
        block: Vec<SubBlocksForShard<AnalyzedTransaction>>,
        concurrency_level_per_shard: usize,
        maybe_block_gas_limit: Option<u64>,
    );

    // Blocking call that waits for the execution results from the executor shards. It returns the execution results
    // from each shard and in the sub-block order.
    fn get_execution_result(&self) -> Result<Vec<Vec<Vec<TransactionOutput>>>, VMStatus>;
}
