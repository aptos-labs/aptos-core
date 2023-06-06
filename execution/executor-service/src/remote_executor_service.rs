// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error, remote_executor_client::RemoteExecutorClient, BlockExecutionRequest,
    BlockExecutionResult,
};
use aptos_logger::{error, info};
use aptos_secure_net::{NetworkClient, NetworkServer};
use aptos_vm::sharded_block_executor::block_executor_client::{
    LocalExecutorClient, TBlockExecutorClient,
};
use std::net::SocketAddr;

pub struct ExecutorService {
    client: LocalExecutorClient,
}

impl ExecutorService {
    pub fn new(num_executor_threads: usize) -> Self {
        Self {
            client: LocalExecutorClient::new(num_executor_threads),
        }
    }

    pub fn handle_message(&self, execution_message: Vec<u8>) -> Result<Vec<u8>, Error> {
        let input = serde_json::from_slice(&execution_message)?;
        let result = self.handle_execution_request(input)?;
        Ok(serde_json::to_vec(&result)?)
    }

    pub fn handle_execution_request(
        &self,
        execution_request: BlockExecutionRequest,
    ) -> Result<BlockExecutionResult, Error> {
        //println!("server executing block");
        let result = match execution_request {
            BlockExecutionRequest::ExecuteBlock(command) => self.client.execute_block(
                command.transactions,
                &command.state_view,
                command.concurrency_level,
                command.maybe_block_gas_limit,
            ),
        };
        //println!("server sending result: {:?}", result);
        Ok(BlockExecutionResult { inner: result })
    }
}

pub trait RemoteExecutorService {
    fn client(&self) -> RemoteExecutorClient {
        let network_client = NetworkClient::new(
            "remote-executor-service",
            self.server_address(),
            self.network_timeout_ms(),
        );
        RemoteExecutorClient::new(network_client)
    }

    fn server_address(&self) -> SocketAddr;

    /// Network Timeout in milliseconds.
    fn network_timeout_ms(&self) -> u64;

    fn executor_threads(&self) -> usize;
}

pub fn execute(listen_addr: SocketAddr, network_timeout_ms: u64, num_executor_threads: usize) {
    info!("Starting remote executor service on {}", listen_addr);
    let mut network_server =
        NetworkServer::new("thread-executor-service", listen_addr, network_timeout_ms);

    let executor_service = ExecutorService::new(num_executor_threads);

    loop {
        if let Err(e) = process_one_message(&mut network_server, &executor_service) {
            error!("Failed to process message: {}", e);
        }
    }
}

fn process_one_message(
    network_server: &mut NetworkServer,
    executor_service: &ExecutorService,
) -> Result<(), Error> {
    let request = network_server.read()?;
    let response = executor_service.handle_message(request)?;
    network_server.write(&response)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        remote_executor_service::RemoteExecutorService,
        thread_executor_service::ThreadExecutorService,
    };
    use aptos_language_e2e_tests::{common_transactions::peer_to_peer_txn, executor::FakeExecutor};
    use aptos_types::{
        account_config::{DepositEvent, WithdrawEvent},
        transaction::{ExecutionStatus, Transaction, TransactionStatus},
    };
    use aptos_vm::sharded_block_executor::block_executor_client::TBlockExecutorClient;

    #[test]
    fn test_remote_execute() {
        let executor_service = ThreadExecutorService::new(1000, 2);
        let client = executor_service.client();
        let mut executor = FakeExecutor::from_head_genesis();

        let sender = executor.create_raw_account_data(3_000_000_000, 10);
        let receiver = executor.create_raw_account_data(3_000_000_000, 10);
        executor.add_account_data(&sender);
        executor.add_account_data(&receiver);

        let transfer_amount = 1_000;

        // execute transaction
        let txns: Vec<Transaction> = vec![
            Transaction::UserTransaction(peer_to_peer_txn(
                sender.account(),
                receiver.account(),
                10,
                transfer_amount,
                100,
            )),
            Transaction::UserTransaction(peer_to_peer_txn(
                sender.account(),
                receiver.account(),
                11,
                transfer_amount,
                100,
            )),
            Transaction::UserTransaction(peer_to_peer_txn(
                sender.account(),
                receiver.account(),
                12,
                transfer_amount,
                100,
            )),
            Transaction::UserTransaction(peer_to_peer_txn(
                sender.account(),
                receiver.account(),
                13,
                transfer_amount,
                100,
            )),
        ];
        let output = client
            .execute_block(txns, executor.data_store(), 2, None)
            .unwrap();
        for (idx, txn_output) in output.iter().enumerate() {
            assert_eq!(
                txn_output.status(),
                &TransactionStatus::Keep(ExecutionStatus::Success)
            );

            // check events
            for event in txn_output.events() {
                if let Ok(payload) = WithdrawEvent::try_from(event) {
                    assert_eq!(transfer_amount, payload.amount());
                } else if let Ok(payload) = DepositEvent::try_from(event) {
                    if payload.amount() == 0 {
                        continue;
                    }
                    assert_eq!(transfer_amount, payload.amount());
                } else {
                    panic!("Unexpected Event Type")
                }
            }

            let original_receiver_balance = executor
                .read_coin_store_resource(receiver.account())
                .expect("receiver balcne must exist");
            executor.apply_write_set(txn_output.write_set());

            // check that numbers in stored DB are correct
            let receiver_balance = original_receiver_balance.coin() + transfer_amount;
            let updated_receiver_balance = executor
                .read_coin_store_resource(receiver.account())
                .expect("receiver balance must exist");
            assert_eq!(receiver_balance, updated_receiver_balance.coin());
            assert_eq!(
                idx as u64 + 1,
                updated_receiver_balance.deposit_events().count()
            );
        }
    }
}
