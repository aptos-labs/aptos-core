// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

/// It has to be integration tests because otherwise it forms an indirect dependency circle between
/// aptos-vm and aptos-language-e2e-tests, which causes static variables to have two instances in
/// the same process while testing, resulting in the counters failing to register with "AlreadyReg"
/// error.
use aptos_block_partitioner::{
    pre_partition::{
        connected_component::config::ConnectedComponentPartitionerConfig,
        uniform_partitioner::config::UniformPartitionerConfig,
    },
    v2::config::PartitionerV2Config,
    PartitionerConfig,
};
use aptos_vm::sharded_block_executor::{
    local_executor_shard::LocalExecutorService, ShardedBlockExecutor,
};
use rand::{rngs::OsRng, Rng};
use rstest::rstest;

#[rstest(
    sender_stateless_account,
    receiver_stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, true, false, false),
    case(true, true, true, false),
    case(true, true, true, true),
    case(true, false, false, false),
    case(true, false, true, false),
    case(true, false, true, true),
    case(false, true, false, false),
    case(false, true, true, false),
    case(false, true, true, true),
    case(false, false, false, false),
    case(false, false, true, false),
    case(false, false, true, true)
)]
fn test_partitioner_v2_uniform_sharded_block_executor_no_conflict(
    sender_stateless_account: bool,
    receiver_stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    for merge_discard in [false, true] {
        let num_shards = 8;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(2));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .partition_last_round(merge_discard)
            .pre_partitioner_config(Box::new(UniformPartitionerConfig {}))
            .build();
        test_utils::test_sharded_block_executor_no_conflict(
            partitioner,
            sharded_block_executor,
            sender_stateless_account,
            receiver_stateless_account,
            use_txn_payload_v2_format,
            use_orderless_transactions,
        );
    }
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
// Sharded execution with cross shard conflict doesn't work for now because we don't have
// cross round dependency tracking yet.
fn test_partitioner_v2_uniform_sharded_block_executor_with_conflict_parallel(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    for merge_discard in [false, true] {
        let num_shards = 7;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .partition_last_round(merge_discard)
            .pre_partitioner_config(Box::new(UniformPartitionerConfig {}))
            .build();
        test_utils::sharded_block_executor_with_conflict(
            partitioner,
            sharded_block_executor,
            4,
            stateless_account,
            use_txn_payload_v2_format,
            use_orderless_transactions,
        );
    }
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_partitioner_v2_uniform_sharded_block_executor_with_conflict_sequential(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    for merge_discard in [false, true] {
        let num_shards = 7;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .partition_last_round(merge_discard)
            .pre_partitioner_config(Box::new(UniformPartitionerConfig {}))
            .build();
        test_utils::sharded_block_executor_with_conflict(
            partitioner,
            sharded_block_executor,
            1,
            stateless_account,
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )
    }
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_partitioner_v2_uniform_sharded_block_executor_with_random_transfers_parallel(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    for merge_discard in [false, true] {
        let num_shards = 3;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .partition_last_round(!merge_discard)
            .pre_partitioner_config(Box::new(UniformPartitionerConfig {}))
            .build();
        test_utils::sharded_block_executor_with_random_transfers(
            partitioner,
            sharded_block_executor,
            4,
            stateless_account,
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )
    }
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_partitioner_v2_uniform_sharded_block_executor_with_random_transfers_sequential(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    for merge_discard in [false, true] {
        let mut rng = OsRng;
        let max_num_shards = 32;
        let num_shards = rng.gen_range(1, max_num_shards);
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .partition_last_round(merge_discard)
            .pre_partitioner_config(Box::new(UniformPartitionerConfig {}))
            .build();
        test_utils::sharded_block_executor_with_random_transfers(
            partitioner,
            sharded_block_executor,
            1,
            stateless_account,
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )
    }
}

#[rstest(
    sender_stateless_account,
    receiver_stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, true, false, false),
    case(true, true, true, false),
    case(true, true, true, true),
    case(true, false, false, false),
    case(true, false, true, false),
    case(true, false, true, true),
    case(false, true, false, false),
    case(false, true, true, false),
    case(false, true, true, true),
    case(false, false, false, false),
    case(false, false, true, false),
    case(false, false, true, true)
)]
fn test_partitioner_v2_connected_component_sharded_block_executor_no_conflict(
    sender_stateless_account: bool,
    receiver_stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    for merge_discard in [false, true] {
        let num_shards = 8;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(2));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .partition_last_round(merge_discard)
            .pre_partitioner_config(Box::<ConnectedComponentPartitionerConfig>::default())
            .build();
        test_utils::test_sharded_block_executor_no_conflict(
            partitioner,
            sharded_block_executor,
            sender_stateless_account,
            receiver_stateless_account,
            use_txn_payload_v2_format,
            use_orderless_transactions,
        );
    }
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
// Sharded execution with cross shard conflict doesn't work for now because we don't have
// cross round dependency tracking yet.
fn test_partitioner_v2_connected_component_sharded_block_executor_with_conflict_parallel(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    for merge_discard in [false, true] {
        let num_shards = 7;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .partition_last_round(merge_discard)
            .pre_partitioner_config(Box::<ConnectedComponentPartitionerConfig>::default())
            .build();
        test_utils::sharded_block_executor_with_conflict(
            partitioner,
            sharded_block_executor,
            4,
            stateless_account,
            use_txn_payload_v2_format,
            use_orderless_transactions,
        );
    }
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_partitioner_v2_connected_component_sharded_block_executor_with_conflict_sequential(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    for merge_discard in [false, true] {
        let num_shards = 7;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .partition_last_round(merge_discard)
            .pre_partitioner_config(Box::<ConnectedComponentPartitionerConfig>::default())
            .build();
        test_utils::sharded_block_executor_with_conflict(
            partitioner,
            sharded_block_executor,
            1,
            stateless_account,
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )
    }
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_partitioner_v2_connected_component_sharded_block_executor_with_random_transfers_parallel(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    for merge_discard in [false, true] {
        let num_shards = 3;
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .partition_last_round(!merge_discard)
            .pre_partitioner_config(Box::<ConnectedComponentPartitionerConfig>::default())
            .build();
        test_utils::sharded_block_executor_with_random_transfers(
            partitioner,
            sharded_block_executor,
            4,
            stateless_account,
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )
    }
}

#[rstest(
    stateless_account,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true)
)]
fn test_partitioner_v2_connected_component_sharded_block_executor_with_random_transfers_sequential(
    stateless_account: bool,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    for merge_discard in [false, true] {
        let mut rng = OsRng;
        let max_num_shards = 32;
        let num_shards = rng.gen_range(1, max_num_shards);
        let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
        let sharded_block_executor = ShardedBlockExecutor::new(client);
        let partitioner = PartitionerV2Config::default()
            .partition_last_round(merge_discard)
            .pre_partitioner_config(Box::<ConnectedComponentPartitionerConfig>::default())
            .build();
        test_utils::sharded_block_executor_with_random_transfers(
            partitioner,
            sharded_block_executor,
            1,
            stateless_account,
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )
    }
}

