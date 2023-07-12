// Copyright © Aptos Foundation

use crate::sharded_block_executor::ExecutorShardCommand;
use aptos_state_view::StateView;
use aptos_types::{block_executor::partitioner::ShardId, transaction::TransactionOutput};
use move_core_types::vm_status::VMStatus;
use std::sync::mpsc::{Receiver, Sender};

pub trait ExecutorShard<S: StateView + Sync + Send + 'static> {
    fn start(&self);

    fn stop(&self);

    fn get_command_sender(&self) -> Sender<ExecutorShardCommand<S>>;

    fn take_result_receiver(&self) -> Receiver<Result<Vec<Vec<TransactionOutput>>, VMStatus>>;

    fn get_shard_id(&self) -> ShardId;

    fn get_num_shards(&self) -> usize;
}
