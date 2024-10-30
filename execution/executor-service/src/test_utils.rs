// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// // Copyright © Aptos Foundation
// // SPDX-License-Identifier: Apache-2.0

// use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
// use aptos_block_partitioner::{v2::config::PartitionerV2Config, PartitionerConfig};
// use aptos_keygen::KeyGen;
// use aptos_language_e2e_tests::{common_transactions::peer_to_peer_txn, executor::FakeExecutor, feature_flags_for_orderless};
// use aptos_transaction_simulation::{
//     Account, AccountData, InMemoryStateStore, SimulationStateStore,
// };
// use aptos_types::{
//     block_executor::{
//         config::BlockExecutorConfigFromOnchain, partitioner::PartitionedTransactions,
//     },
//     state_store::state_key::inner::StateKeyInner,
//     transaction::{
//         analyzed_transaction::AnalyzedTransaction,
//         signature_verified_transaction::SignatureVerifiedTransaction, Transaction,
//         TransactionOutput,
//     },
// };
// use aptos_vm::{
//     aptos_vm::AptosVMBlockExecutor,
//     sharded_block_executor::{executor_client::ExecutorClient, ShardedBlockExecutor},
//     VMBlockExecutor,
// };
// use std::{
//     collections::HashMap,
//     sync::{Arc, Mutex},
// };

// pub fn generate_account_with_balance(
//     rng: &mut KeyGen,
//     state_store: &impl SimulationStateStore,
//     stateless_account: bool,
// ) -> AccountData {
//     let acc = Account::new_from_seed(rng);

//     state_store
//         .store_and_fund_account(acc, 1_000_000_000_000_000, if stateless_account { None } else { Some(0) })
//         .unwrap()
// }

// fn generate_non_conflicting_sender_receiver(
//     rng: &mut KeyGen,
//     state_store: &impl SimulationStateStore,
//     stateless_account: bool,
// ) -> (AccountData, AccountData) {
//     // TODO[Orderless]: Also add a case where sender is stateless and receiver is not, etc.
//     let sender = AccountData::new_from_seed(rng,
//         3_000_000_000,
//         if stateless_account { None } else { Some(0) },
//     );
//     let receiver = AccountData::new_from_seed(rng,
//         3_000_000_000,
//         if stateless_account { None } else { Some(0) },
//     );
//     state_store.add_account_data(&sender).unwrap();
//     state_store.add_account_data(&receiver).unwrap();
//     (sender, receiver)
// }

// pub fn generate_non_conflicting_p2p(
//     rng: &mut KeyGen,
//     state_store: &impl SimulationStateStore,
//     stateless_account: bool,
//     use_txn_payload_v2_format: bool,
//     use_orderless_transactions: bool,
// ) -> (AnalyzedTransaction, AccountData, AccountData) {
//     let (mut sender, receiver) =
//         generate_non_conflicting_sender_receiver(rng, state_store, stateless_account);
//     let transfer_amount = 1_000;
//     let txn = generate_p2p_txn(
//         &mut sender,
//         &receiver,
//         transfer_amount,
//         use_txn_payload_v2_format,
//         use_orderless_transactions,
//     );
//     // execute transaction
//     (txn, sender, receiver)
// }

// pub fn generate_p2p_txn(
//     sender: &mut AccountData,
//     receiver: &AccountData,
//     transfer_amount: u64,
//     use_txn_payload_v2_format: bool,
//     use_orderless_transactions: bool,
// ) -> AnalyzedTransaction {
//     let txn = Transaction::UserTransaction(peer_to_peer_txn(
//         sender.account(),
//         receiver.account(),
//         Some(sender.sequence_number().unwrap_or(0)),
//         transfer_amount,
//         100,
//         use_txn_payload_v2_format,
//         use_orderless_transactions,
//     ))
//     .into();
//     sender.increment_sequence_number();
//     txn
// }

// pub fn compare_txn_outputs(
//     unsharded_txn_output: Vec<TransactionOutput>,
//     sharded_txn_output: Vec<TransactionOutput>,
// ) {
//     assert_eq!(unsharded_txn_output.len(), sharded_txn_output.len());
//     for i in 0..unsharded_txn_output.len() {
//         assert_eq!(
//             unsharded_txn_output[i].status(),
//             sharded_txn_output[i].status()
//         );
//         assert_eq!(
//             unsharded_txn_output[i].gas_used(),
//             sharded_txn_output[i].gas_used()
//         );
//         //assert_eq!(unsharded_txn_output[i].write_set(), sharded_txn_output[i].write_set());
//         assert_eq!(
//             unsharded_txn_output[i].events(),
//             sharded_txn_output[i].events()
//         );
//         // Global supply tracking for coin is not supported in sharded execution yet, so we filter
//         // out the table item from the write set, which has the global supply. This is a hack until
//         // we support global supply tracking in sharded execution.
//         let unsharded_write_set_without_table_item = unsharded_txn_output[i]
//             .write_set()
//             .into_iter()
//             .filter(|(k, _)| matches!(k.inner(), &StateKeyInner::AccessPath(_)))
//             .collect::<Vec<_>>();
//         let sharded_write_set_without_table_item = sharded_txn_output[i]
//             .write_set()
//             .into_iter()
//             .filter(|(k, _)| matches!(k.inner(), &StateKeyInner::AccessPath(_)))
//             .collect::<Vec<_>>();
//         assert_eq!(
//             unsharded_write_set_without_table_item,
//             sharded_write_set_without_table_item
//         );
//     }
// }