mod test_utils {
    use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
    use aptos_block_partitioner::BlockPartitioner;
    use aptos_language_e2e_tests::{
        account::AccountData, common_transactions::peer_to_peer_txn, data_store::FakeDataStore,
        executor::FakeExecutor, feature_flags_for_orderless,
    };
    use aptos_types::{
        block_executor::{
            config::BlockExecutorConfigFromOnchain, partitioner::PartitionedTransactions,
        },
        transaction::{
            analyzed_transaction::AnalyzedTransaction,
            signature_verified_transaction::SignatureVerifiedTransaction, Transaction,
            TransactionOutput,
        },
    };
    use aptos_vm::{
        aptos_vm::AptosVMBlockExecutor,
        sharded_block_executor::{executor_client::ExecutorClient, ShardedBlockExecutor},
        VMBlockExecutor,
    };
    use move_core_types::account_address::AccountAddress;
    use rand::{rngs::OsRng, Rng};
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    pub fn generate_account_at(
        executor: &mut FakeExecutor,
        address: AccountAddress,
        seq_num: Option<u64>,
    ) -> AccountData {
        executor.new_account_data_at(address, seq_num)
    }

    fn generate_non_conflicting_sender_receiver(
        executor: &mut FakeExecutor,
        sender_stateless_account: bool,
        receiver_stateless_account: bool,
    ) -> (AccountData, AccountData) {
        let sender = executor.create_raw_account_data(
            3_000_000_000,
            if sender_stateless_account {
                None
            } else {
                Some(0)
            },
        );
        let receiver = executor.create_raw_account_data(
            3_000_000_000,
            if receiver_stateless_account {
                None
            } else {
                Some(0)
            },
        );
        executor.add_account_data(&sender);
        executor.add_account_data(&receiver);
        (sender, receiver)
    }

