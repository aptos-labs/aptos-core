// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    remote_cordinator_client::RemoteCoordinatorClient,
    remote_cross_shard_client::RemoteCrossShardClient,
    remote_executor_client::RemoteExecutorClient,
};
use aptos_secure_net::network_controller::NetworkController;
use aptos_state_view::in_memory_state_view::InMemoryStateView;
use aptos_types::block_executor::partitioner::ShardId;
use aptos_vm::sharded_block_executor::sharded_executor_service::ShardedExecutorService;
use std::{net::SocketAddr, sync::Arc};

/// A service that provides support for remote execution. Essentially, it reads a request from
/// the remote executor client and executes the block locally and returns the result.
pub struct ExecutorService {
    executor_service: Arc<ShardedExecutorService<InMemoryStateView>>,
}

impl ExecutorService {
    pub fn new(
        shard_id: ShardId,
        num_shards: usize,
        num_threads: usize,
        self_address: SocketAddr,
        coordinator_address: SocketAddr,
        remote_shard_addresses: Vec<SocketAddr>,
    ) -> Self {
        let mut controller = NetworkController::new("executor_service", self_address, 5000);
        let coordinator_client = Arc::new(RemoteCoordinatorClient::new(
            &mut controller,
            coordinator_address,
        ));
        let cross_shard_client = Arc::new(RemoteCrossShardClient::new(
            &mut controller,
            remote_shard_addresses,
        ));

        let executor_service = Arc::new(ShardedExecutorService::new(
            shard_id,
            num_shards,
            num_threads,
            coordinator_client,
            cross_shard_client,
        ));

        Self { executor_service }
    }

