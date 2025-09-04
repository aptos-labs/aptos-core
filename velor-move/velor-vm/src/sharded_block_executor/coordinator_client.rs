// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::sharded_block_executor::ExecutorShardCommand;
use velor_types::{state_store::StateView, transaction::TransactionOutput};
use move_core_types::vm_status::VMStatus;

// Interface to communicate from the executor shards to the block executor coordinator.
pub trait CoordinatorClient<S: StateView + Sync + Send + 'static>: Send + Sync {
    fn receive_execute_command(&self) -> ExecutorShardCommand<S>;

    fn send_execution_result(&self, result: Result<Vec<Vec<TransactionOutput>>, VMStatus>);
}
