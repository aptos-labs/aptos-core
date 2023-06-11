// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{error::Error, BlockExecutionRequest, BlockExecutionResult, ExecuteBlockCommand};
use aptos_logger::error;
use aptos_retrier::{fixed_retry_strategy, retry};
use aptos_secure_net::NetworkClient;
use aptos_state_view::StateView;
use aptos_types::{
    transaction::{Transaction, TransactionOutput},
    vm_status::VMStatus,
};
use aptos_vm::sharded_block_executor::block_executor_client::BlockExecutorClient;
use std::{net::SocketAddr, sync::Mutex};

/// An implementation of [`BlockExecutorClient`] that supports executing blocks remotely.
pub struct RemoteExecutorClient {
    network_client: Mutex<NetworkClient>,
}

impl RemoteExecutorClient {
    pub fn new(server_address: SocketAddr, network_timeout_ms: u64) -> Self {
        let network_client = NetworkClient::new(
            "remote-executor-service",
            server_address,
            network_timeout_ms,
        );
        Self {
            network_client: Mutex::new(network_client),
        }
    }

    fn execute_block_inner(
        &self,
        execution_request: BlockExecutionRequest,
    ) -> Result<BlockExecutionResult, Error> {
        let input_message = bcs::to_bytes(&execution_request)?;
        let mut network_client = self.network_client.lock().unwrap();
        network_client.write(&input_message)?;
        let bytes = network_client.read()?;
        Ok(bcs::from_bytes(&bytes)?)
    }

    fn execute_block_with_retry(
        &self,
        execution_request: BlockExecutionRequest,
    ) -> BlockExecutionResult {
        retry(fixed_retry_strategy(5, 20), || {
            let res = self.execute_block_inner(execution_request.clone());
            if let Err(e) = &res {
                error!("Failed to execute block: {:?}", e);
            }
            res
        })
        .unwrap()
    }
}

impl BlockExecutorClient for RemoteExecutorClient {
    fn execute_block<S: StateView + Sync>(
        &self,
        transactions: Vec<Transaction>,
        state_view: &S,
        concurrency_level: usize,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        let input = BlockExecutionRequest::ExecuteBlock(ExecuteBlockCommand {
            transactions,
            state_view: S::as_in_memory_state_view(state_view),
            concurrency_level,
            maybe_block_gas_limit,
        });
        self.execute_block_with_retry(input).inner
    }
}
