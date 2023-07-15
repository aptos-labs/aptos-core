// Copyright © Aptos Foundation

use std::sync::Arc;
use crate::sharded_block_executor::{messages::CrossShardMsg, ExecutorShardCommand};
use aptos_state_view::StateView;
use aptos_types::{
    block_executor::partitioner::{RoundId, ShardId},
    transaction::TransactionOutput,
};
use crossbeam_channel::SendError;
use aptos_types::block_executor::partitioner::SubBlocksForShard;
use aptos_types::transaction::Transaction;
use move_core_types::vm_status::VMStatus;

pub trait ExecutorShard<S: StateView + Sync + Send + 'static> {
    fn start(&mut self);

    fn stop(&mut self);

    fn send_execute_command(&self, execute_command: ExecutorShardCommand<S>);

    fn get_execution_result(&self) -> Result<Vec<Vec<TransactionOutput>>, VMStatus>;
}

// Trait that defines the communication interface between the coordinator
// and the executor shard.
pub trait CoordinatorClient<S: StateView + Sync + Send + 'static>: Send + Sync {
    fn send_execute_command(
        &self,
        execute_command: ExecutorShardCommand<S>,
    ) -> Result<(), SendError<ExecutorShardCommand<S>>>;

    fn get_execution_result(&self) -> Result<Vec<Vec<TransactionOutput>>, VMStatus>;

    fn receive_execute_command(&self) -> ExecutorShardCommand<S>;

    fn send_execution_result(&self, result: Result<Vec<Vec<TransactionOutput>>, VMStatus>);
}

pub trait CoordinatorToExecutorClient<S: StateView + Sync + Send + 'static>: Send + Sync {
    fn num_shards(&self) -> usize;
    fn execute_block(&self,
                     state_view: Arc<S>,
                     block: Vec<SubBlocksForShard<Transaction>>,
                     concurrency_level_per_shard: usize,
                     maybe_block_gas_limit: Option<u64>);

    fn get_execution_result(&self) -> Result<Vec<Vec<Vec<TransactionOutput>>>, VMStatus>;
}

pub trait ExecutorToCoordinatorClient<S: StateView + Sync + Send + 'static>: Send + Sync {
    fn receive_execute_command(&self) -> ExecutorShardCommand<S>;

    fn send_execution_result(&self, result: Result<Vec<Vec<TransactionOutput>>, VMStatus>);
}

// CrossShardClient is a trait that defines the interface for sending and receiving messages across
// shards.
pub trait CrossShardClient: Send + Sync {

    fn send_cross_shard_msg(&self, shard_id: ShardId, round: RoundId, msg: CrossShardMsg);

    fn receive_cross_shard_msg(&self, current_round: RoundId) -> CrossShardMsg;
}
