// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use aptos_state_view::in_memory_state_view::InMemoryStateView;
use aptos_types::{
    block_executor::partitioner::SubBlocksForShard,
    transaction::{analyzed_transaction::AnalyzedTransaction, TransactionOutput},
    vm_status::VMStatus,
};
use serde::{Deserialize, Serialize};

mod error;
pub mod process_executor_service;
mod remote_cordinator_client;
mod remote_cross_shard_client;
mod remote_executor_client;
pub mod remote_executor_service;
#[cfg(test)]
mod test_utils;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod thread_executor_service;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RemoteExecutionResult {
    pub inner: Result<Vec<Vec<TransactionOutput>>, VMStatus>,
}

impl RemoteExecutionResult {
    pub fn new(inner: Result<Vec<Vec<TransactionOutput>>, VMStatus>) -> Self {
        Self { inner }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum RemoteExecutionRequest {
    ExecuteBlock(ExecuteBlockCommand),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExecuteBlockCommand {
    pub(crate) sub_blocks: SubBlocksForShard<AnalyzedTransaction>,
    // Currently we only support the state view backed by in-memory hashmap, which means that
    // the controller needs to pre-read all the KV pairs from the storage and pass them to the
    // executor service. In the future, we will support other types of state view, e.g., the
    // state view backed by remote storage service, which will allow the executor service to read the KV pairs
    // directly from the storage.
    pub(crate) state_view: InMemoryStateView,
    pub(crate) concurrency_level: usize,
    pub(crate) maybe_block_gas_limit: Option<u64>,
}

impl ExecuteBlockCommand {
    pub fn into(
        self,
    ) -> (
        SubBlocksForShard<AnalyzedTransaction>,
        InMemoryStateView,
        usize,
        Option<u64>,
    ) {
        (
            self.sub_blocks,
            self.state_view,
            self.concurrency_level,
            self.maybe_block_gas_limit,
        )
    }
}
