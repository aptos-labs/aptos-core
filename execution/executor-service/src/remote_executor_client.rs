// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{error::Error, BlockExecutionRequest, BlockExecutionResult, ExecuteBlockCommand};
use aptos_logger::{error, info};
use aptos_secure_net::NetworkClient;
use aptos_state_view::StateView;
use aptos_types::{
    transaction::{Transaction, TransactionOutput},
    vm_status::{
        StatusCode::{REMOTE_EXECUTION_SERVER_READ_ERROR, REMOTE_EXECUTION_SERVER_WRITE_ERROR},
        VMStatus,
    },
};
use aptos_vm::sharded_block_executor::block_executor_client::TBlockExecutorClient;
use std::{net::SocketAddr, sync::Mutex};

pub struct RemoteExecutorClient {
    // fields omitted
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

    fn process_one_message(&self, input: &[u8]) -> Result<Vec<u8>, Error> {
        let mut network_client = self.network_client.lock().unwrap();
        network_client.write(input)?;
        Ok(network_client.read()?)
    }
}

impl TBlockExecutorClient for RemoteExecutorClient {
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
        let input_message = serde_json::to_vec(&input).map_err(|e| {
            VMStatus::Error(
                REMOTE_EXECUTION_SERVER_WRITE_ERROR,
                Some(format!(
                    "Failed to serialize request to remote execution server: {}",
                    e
                )),
            )
        })?;
        loop {
            info!("Sending request to Executor service");
            match self.process_one_message(&input_message) {
                Err(err) => {
                    error!("Failed to communicate with Executor service: {}", err)
                },
                Ok(value) => {
                    let result =
                        serde_json::from_slice::<BlockExecutionResult>(&value).map_err(|e| {
                            VMStatus::Error(
                                REMOTE_EXECUTION_SERVER_READ_ERROR,
                                Some(format!(
                                "Failed to deserialize response from remote execution server: {}",
                                e
                            )),
                            )
                        });
                    return result?.inner;
                },
            }
        }
    }
}