    pub fn start(&self) {
        self.executor_service.start();
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

#[cfg(test)]
mod tests {
    // use crate::{
    //     remote_executor_service::RemoteExecutorService,
    //     thread_executor_service::ThreadExecutorService,
    // };
    // use aptos_language_e2e_tests::{
    //     account::AccountData, common_transactions::peer_to_peer_txn, executor::FakeExecutor,
    // };
    // use aptos_types::{
    //     account_config::{DepositEvent, WithdrawEvent},
    //     block_executor::partitioner::{
    //         CrossShardDependencies, SubBlock, SubBlocksForShard, TransactionWithDependencies,
    //     },
    //     transaction::{ExecutionStatus, Transaction, TransactionOutput, TransactionStatus},
    // };
    // use aptos_vm::sharded_block_executor::{
    //     block_executor_client::BlockExecutorClient, ShardedBlockExecutor,
    // };
    // use std::sync::Arc;
    //
    // fn generate_transactions(executor: &mut FakeExecutor) -> (Vec<Transaction>, AccountData) {
    //     let sender = executor.create_raw_account_data(3_000_000_000, 10);
    //     let receiver = executor.create_raw_account_data(3_000_000_000, 10);
    //     executor.add_account_data(&sender);
    //     executor.add_account_data(&receiver);
    //
    //     let transfer_amount = 1_000;
    //
    //     // execute transaction
    //     let txns: Vec<Transaction> = vec![
    //         Transaction::UserTransaction(peer_to_peer_txn(
    //             sender.account(),
    //             receiver.account(),
    //             10,
    //             transfer_amount,
    //             100,
    //         )),
    //         Transaction::UserTransaction(peer_to_peer_txn(
    //             sender.account(),
    //             receiver.account(),
    //             11,
    //             transfer_amount,
    //             100,
    //         )),
    //         Transaction::UserTransaction(peer_to_peer_txn(
    //             sender.account(),
    //             receiver.account(),
    //             12,
    //             transfer_amount,
    //             100,
    //         )),
    //         Transaction::UserTransaction(peer_to_peer_txn(
    //             sender.account(),
    //             receiver.account(),
    //             13,
    //             transfer_amount,
    //             100,
    //         )),
    //     ];
    //     (txns, receiver)
    // }
    //
    // fn verify_txn_output(
    //     transfer_amount: u64,
    //     output: &[TransactionOutput],
    //     executor: &mut FakeExecutor,
    //     receiver: &AccountData,
    // ) {
    //     for (idx, txn_output) in output.iter().enumerate() {
    //         assert_eq!(
    //             txn_output.status(),
    //             &TransactionStatus::Keep(ExecutionStatus::Success)
    //         );
    //
    //         // check events
    //         for event in txn_output.events() {
    //             if let Ok(payload) = WithdrawEvent::try_from(event) {
    //                 assert_eq!(transfer_amount, payload.amount());
    //             } else if let Ok(payload) = DepositEvent::try_from(event) {
    //                 if payload.amount() == 0 {
    //                     continue;
    //                 }
    //                 assert_eq!(transfer_amount, payload.amount());
    //             } else {
    //                 panic!("Unexpected Event Type")
    //             }
    //         }
    //
    //         let original_receiver_balance = executor
    //             .read_coin_store_resource(receiver.account())
    //             .expect("receiver balcne must exist");
    //         executor.apply_write_set(txn_output.write_set());
    //
    //         // check that numbers in stored DB are correct
    //         let receiver_balance = original_receiver_balance.coin() + transfer_amount;
    //         let updated_receiver_balance = executor
    //             .read_coin_store_resource(receiver.account())
    //             .expect("receiver balance must exist");
    //         assert_eq!(receiver_balance, updated_receiver_balance.coin());
    //         assert_eq!(
    //             idx as u64 + 1,
    //             updated_receiver_balance.deposit_events().count()
    //         );
    //     }
    // }

    // #[test]
    // fn test_remote_block_execute() {
    //     let executor_service = ThreadExecutorService::new(5000, 2);
    //     // Uncomment for testing with a real server
    //     // let server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
    //     // let client = RemoteExecutorClient::new(server_addr, 1000);
    //
    //     let client = executor_service.client();
    //     let mut executor = FakeExecutor::from_head_genesis();
    //     for _ in 0..5 {
    //         let (txns, receiver) = generate_transactions(&mut executor);
    //         let txns_with_deps = txns
    //             .into_iter()
    //             .map(|txn| TransactionWithDependencies::new(txn, CrossShardDependencies::default()))
    //             .collect::<Vec<_>>();
    //         let sub_block = SubBlock::new(0, txns_with_deps);
    //         let sub_blocks_for_shard = SubBlocksForShard::new(0, vec![sub_block]);
    //
    //         let output = client
    //             .execute_block(sub_blocks_for_shard, executor.data_store(), 2, None)
    //             .unwrap();
    //         verify_txn_output(1_000, &output[0], &mut executor, &receiver);
    //     }
    // }
    //
    // #[test]
    // fn test_sharded_remote_block_executor() {
    //     let executor_service = ThreadExecutorService::new(5000, 2);
    //     let client = executor_service.client();
    //     // Uncomment for testing with a real server
    //     // let server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
    //     // let client = RemoteExecutorClient::new(server_addr, 1000);
    //
    //     let sharded_block_executor = ShardedBlockExecutor::new(vec![client]);
    //     let mut executor = FakeExecutor::from_head_genesis();
    //     for _ in 0..5 {
    //         let (txns, receiver) = generate_transactions(&mut executor);
    //         let txns_with_deps = txns
    //             .into_iter()
    //             .map(|txn| TransactionWithDependencies::new(txn, CrossShardDependencies::default()))
    //             .collect::<Vec<_>>();
    //         let sub_block = SubBlock::new(0, txns_with_deps);
    //         let sub_blocks_for_shard = SubBlocksForShard::new(0, vec![sub_block]);
    //
    //         let output = sharded_block_executor
    //             .execute_block(
    //                 Arc::new(executor.data_store().clone()),
    //                 vec![sub_blocks_for_shard],
    //                 2,
    //                 None,
    //             )
    //             .unwrap();
    //         verify_txn_output(1_000, &output, &mut executor, &receiver);
    //     }
    // }
}