    pub fn generate_non_conflicting_p2p(
        executor: &mut FakeExecutor,
        sender_stateless_account: bool,
        receiver_stateless_account: bool,
        use_txn_payload_v2_format: bool,
        use_orderless_transactions: bool,
    ) -> (AnalyzedTransaction, AccountData, AccountData) {
        let (mut sender, receiver) = generate_non_conflicting_sender_receiver(
            executor,
            sender_stateless_account,
            receiver_stateless_account,
        );
        let transfer_amount = 1_000;
        let txn = generate_p2p_txn(
            &mut sender,
            &receiver,
            transfer_amount,
            use_txn_payload_v2_format,
            use_orderless_transactions,
        );
        // execute transaction
        (txn, sender, receiver)
    }

    pub fn generate_p2p_txn(
        sender: &mut AccountData,
        receiver: &AccountData,
        transfer_amount: u64,
        use_txn_payload_v2_format: bool,
        use_orderless_transactions: bool,
    ) -> AnalyzedTransaction {
        let seq_num = if use_orderless_transactions {
            Some(u64::MAX)
        } else {
            Some(sender.sequence_number().unwrap_or(0))
        };
        let txn = Transaction::UserTransaction(peer_to_peer_txn(
            sender.account(),
            receiver.account(),
            seq_num,
            transfer_amount,
            100,
            use_txn_payload_v2_format,
            use_orderless_transactions,
        ))
        .into();
        sender.increment_sequence_number();
        txn
    }

    pub fn compare_txn_outputs(
        unsharded_txn_output: Vec<TransactionOutput>,
        sharded_txn_output: Vec<TransactionOutput>,
    ) {
        assert_eq!(unsharded_txn_output.len(), sharded_txn_output.len());
        for i in 0..unsharded_txn_output.len() {
            assert_eq!(
                unsharded_txn_output[i].status(),
                sharded_txn_output[i].status()
            );
            assert_eq!(
                unsharded_txn_output[i].gas_used(),
                sharded_txn_output[i].gas_used()
            );
            assert_eq!(
                unsharded_txn_output[i].write_set(),
                sharded_txn_output[i].write_set()
            );
            assert_eq!(
                unsharded_txn_output[i].events(),
                sharded_txn_output[i].events()
            );
        }
    }

    pub fn test_sharded_block_executor_no_conflict<E: ExecutorClient<FakeDataStore>>(
        partitioner: Box<dyn BlockPartitioner>,
        sharded_block_executor: ShardedBlockExecutor<FakeDataStore, E>,
        sender_stateless_account: bool,
        receiver_stateless_account: bool,
        use_txn_payload_v2_format: bool,
        use_orderless_transactions: bool,
    ) {
        let num_txns = 400;
        let num_shards = 8;
        let mut executor = FakeExecutor::from_head_genesis();
        executor.enable_features(
            feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
            vec![],
        );
        let mut transactions = Vec::new();
        for _ in 0..num_txns {
            transactions.push(
                generate_non_conflicting_p2p(
                    &mut executor,
                    sender_stateless_account,
                    receiver_stateless_account,
                    use_txn_payload_v2_format,
                    use_orderless_transactions,
                )
                .0,
            );
        }
        let partitioned_txns = partitioner.partition(transactions.clone(), num_shards);
        let sharded_txn_output = sharded_block_executor
            .execute_block(
                Arc::new(executor.data_store().clone()),
                partitioned_txns.clone(),
                2,
                BlockExecutorConfigFromOnchain::new_no_block_limit(),
            )
            .unwrap();

        let ordered_txns: Vec<SignatureVerifiedTransaction> =
            PartitionedTransactions::flatten(partitioned_txns)
                .into_iter()
                .map(|t| t.into_txn())
                .collect();
        let txn_provider = DefaultTxnProvider::new(ordered_txns);
        let unsharded_txn_output = AptosVMBlockExecutor::new()
            .execute_block_no_limit(&txn_provider, executor.data_store())
            .unwrap();
        compare_txn_outputs(unsharded_txn_output, sharded_txn_output);
    }