// pub fn test_sharded_block_executor_no_conflict<E: ExecutorClient<InMemoryStateStore>>(
//     mut sharded_block_executor: ShardedBlockExecutor<InMemoryStateStore, E>,
//     stateless_account: bool,
//     use_txn_payload_v2_format: bool,
//     use_orderless_transactions: bool,
// ) {
//     let num_txns = 400;
//     let num_shards = sharded_block_executor.num_shards();
//     let mut executor = FakeExecutor::from_head_genesis();
//     executor.enable_features(
//         feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
//         vec![],
//     );
//     let state_store = executor.state_store();
//     // let state_store = InMemoryStateStore::from_head_genesis();
//     let mut transactions = Vec::new();
//     let mut rng = KeyGen::from_seed([9; 32]);
//     for _ in 0..num_txns {
//         transactions.push(
//             generate_non_conflicting_p2p(
//                 &mut rng, state_store,
//                 stateless_account,
//                 use_txn_payload_v2_format,
//                 use_orderless_transactions,
//             )
//             .0,
//         )
//     }
//     let partitioner = PartitionerV2Config::default()
//         .max_partitioning_rounds(2)
//         .cross_shard_dep_avoid_threshold(0.9)
//         .partition_last_round(true)
//         .build();
//     let partitioned_txns = partitioner.partition(transactions.clone(), num_shards);
//     let sharded_txn_output = sharded_block_executor
//         .execute_block(
//             Arc::new(state_store.clone()),
//             partitioned_txns.clone(),
//             2,
//             BlockExecutorConfigFromOnchain::new_no_block_limit(),
//         )
//         .unwrap();
//     let txns: Vec<SignatureVerifiedTransaction> =
//         PartitionedTransactions::flatten(partitioned_txns)
//             .into_iter()
//             .map(|t| t.into_txn())
//             .collect();
//     let txn_provider = DefaultTxnProvider::new(txns);
//     let unsharded_txn_output = AptosVMBlockExecutor::new()
//         .execute_block_no_limit(&txn_provider, &state_store)
//         .unwrap();
//     compare_txn_outputs(unsharded_txn_output, sharded_txn_output);
//     sharded_block_executor.shutdown();
// }

// pub fn sharded_block_executor_with_conflict<E: ExecutorClient<InMemoryStateStore>>(
//     mut sharded_block_executor: ShardedBlockExecutor<InMemoryStateStore, E>,
//     concurrency: usize,
//     stateless_account: bool,
//     use_txn_payload_v2_format: bool,
//     use_orderless_transactions: bool,
// ) {
//     let num_txns = 800;
//     let num_shards = sharded_block_executor.num_shards();
//     let num_accounts = 80;
//     let mut executor = FakeExecutor::from_head_genesis();
//     executor.enable_features(
//         feature_flags_for_orderless(use_txn_payload_v2_format, use_orderless_transactions),
//         vec![],
//     );
//     let state_store = executor.state_store();
//     // let state_store = InMemoryStateStore::from_head_genesis();
//     let mut transactions = Vec::new();
//     let mut accounts = Vec::new();
//     let mut txn_hash_to_account = HashMap::new();
//     let mut key_gen = KeyGen::from_seed([9; 32]);
//     for _ in 0..num_accounts {
//         let account =
//             generate_account_with_balance(&mut key_gen, state_store, stateless_account);
//         accounts.push(Mutex::new(account));
//     }
//     for i in 1..num_txns / num_accounts {
//         for j in 0..num_accounts {
//             let sender = &mut accounts[j].lock().unwrap();
//             let sender_addr = *sender.address();
//             let receiver = &accounts[(j + i) % num_accounts].lock().unwrap();
//             let transfer_amount = 1_000;
//             let txn = generate_p2p_txn(
//                 sender,
//                 receiver,
//                 transfer_amount,
//                 use_txn_payload_v2_format,
//                 use_orderless_transactions,
//             );
//             txn_hash_to_account.insert(txn.transaction().hash(), sender_addr);
//             transactions.push(txn)
//         }
//     }

//     let partitioner = PartitionerV2Config::default()
//         .max_partitioning_rounds(2)
//         .cross_shard_dep_avoid_threshold(0.9)
//         .partition_last_round(true)
//         .build();
//     let partitioned_txns = partitioner.partition(transactions.clone(), num_shards);

//     let execution_ordered_txns: Vec<SignatureVerifiedTransaction> =
//         PartitionedTransactions::flatten(partitioned_txns.clone())
//             .into_iter()
//             .map(|t| t.into_txn())
//             .collect();
//     let sharded_txn_output = sharded_block_executor
//         .execute_block(
//             Arc::new(state_store.clone()),
//             partitioned_txns,
//             concurrency,
//             BlockExecutorConfigFromOnchain::new_no_block_limit(),
//         )
//         .unwrap();

//     let txn_provider = DefaultTxnProvider::new(execution_ordered_txns);
//     let unsharded_txn_output = AptosVMBlockExecutor::new()
//         .execute_block_no_limit(&txn_provider, &state_store)
//         .unwrap();
//     compare_txn_outputs(unsharded_txn_output, sharded_txn_output);
//     sharded_block_executor.shutdown();
// }
