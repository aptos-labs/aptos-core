// Copyright © Aptos Foundation

use crate::sharded_block_executor::{messages::CrossShardMsg, ExecutorShardCommand};
use aptos_state_view::StateView;
use aptos_types::{
    block_executor::partitioner::{RoundId, ShardId, SubBlocksForShard},
    transaction::{analyzed_transaction::AnalyzedTransaction, TransactionOutput},
};
use move_core_types::vm_status::VMStatus;
use std::sync::Arc;

// Interface to communicate from the block executor coordinator to the executor shards.
pub trait ExecutorClient<S: StateView + Sync + Send + 'static>: Send + Sync {
    fn num_shards(&self) -> usize;
    fn execute_block(
        &self,
        state_view: Arc<S>,
        block: Vec<SubBlocksForShard<AnalyzedTransaction>>,
        concurrency_level_per_shard: usize,
        maybe_block_gas_limit: Option<u64>,
    );

    fn get_execution_result(&self) -> Result<Vec<Vec<Vec<TransactionOutput>>>, VMStatus>;
}

// Interface to communicate from the executor shards to the block executor coordinator.
pub trait CoordinatorClient<S: StateView + Sync + Send + 'static>: Send + Sync {
    fn receive_execute_command(&self) -> ExecutorShardCommand<S>;

    fn send_execution_result(&self, result: Result<Vec<Vec<TransactionOutput>>, VMStatus>);
}

// CrossShardClient is a trait that defines the interface for sending and receiving messages across
// shards.
pub trait CrossShardClient: Send + Sync {
    fn send_cross_shard_msg(&self, shard_id: ShardId, round: RoundId, msg: CrossShardMsg);

    fn receive_cross_shard_msg(&self, current_round: RoundId) -> CrossShardMsg;
}