    pub fn sharded_block_executor_with_conflict<E: ExecutorClient<FakeDataStore>>(
        partitioner: Box<dyn BlockPartitioner>,
        sharded_block_executor: ShardedBlockExecutor<FakeDataStore, E>,
        concurrency: usize,
        stateless_account: bool,
        use_txn_payload_v2_format: bool,
        use_orderless_transactions: bool,
    ) {
        let num_txns = 800;
        let num_shards = sharded_block_executor.num_shards();
        let num_accounts = 80;
        let mut executor = FakeExecutor::from_head_genesis();
        executor.enable_features(
            feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
            vec![],
        );
        let mut transactions = Vec::new();
        let mut accounts = Vec::new();
        let mut txn_hash_to_account = HashMap::new();
        for _ in 0..num_accounts {
            let account = generate_account_at(
                &mut executor,
                AccountAddress::random(),
                if stateless_account { None } else { Some(0) },
            );
            accounts.push(Mutex::new(account));
        }
        for i in 1..num_txns / num_accounts {
            for j in 0..num_accounts {
                let sender = &mut accounts[j].lock().unwrap();
                let sender_addr = *sender.address();
                let receiver = &accounts[(j + i) % num_accounts].lock().unwrap();
                let transfer_amount = 1_000;
                let txn = generate_p2p_txn(
                    sender,
                    receiver,
                    transfer_amount,
                    use_txn_payload_v2_format,
                    use_orderless_transactions,
                );
                txn_hash_to_account.insert(txn.transaction().hash(), sender_addr);
                transactions.push(txn)
            }
        }

        let partitioned_txns = partitioner.partition(transactions.clone(), num_shards);

        let execution_ordered_txns: Vec<SignatureVerifiedTransaction> =
            PartitionedTransactions::flatten(partitioned_txns.clone())
                .into_iter()
                .map(|t| t.into_txn())
                .collect();
        let sharded_txn_output = sharded_block_executor
            .execute_block(
                Arc::new(executor.data_store().clone()),
                partitioned_txns,
                concurrency,
                BlockExecutorConfigFromOnchain::new_no_block_limit(),
            )
            .unwrap();

        let txn_provider = DefaultTxnProvider::new(execution_ordered_txns);
        let unsharded_txn_output = AptosVMBlockExecutor::new()
            .execute_block_no_limit(&txn_provider, executor.data_store())
            .unwrap();
        compare_txn_outputs(unsharded_txn_output, sharded_txn_output);
    }

    pub fn sharded_block_executor_with_random_transfers<E: ExecutorClient<FakeDataStore>>(
        partitioner: Box<dyn BlockPartitioner>,
        sharded_block_executor: ShardedBlockExecutor<FakeDataStore, E>,
        concurrency: usize,
        stateless_account: bool,
        use_txn_payload_v2_format: bool,
        use_orderless_transactions: bool,
    ) {
        let mut rng = OsRng;
        let max_accounts = 200;
        let max_txns = 1000;
        let num_accounts = rng.gen_range(2, max_accounts);
        let mut accounts = Vec::new();
        let mut executor = FakeExecutor::from_head_genesis();
        executor.enable_features(
            feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
            vec![],
        );

        for _ in 0..num_accounts {
            let account = generate_account_at(
                &mut executor,
                AccountAddress::random(),
                if stateless_account { None } else { Some(0) },
            );
            accounts.push(Mutex::new(account));
        }

        let num_txns = rng.gen_range(1, max_txns);
        let num_shards = sharded_block_executor.num_shards();

        let mut transactions = Vec::new();

        for _ in 0..num_txns {
            let indices = rand::seq::index::sample(&mut rng, num_accounts, 2);
            let sender = &mut accounts[indices.index(0)].lock().unwrap();
            let receiver = &accounts[indices.index(1)].lock().unwrap();
            let transfer_amount = rng.gen_range(1, 1000);
            let txn = generate_p2p_txn(
                sender,
                receiver,
                transfer_amount,
                use_txn_payload_v2_format,
                use_orderless_transactions,
            );
            transactions.push(txn)
        }

        let partitioned_txns = partitioner.partition(transactions.clone(), num_shards);

        let execution_ordered_txns: Vec<SignatureVerifiedTransaction> =
            PartitionedTransactions::flatten(partitioned_txns.clone())
                .into_iter()
                .map(|t| t.into_txn())
                .collect();

        let sharded_txn_output = sharded_block_executor
            .execute_block(
                Arc::new(executor.data_store().clone()),
                partitioned_txns,
                concurrency,
                BlockExecutorConfigFromOnchain::new_no_block_limit(),
            )
            .unwrap();

        let txn_provider = DefaultTxnProvider::new(execution_ordered_txns);
        let unsharded_txn_output = AptosVMBlockExecutor::new()
            .execute_block_no_limit(&txn_provider, executor.data_store())
            .unwrap();
        compare_txn_outputs(unsharded_txn_output, sharded_txn_output);
    }
}
