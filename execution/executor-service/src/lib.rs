// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use aptos_state_view::in_memory_state_view::InMemoryStateView;
use aptos_types::{
    block_executor::partitioner::SubBlocksForShard,
    transaction::{Transaction, TransactionOutput},
    vm_status::VMStatus,
};
use serde::{Deserialize, Serialize};

mod error;
pub mod process_executor_service;
pub mod remote_executor_client;
pub mod remote_executor_service;
#[cfg(test)]
mod thread_executor_service;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BlockExecutionResult {
    pub inner: Result<Vec<Vec<TransactionOutput>>, VMStatus>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum BlockExecutionRequest {
    ExecuteBlock(ExecuteBlockCommand),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExecuteBlockCommand {
    pub(crate) sub_blocks: SubBlocksForShard<Transaction>,
    // Currently we only support the state view backed by in-memory hashmap, which means that
    // the controller needs to pre-read all the KV pairs from the storage and pass them to the
    // executor service. In the future, we will support other types of state view, e.g., the
    // state view backed by remote storage service, which will allow the executor service to read the KV pairs
    // directly from the storage.
    pub(crate) state_view: InMemoryStateView,
    pub(crate) concurrency_level: usize,
    pub(crate) maybe_block_gas_limit: Option<u64>,
}
