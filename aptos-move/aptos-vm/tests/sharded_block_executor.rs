// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// // Copyright © Aptos Foundation
// // SPDX-License-Identifier: Apache-2.0

// #![forbid(unsafe_code)]

// /// It has to be integration tests because otherwise it forms an indirect dependency circle between
// /// aptos-vm and aptos-language-e2e-tests, which causes static variables to have two instances in
// /// the same process while testing, resulting in the counters failing to register with "AlreadyReg"
// /// error.
// use aptos_block_partitioner::{
//     pre_partition::{
//         connected_component::config::ConnectedComponentPartitionerConfig,
//         uniform_partitioner::config::UniformPartitionerConfig,
//     },
//     v2::config::PartitionerV2Config,
//     PartitionerConfig,
// };
// use aptos_vm::sharded_block_executor::{
//     local_executor_shard::LocalExecutorService, ShardedBlockExecutor,
// };
// use rand::{rngs::OsRng, Rng};
// use rstest::rstest;

// #[rstest(
//     sender_stateless_account,
//     receiver_stateless_account,
//     use_txn_payload_v2_format,
//     use_orderless_transactions,
//     case(true, true, false, false),
//     case(true, true, true, false),
//     case(true, true, true, true),
//     case(true, false, false, false),
//     case(true, false, true, false),
//     case(true, false, true, true),
//     case(false, true, false, false),
//     case(false, true, true, false),
//     case(false, true, true, true),
//     case(false, false, false, false),
//     case(false, false, true, false),
//     case(false, false, true, true)
// )]
// fn test_partitioner_v2_uniform_sharded_block_executor_no_conflict(
//     sender_stateless_account: bool,
//     receiver_stateless_account: bool,
//     use_txn_payload_v2_format: bool,
//     use_orderless_transactions: bool,
// ) {
//     for merge_discard in [false, true] {
//         let num_shards = 8;
//         let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(2));
//         let sharded_block_executor = ShardedBlockExecutor::new(client);
//         let partitioner = PartitionerV2Config::default()
//             .partition_last_round(merge_discard)
//             .pre_partitioner_config(Box::new(UniformPartitionerConfig {}))
//             .build();
//         test_utils::test_sharded_block_executor_no_conflict(
//             partitioner,
//             sharded_block_executor,
//             sender_stateless_account,
//             receiver_stateless_account,
//             use_txn_payload_v2_format,
//             use_orderless_transactions,
//         );
//     }
// }

// #[rstest(
//     stateless_account,
//     use_txn_payload_v2_format,
//     use_orderless_transactions,
//     case(true, false, false),
//     case(true, true, false),
//     case(true, true, true),
//     case(false, false, false),
//     case(false, true, false),
//     case(false, true, true)
// )]
// // Sharded execution with cross shard conflict doesn't work for now because we don't have
// // cross round dependency tracking yet.
// fn test_partitioner_v2_uniform_sharded_block_executor_with_conflict_parallel(
//     stateless_account: bool,
//     use_txn_payload_v2_format: bool,
//     use_orderless_transactions: bool,
// ) {
//     for merge_discard in [false, true] {
//         let num_shards = 7;
//         let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
//         let sharded_block_executor = ShardedBlockExecutor::new(client);
//         let partitioner = PartitionerV2Config::default()
//             .partition_last_round(merge_discard)
//             .pre_partitioner_config(Box::new(UniformPartitionerConfig {}))
//             .build();
//         test_utils::sharded_block_executor_with_conflict(
//             partitioner,
//             sharded_block_executor,
//             4,
//             stateless_account,
//             use_txn_payload_v2_format,
//             use_orderless_transactions,
//         );
//     }
// }

// #[rstest(
//     stateless_account,
//     use_txn_payload_v2_format,
//     use_orderless_transactions,
//     case(true, false, false),
//     case(true, true, false),
//     case(true, true, true),
//     case(false, false, false),
//     case(false, true, false),
//     case(false, true, true)
// )]
// fn test_partitioner_v2_uniform_sharded_block_executor_with_conflict_sequential(
//     stateless_account: bool,
//     use_txn_payload_v2_format: bool,
//     use_orderless_transactions: bool,
// ) {
//     for merge_discard in [false, true] {
//         let num_shards = 7;
//         let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
//         let sharded_block_executor = ShardedBlockExecutor::new(client);
//         let partitioner = PartitionerV2Config::default()
//             .partition_last_round(merge_discard)
//             .pre_partitioner_config(Box::new(UniformPartitionerConfig {}))
//             .build();
//         test_utils::sharded_block_executor_with_conflict(
//             partitioner,
//             sharded_block_executor,
//             1,
//             stateless_account,
//             use_txn_payload_v2_format,
//             use_orderless_transactions,
//         )
//     }
// }

