// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error, remote_executor_client::RemoteExecutorClient, BlockExecutionRequest,
    BlockExecutionResult,
};
use aptos_logger::{error, info};
use aptos_secure_net::NetworkServer;
use aptos_vm::sharded_block_executor::block_executor_client::{
    BlockExecutorClient, VMExecutorClient,
};
use std::net::SocketAddr;

/// A service that provides support for remote execution. Essentially, it reads a request from
/// the remote executor client and executes the block locally and returns the result.
pub struct ExecutorService {
    client: VMExecutorClient,
}

impl ExecutorService {
    pub fn new(num_executor_threads: usize) -> Self {
        Self {
            client: VMExecutorClient::new(num_executor_threads),
        }
    }

    pub fn handle_message(&self, execution_message: Vec<u8>) -> Result<Vec<u8>, Error> {
        let input = bcs::from_bytes(&execution_message)?;
        let result = self.handle_execution_request(input)?;
        Ok(bcs::to_bytes(&result)?)
    }

    pub fn handle_execution_request(
        &self,
        execution_request: BlockExecutionRequest,
    ) -> Result<BlockExecutionResult, Error> {
        let result = match execution_request {
            BlockExecutionRequest::ExecuteBlock(command) => self.client.execute_block(
                command.sub_blocks,
                &command.state_view,
                command.concurrency_level,
                command.maybe_block_gas_limit,
            ),
        };
        Ok(BlockExecutionResult { inner: result })
    }
}

pub trait RemoteExecutorService {
    fn client(&self) -> RemoteExecutorClient {
        RemoteExecutorClient::new(self.server_address(), self.network_timeout_ms())
    }

    fn server_address(&self) -> SocketAddr;

    /// Network Timeout in milliseconds.
    fn network_timeout_ms(&self) -> u64;

    fn executor_threads(&self) -> usize;
}

pub fn execute(mut network_server: NetworkServer, executor_service: ExecutorService) {
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
    info!("server sending response");
    network_server.write(&response)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        remote_executor_service::RemoteExecutorService,
        thread_executor_service::ThreadExecutorService,
    };
    use aptos_language_e2e_tests::{
        account::AccountData, common_transactions::peer_to_peer_txn, executor::FakeExecutor,
    };
    use aptos_types::{
        account_config::{DepositEvent, WithdrawEvent},
        block_executor::partitioner::{
            CrossShardDependencies, SubBlock, SubBlocksForShard, TransactionWithDependencies,
        },
        transaction::{ExecutionStatus, Transaction, TransactionOutput, TransactionStatus},
    };
    use aptos_vm::sharded_block_executor::{
        block_executor_client::BlockExecutorClient, ShardedBlockExecutor,
    };
    use std::sync::Arc;

    fn generate_transactions(executor: &mut FakeExecutor) -> (Vec<Transaction>, AccountData) {
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
        (txns, receiver)
    }

    fn verify_txn_output(
        transfer_amount: u64,
        output: &[TransactionOutput],
        executor: &mut FakeExecutor,
        receiver: &AccountData,
    ) {
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

    #[test]
    fn test_remote_block_execute() {
        let executor_service = ThreadExecutorService::new(5000, 2);
        // Uncomment for testing with a real server
        // let server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
        // let client = RemoteExecutorClient::new(server_addr, 1000);

        let client = executor_service.client();
        let mut executor = FakeExecutor::from_head_genesis();
        for _ in 0..5 {
            let (txns, receiver) = generate_transactions(&mut executor);
            let txns_with_deps = txns
                .into_iter()
                .map(|txn| TransactionWithDependencies::new(txn, CrossShardDependencies::default()))
                .collect::<Vec<_>>();
            let sub_block = SubBlock::new(0, txns_with_deps);
            let sub_blocks_for_shard = SubBlocksForShard::new(0, vec![sub_block]);

            let output = client
                .execute_block(sub_blocks_for_shard, executor.data_store(), 2, None)
                .unwrap();
            verify_txn_output(1_000, &output[0], &mut executor, &receiver);
        }
    }

    #[test]
    fn test_sharded_remote_block_executor() {
        let executor_service = ThreadExecutorService::new(5000, 2);
        let client = executor_service.client();
        // Uncomment for testing with a real server
        // let server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
        // let client = RemoteExecutorClient::new(server_addr, 1000);

        let sharded_block_executor = ShardedBlockExecutor::new(vec![client]);
        let mut executor = FakeExecutor::from_head_genesis();
        for _ in 0..5 {
            let (txns, receiver) = generate_transactions(&mut executor);
            let txns_with_deps = txns
                .into_iter()
                .map(|txn| TransactionWithDependencies::new(txn, CrossShardDependencies::default()))
                .collect::<Vec<_>>();
            let sub_block = SubBlock::new(0, txns_with_deps);
            let sub_blocks_for_shard = SubBlocksForShard::new(0, vec![sub_block]);

            let output = sharded_block_executor
                .execute_block(
                    Arc::new(executor.data_store().clone()),
                    vec![sub_blocks_for_shard],
                    2,
                    None,
                )
                .unwrap();
            verify_txn_output(1_000, &output, &mut executor, &receiver);
        }
    }
}
