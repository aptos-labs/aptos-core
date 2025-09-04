// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfigFromOnchain,
        partitioner::{ShardId, SubBlocksForShard},
    },
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{TransactionOutput, analyzed_transaction::AnalyzedTransaction},
    vm_status::VMStatus,
};
use serde::{Deserialize, Serialize};

mod error;
pub mod local_executor_helper;
mod metrics;
pub mod process_executor_service;
mod remote_cordinator_client;
mod remote_cross_shard_client;
pub mod remote_executor_client;
pub mod remote_executor_service;
mod remote_state_view;
mod remote_state_view_service;
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
    pub(crate) concurrency_level: usize,
    pub(crate) onchain_config: BlockExecutorConfigFromOnchain,
}

impl ExecuteBlockCommand {
    pub fn into(
        self,
    ) -> (
        SubBlocksForShard<AnalyzedTransaction>,
        usize,
        BlockExecutorConfigFromOnchain,
    ) {
        (self.sub_blocks, self.concurrency_level, self.onchain_config)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RemoteKVRequest {
    pub(crate) shard_id: ShardId,
    pub(crate) keys: Vec<StateKey>,
}

impl RemoteKVRequest {
    pub fn new(shard_id: ShardId, keys: Vec<StateKey>) -> Self {
        Self { shard_id, keys }
    }

    pub fn into(self) -> (ShardId, Vec<StateKey>) {
        (self.shard_id, self.keys)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RemoteKVResponse {
    pub(crate) inner: Vec<(StateKey, Option<StateValue>)>,
}

impl RemoteKVResponse {
    pub fn new(inner: Vec<(StateKey, Option<StateValue>)>) -> Self {
        Self { inner }
    }
}
