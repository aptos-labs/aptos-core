// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::sharded_block_executor::ExecutorShardCommand;
use aptos_types::{state_store::StateView, transaction::TransactionOutput};
use move_core_types::vm_status::VMStatus;

// Interface to communicate from the executor shards to the block executor coordinator.
pub trait CoordinatorClient<S: StateView + Sync + Send + 'static>: Send + Sync {
    fn receive_execute_command(&self) -> ExecutorShardCommand<S>;

    fn send_execution_result(&self, result: Result<Vec<Vec<TransactionOutput>>, VMStatus>);
}