// #[rstest(
//     stateless_account,
//     use_txn_payload_v2_format,
//     use_orderless_transactions,
//     case(true, false, false),
//     case(true, true, false),
//     case(true, true, true),
//     case(false, false, false),
//     case(false, true, false),
//     case(false, true, true)
// )]
// fn test_partitioner_v2_uniform_sharded_block_executor_with_random_transfers_parallel(
//     stateless_account: bool,
//     use_txn_payload_v2_format: bool,
//     use_orderless_transactions: bool,
// ) {
//     for merge_discard in [false, true] {
//         let num_shards = 3;
//         let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
//         let sharded_block_executor = ShardedBlockExecutor::new(client);
//         let partitioner = PartitionerV2Config::default()
//             .partition_last_round(!merge_discard)
//             .pre_partitioner_config(Box::new(UniformPartitionerConfig {}))
//             .build();
//         test_utils::sharded_block_executor_with_random_transfers(
//             partitioner,
//             sharded_block_executor,
//             4,
//             stateless_account,
//             use_txn_payload_v2_format,
//             use_orderless_transactions,
//         )
//     }
// }

// #[rstest(
//     stateless_account,
//     use_txn_payload_v2_format,
//     use_orderless_transactions,
//     case(true, false, false),
//     case(true, true, false),
//     case(true, true, true),
//     case(false, false, false),
//     case(false, true, false),
//     case(false, true, true)
// )]
// fn test_partitioner_v2_uniform_sharded_block_executor_with_random_transfers_sequential(
//     stateless_account: bool,
//     use_txn_payload_v2_format: bool,
//     use_orderless_transactions: bool,
// ) {
//     for merge_discard in [false, true] {
//         let mut rng = OsRng;
//         let max_num_shards = 32;
//         let num_shards = rng.gen_range(1, max_num_shards);
//         let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
//         let sharded_block_executor = ShardedBlockExecutor::new(client);
//         let partitioner = PartitionerV2Config::default()
//             .partition_last_round(merge_discard)
//             .pre_partitioner_config(Box::new(UniformPartitionerConfig {}))
//             .build();
//         test_utils::sharded_block_executor_with_random_transfers(
//             partitioner,
//             sharded_block_executor,
//             1,
//             stateless_account,
//             use_txn_payload_v2_format,
//             use_orderless_transactions,
//         )
//     }
// }

// #[rstest(
//     sender_stateless_account,
//     receiver_stateless_account,
//     use_txn_payload_v2_format,
//     use_orderless_transactions,
//     case(true, true, false, false),
//     case(true, true, true, false),
//     case(true, true, true, true),
//     case(true, false, false, false),
//     case(true, false, true, false),
//     case(true, false, true, true),
//     case(false, true, false, false),
//     case(false, true, true, false),
//     case(false, true, true, true),
//     case(false, false, false, false),
//     case(false, false, true, false),
//     case(false, false, true, true)
// )]
// fn test_partitioner_v2_connected_component_sharded_block_executor_no_conflict(
//     sender_stateless_account: bool,
//     receiver_stateless_account: bool,
//     use_txn_payload_v2_format: bool,
//     use_orderless_transactions: bool,
// ) {
//     for merge_discard in [false, true] {
//         let num_shards = 8;
//         let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(2));
//         let sharded_block_executor = ShardedBlockExecutor::new(client);
//         let partitioner = PartitionerV2Config::default()
//             .partition_last_round(merge_discard)
//             .pre_partitioner_config(Box::<ConnectedComponentPartitionerConfig>::default())
//             .build();
//         test_utils::test_sharded_block_executor_no_conflict(
//             partitioner,
//             sharded_block_executor,
//             sender_stateless_account,
//             receiver_stateless_account,
//             use_txn_payload_v2_format,
//             use_orderless_transactions,
//         );
//     }
// }

