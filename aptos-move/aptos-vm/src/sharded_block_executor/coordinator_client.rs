// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::sharded_block_executor::ExecutorShardCommand;
use aptos_state_view::StateView;
use aptos_types::transaction::TransactionOutput;
use async_trait::async_trait;
use move_core_types::vm_status::VMStatus;

// Interface to communicate from the executor shards to the block executor coordinator.
#[async_trait]
pub trait CoordinatorClient<S: StateView + Sync + Send + 'static>: Send + Sync {
    async fn receive_execute_command(&mut self) -> ExecutorShardCommand<S>;

    fn send_execution_result(&self, result: Result<Vec<Vec<TransactionOutput>>, VMStatus>);
}