// #[rstest(
//     stateless_account,
//     use_txn_payload_v2_format,
//     use_orderless_transactions,
//     case(true, false, false),
//     case(true, true, false),
//     case(true, true, true),
//     case(false, false, false),
//     case(false, true, false),
//     case(false, true, true)
// )]
// // Sharded execution with cross shard conflict doesn't work for now because we don't have
// // cross round dependency tracking yet.
// fn test_partitioner_v2_connected_component_sharded_block_executor_with_conflict_parallel(
//     stateless_account: bool,
//     use_txn_payload_v2_format: bool,
//     use_orderless_transactions: bool,
// ) {
//     for merge_discard in [false, true] {
//         let num_shards = 7;
//         let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
//         let sharded_block_executor = ShardedBlockExecutor::new(client);
//         let partitioner = PartitionerV2Config::default()
//             .partition_last_round(merge_discard)
//             .pre_partitioner_config(Box::<ConnectedComponentPartitionerConfig>::default())
//             .build();
//         test_utils::sharded_block_executor_with_conflict(
//             partitioner,
//             sharded_block_executor,
//             4,
//             stateless_account,
//             use_txn_payload_v2_format,
//             use_orderless_transactions,
//         );
//     }
// }

// #[rstest(
//     stateless_account,
//     use_txn_payload_v2_format,
//     use_orderless_transactions,
//     case(true, false, false),
//     case(true, true, false),
//     case(true, true, true),
//     case(false, false, false),
//     case(false, true, false),
//     case(false, true, true)
// )]
// fn test_partitioner_v2_connected_component_sharded_block_executor_with_conflict_sequential(
//     stateless_account: bool,
//     use_txn_payload_v2_format: bool,
//     use_orderless_transactions: bool,
// ) {
//     for merge_discard in [false, true] {
//         let num_shards = 7;
//         let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
//         let sharded_block_executor = ShardedBlockExecutor::new(client);
//         let partitioner = PartitionerV2Config::default()
//             .partition_last_round(merge_discard)
//             .pre_partitioner_config(Box::<ConnectedComponentPartitionerConfig>::default())
//             .build();
//         test_utils::sharded_block_executor_with_conflict(
//             partitioner,
//             sharded_block_executor,
//             1,
//             stateless_account,
//             use_txn_payload_v2_format,
//             use_orderless_transactions,
//         )
//     }
// }

// #[rstest(
//     stateless_account,
//     use_txn_payload_v2_format,
//     use_orderless_transactions,
//     case(true, false, false),
//     case(true, true, false),
//     case(true, true, true),
//     case(false, false, false),
//     case(false, true, false),
//     case(false, true, true)
// )]
// fn test_partitioner_v2_connected_component_sharded_block_executor_with_random_transfers_parallel(
//     stateless_account: bool,
//     use_txn_payload_v2_format: bool,
//     use_orderless_transactions: bool,
// ) {
//     for merge_discard in [false, true] {
//         let num_shards = 3;
//         let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(4));
//         let sharded_block_executor = ShardedBlockExecutor::new(client);
//         let partitioner = PartitionerV2Config::default()
//             .partition_last_round(!merge_discard)
//             .pre_partitioner_config(Box::<ConnectedComponentPartitionerConfig>::default())
//             .build();
//         test_utils::sharded_block_executor_with_random_transfers(
//             partitioner,
//             sharded_block_executor,
//             4,
//             stateless_account,
//             use_txn_payload_v2_format,
//             use_orderless_transactions,
//         )
//     }
// }

// #[rstest(
//     stateless_account,
//     use_txn_payload_v2_format,
//     use_orderless_transactions,
//     case(true, false, false),
//     case(true, true, false),
//     case(true, true, true),
//     case(false, false, false),
//     case(false, true, false),
//     case(false, true, true)
// )]
// fn test_partitioner_v2_connected_component_sharded_block_executor_with_random_transfers_sequential(
//     stateless_account: bool,
//     use_txn_payload_v2_format: bool,
//     use_orderless_transactions: bool,
// ) {
//     for merge_discard in [false, true] {
//         let mut rng = OsRng;
//         let max_num_shards = 32;
//         let num_shards = rng.gen_range(1, max_num_shards);
//         let client = LocalExecutorService::setup_local_executor_shards(num_shards, Some(1));
//         let sharded_block_executor = ShardedBlockExecutor::new(client);
//         let partitioner = PartitionerV2Config::default()
//             .partition_last_round(merge_discard)
//             .pre_partitioner_config(Box::<ConnectedComponentPartitionerConfig>::default())
//             .build();
//         test_utils::sharded_block_executor_with_random_transfers(
//             partitioner,
//             sharded_block_executor,
//             1,
//             stateless_account,
//             use_txn_payload_v2_format,
//             use_orderless_transactions,
//         )
//     }
// }

// mod test_utils {
//     // Question: This code seems to be duplicated from execution/executor-service/src/test_utils.rs. Is the duplication required?
//     use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
//     use aptos_block_partitioner::BlockPartitioner;
//     use aptos_keygen::KeyGen;
//     use aptos_language_e2e_tests::{common_transactions::peer_to_peer_txn, executor::FakeExecutor, feature_flags_for_orderless};
//     use aptos_transaction_simulation::{
//         Account, AccountData, InMemoryStateStore, SimulationStateStore,
//     };
//     use aptos_types::{
//         block_executor::{
//             config::BlockExecutorConfigFromOnchain, partitioner::PartitionedTransactions,
//         },
//         transaction::{
//             analyzed_transaction::AnalyzedTransaction,
//             signature_verified_transaction::SignatureVerifiedTransaction, Transaction,
//             TransactionOutput,
//         },
//     };
//     use aptos_vm::{
//         aptos_vm::AptosVMBlockExecutor,
//         sharded_block_executor::{executor_client::ExecutorClient, ShardedBlockExecutor},
//         VMBlockExecutor,
//     };
//     use rand::{rngs::OsRng, Rng};
//     use std::{
//         collections::HashMap,
//         sync::{Arc, Mutex},
//     };

//     pub fn generate_account_with_balance(
//         rng: &mut KeyGen,
//         state_store: &impl SimulationStateStore,
//         stateless_account: bool,
//     ) -> AccountData {
//         let acc = Account::new_from_seed(rng);

//         state_store
//             .store_and_fund_account(acc, 1_000_000_000_000_000, if stateless_account { None } else { Some(0) })
//             .unwrap()
//     }

//     fn generate_non_conflicting_sender_receiver(
//         rng: &mut KeyGen,
//         state_store: &impl SimulationStateStore,
//         sender_stateless_account: bool,
//         receiver_stateless_account: bool,
//     ) -> (AccountData, AccountData) {
//         let sender = AccountData::new_from_seed(rng,
//             3_000_000_000,
//             if sender_stateless_account {
//                 None
//             } else {
//                 Some(0)
//             },
//         );
//         let receiver = AccountData::new_from_seed(rng,
//             3_000_000_000,
//             if receiver_stateless_account {
//                 None
//             } else {
//                 Some(0)
//             },
//         );
//         state_store.add_account_data(&sender).unwrap();
//         state_store.add_account_data(&receiver).unwrap();
//         (sender, receiver)
//     }

//     pub fn generate_non_conflicting_p2p(
//         rng: &mut KeyGen,
//         state_store: &impl SimulationStateStore,
//         sender_stateless_account: bool,
//         receiver_stateless_account: bool,
//         use_txn_payload_v2_format: bool,
//         use_orderless_transactions: bool,
//     ) -> (AnalyzedTransaction, AccountData, AccountData) {
//         let (mut sender, receiver) = generate_non_conflicting_sender_receiver(rng, state_store,
//             sender_stateless_account,
//             receiver_stateless_account,
//         );
//         let transfer_amount = 1_000;
//         let txn = generate_p2p_txn(
//             &mut sender,
//             &receiver,
//             transfer_amount,
//             use_txn_payload_v2_format,
//             use_orderless_transactions,
//         );
//         // execute transaction
//         (txn, sender, receiver)
//     }

//     pub fn generate_p2p_txn(
//         sender: &mut AccountData,
//         receiver: &AccountData,
//         transfer_amount: u64,
//         use_txn_payload_v2_format: bool,
//         use_orderless_transactions: bool,
//     ) -> AnalyzedTransaction {
//         let seq_num = if use_orderless_transactions {
//             Some(u64::MAX)
//         } else {
//             Some(sender.sequence_number().unwrap_or(0))
//         };
//         let txn = Transaction::UserTransaction(peer_to_peer_txn(
//             sender.account(),
//             receiver.account(),
//             seq_num,
//             transfer_amount,
//             100,
//             use_txn_payload_v2_format,
//             use_orderless_transactions,
//         ))
//         .into();
//         sender.increment_sequence_number();
//         txn
//     }

//     pub fn compare_txn_outputs(
//         unsharded_txn_output: Vec<TransactionOutput>,
//         sharded_txn_output: Vec<TransactionOutput>,
//     ) {
//         assert_eq!(unsharded_txn_output.len(), sharded_txn_output.len());
//         for i in 0..unsharded_txn_output.len() {
//             assert_eq!(
//                 unsharded_txn_output[i].status(),
//                 sharded_txn_output[i].status()
//             );
//             assert_eq!(
//                 unsharded_txn_output[i].gas_used(),
//                 sharded_txn_output[i].gas_used()
//             );
//             assert_eq!(
//                 unsharded_txn_output[i].write_set(),
//                 sharded_txn_output[i].write_set()
//             );
//             assert_eq!(
//                 unsharded_txn_output[i].events(),
//                 sharded_txn_output[i].events()
//             );
//         }
//     }

//     pub fn test_sharded_block_executor_no_conflict<E: ExecutorClient<InMemoryStateStore>>(
//         partitioner: Box<dyn BlockPartitioner>,
//         sharded_block_executor: ShardedBlockExecutor<InMemoryStateStore, E>,
//         sender_stateless_account: bool,
//         receiver_stateless_account: bool,
//         use_txn_payload_v2_format: bool,
//         use_orderless_transactions: bool,
//     ) {
//         let num_txns = 400;
//         let num_shards = 8;
//         let mut executor = FakeExecutor::from_head_genesis();
//         executor.enable_features(
//             feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
//             vec![],
//         );
//         let state_store = executor.state_store();
//         // let state_store = InMemoryStateStore::from_head_genesis();
//         let mut rng = KeyGen::from_seed([9; 32]);
//         let mut transactions = Vec::new();
//         for _ in 0..num_txns {
//             transactions.push(
//                 generate_non_conflicting_p2p(
//                     &mut rng, state_store,
//                     sender_stateless_account,
//                     receiver_stateless_account,
//                     use_txn_payload_v2_format,
//                     use_orderless_transactions,
//                 )
//                 .0,
//             );
//         }
//         let partitioned_txns = partitioner.partition(transactions.clone(), num_shards);
//         let sharded_txn_output = sharded_block_executor
//             .execute_block(
//                 Arc::new(state_store.clone()),
//                 partitioned_txns.clone(),
//                 2,
//                 BlockExecutorConfigFromOnchain::new_no_block_limit(),
//             )
//             .unwrap();

//         let ordered_txns: Vec<SignatureVerifiedTransaction> =
//             PartitionedTransactions::flatten(partitioned_txns)
//                 .into_iter()
//                 .map(|t| t.into_txn())
//                 .collect();
//         let txn_provider = DefaultTxnProvider::new(ordered_txns);
//         let unsharded_txn_output = AptosVMBlockExecutor::new()
//             .execute_block_no_limit(&txn_provider, &state_store)
//             .unwrap();
//         compare_txn_outputs(unsharded_txn_output, sharded_txn_output);
//     }

//     pub fn sharded_block_executor_with_conflict<E: ExecutorClient<InMemoryStateStore>>(
//         partitioner: Box<dyn BlockPartitioner>,
//         sharded_block_executor: ShardedBlockExecutor<InMemoryStateStore, E>,
//         concurrency: usize,
//         stateless_account: bool,
//         use_txn_payload_v2_format: bool,
//         use_orderless_transactions: bool,
//     ) {
//         let mut rng = KeyGen::from_seed([9; 32]);
//         let num_txns = 800;
//         let num_shards = sharded_block_executor.num_shards();
//         let num_accounts = 80;
//         let mut executor = FakeExecutor::from_head_genesis();
//         executor.enable_features(
//             feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
//             vec![],
//         );
//         let state_store = executor.state_store();
//         // let state_store = InMemoryStateStore::from_head_genesis();
//         let mut transactions = Vec::new();
//         let mut accounts = Vec::new();
//         let mut txn_hash_to_account = HashMap::new();
//         for _ in 0..num_accounts {
//             let account_data = generate_account_with_balance(
//                 &mut rng,
//                 state_store,
//                 stateless_account,
//             );
//             accounts.push(Mutex::new(account_data));
//         }
//         for i in 1..num_txns / num_accounts {
//             for j in 0..num_accounts {
//                 let sender = &mut accounts[j].lock().unwrap();
//                 let sender_addr = *sender.address();
//                 let receiver = &accounts[(j + i) % num_accounts].lock().unwrap();
//                 let transfer_amount = 1_000;
//                 let txn = generate_p2p_txn(
//                     sender,
//                     receiver,
//                     transfer_amount,
//                     use_txn_payload_v2_format,
//                     use_orderless_transactions,
//                 );
//                 txn_hash_to_account.insert(txn.transaction().hash(), sender_addr);
//                 transactions.push(txn)
//             }
//         }

//         let partitioned_txns = partitioner.partition(transactions.clone(), num_shards);

//         let execution_ordered_txns: Vec<SignatureVerifiedTransaction> =
//             PartitionedTransactions::flatten(partitioned_txns.clone())
//                 .into_iter()
//                 .map(|t| t.into_txn())
//                 .collect();
//         let sharded_txn_output = sharded_block_executor
//             .execute_block(
//                 Arc::new(state_store.clone()),
//                 partitioned_txns,
//                 concurrency,
//                 BlockExecutorConfigFromOnchain::new_no_block_limit(),
//             )
//             .unwrap();

//         let txn_provider = DefaultTxnProvider::new(execution_ordered_txns);
//         let unsharded_txn_output = AptosVMBlockExecutor::new()
//             .execute_block_no_limit(&txn_provider, &state_store)
//             .unwrap();
//         compare_txn_outputs(unsharded_txn_output, sharded_txn_output);
//     }

//     pub fn sharded_block_executor_with_random_transfers<E: ExecutorClient<InMemoryStateStore>>(
//         partitioner: Box<dyn BlockPartitioner>,
//         sharded_block_executor: ShardedBlockExecutor<InMemoryStateStore, E>,
//         concurrency: usize,
//         stateless_account: bool,
//         use_txn_payload_v2_format: bool,
//         use_orderless_transactions: bool,
//     ) {
//         let mut rng = OsRng;
//         let max_accounts = 200;
//         let max_txns = 1000;
//         let num_accounts = rng.gen_range(2, max_accounts);
//         let mut accounts = Vec::new();
//         let mut executor = FakeExecutor::from_head_genesis();
//         executor.enable_features(
//             feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
//             vec![],
//         );
//         let state_store = executor.state_store();
//         // let state_store = InMemoryStateStore::from_head_genesis();
//         let mut key_gen = KeyGen::from_seed([9; 32]);
//         for _ in 0..num_accounts {
//             let account = generate_account_with_balance(
//                 &mut key_gen,
//                 state_store,
//                 stateless_account,
//             );
//             accounts.push(Mutex::new(account));
//         }

//         let num_txns = rng.gen_range(1, max_txns);
//         let num_shards = sharded_block_executor.num_shards();

//         let mut transactions = Vec::new();

//         for _ in 0..num_txns {
//             let indices = rand::seq::index::sample(&mut rng, num_accounts, 2);
//             let sender = &mut accounts[indices.index(0)].lock().unwrap();
//             let receiver = &accounts[indices.index(1)].lock().unwrap();
//             let transfer_amount = rng.gen_range(1, 1000);
//             let txn = generate_p2p_txn(
//                 sender,
//                 receiver,
//                 transfer_amount,
//                 use_txn_payload_v2_format,
//                 use_orderless_transactions,
//             );
//             transactions.push(txn)
//         }

//         let partitioned_txns = partitioner.partition(transactions.clone(), num_shards);

//         let execution_ordered_txns: Vec<SignatureVerifiedTransaction> =
//             PartitionedTransactions::flatten(partitioned_txns.clone())
//                 .into_iter()
//                 .map(|t| t.into_txn())
//                 .collect();

//         let sharded_txn_output = sharded_block_executor
//             .execute_block(
//                 Arc::new(state_store.clone()),
//                 partitioned_txns,
//                 concurrency,
//                 BlockExecutorConfigFromOnchain::new_no_block_limit(),
//             )
//             .unwrap();

//         let txn_provider = DefaultTxnProvider::new(execution_ordered_txns);
//         let unsharded_txn_output = AptosVMBlockExecutor::new()
//             .execute_block_no_limit(&txn_provider, &state_store)
//             .unwrap();
//         compare_txn_outputs(unsharded_txn_output, sharded_txn_output);
//     }
// }
