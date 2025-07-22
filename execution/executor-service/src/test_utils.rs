// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
use aptos_block_partitioner::{v2::config::PartitionerV2Config, PartitionerConfig};
use aptos_keygen::KeyGen;
use aptos_language_e2e_tests::common_transactions::peer_to_peer_txn;
use aptos_transaction_simulation::{
    Account, AccountData, InMemoryStateStore, SimulationStateStore,
};
use aptos_types::{
    account_address::AccountAddress,
    block_executor::{
        config::BlockExecutorConfigFromOnchain, partitioner::PartitionedTransactions,
    },
    state_store::state_key::inner::StateKeyInner,
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
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::SystemTime,
};

pub fn generate_account_at(
    state_store: &impl SimulationStateStore,
    address: AccountAddress,
) -> AccountData {
    let acc = Account::new_genesis_account(address);
    state_store
        .store_and_fund_account(acc, 1_000_000_000_000_000, Some(0))
        .unwrap()
}

fn generate_non_conflicting_sender_receiver(
    rng: &mut KeyGen,
    state_store: &impl SimulationStateStore,
) -> (AccountData, AccountData) {
    let sender = AccountData::new_from_seed(rng, 3_000_000_000, Some(0));
    let receiver = AccountData::new_from_seed(rng, 3_000_000_000, Some(0));
    state_store.add_account_data(&sender).unwrap();
    state_store.add_account_data(&receiver).unwrap();
    (sender, receiver)
}

pub fn generate_non_conflicting_p2p(
    rng: &mut KeyGen,
    state_store: &impl SimulationStateStore,
) -> (AnalyzedTransaction, AccountData, AccountData) {
    let (mut sender, receiver) = generate_non_conflicting_sender_receiver(rng, state_store);
    let transfer_amount = 1_000;
    let txn = generate_p2p_txn(&mut sender, &receiver, transfer_amount);
    // execute transaction
    (txn, sender, receiver)
}

pub fn generate_p2p_txn(
    sender: &mut AccountData,
    receiver: &AccountData,
    transfer_amount: u64,
) -> AnalyzedTransaction {
    let txn = Transaction::UserTransaction(peer_to_peer_txn(
        sender.account(),
        receiver.account(),
        sender.sequence_number(),
        transfer_amount,
        100,
        SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        false,
        false,
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
        //assert_eq!(unsharded_txn_output[i].write_set(), sharded_txn_output[i].write_set());
        assert_eq!(
            unsharded_txn_output[i].events(),
            sharded_txn_output[i].events()
        );
        // Global supply tracking for coin is not supported in sharded execution yet, so we filter
        // out the table item from the write set, which has the global supply. This is a hack until
        // we support global supply tracking in sharded execution.
        let unsharded_write_set_without_table_item = unsharded_txn_output[i]
            .write_set()
            .write_op_iter()
            .filter(|(k, _)| matches!(k.inner(), &StateKeyInner::AccessPath(_)))
            .collect::<Vec<_>>();
        let sharded_write_set_without_table_item = sharded_txn_output[i]
            .write_set()
            .write_op_iter()
            .filter(|(k, _)| matches!(k.inner(), &StateKeyInner::AccessPath(_)))
            .collect::<Vec<_>>();
        assert_eq!(
            unsharded_write_set_without_table_item,
            sharded_write_set_without_table_item
        );
    }
}

pub fn test_sharded_block_executor_no_conflict<E: ExecutorClient<InMemoryStateStore>>(
    mut sharded_block_executor: ShardedBlockExecutor<InMemoryStateStore, E>,
) {
    let num_txns = 400;
    let num_shards = sharded_block_executor.num_shards();
    let state_store = InMemoryStateStore::from_head_genesis();
    let mut transactions = Vec::new();
    let mut rng = KeyGen::from_seed([9; 32]);
    for _ in 0..num_txns {
        transactions.push(generate_non_conflicting_p2p(&mut rng, &state_store).0)
    }
    let partitioner = PartitionerV2Config::default()
        .max_partitioning_rounds(2)
        .cross_shard_dep_avoid_threshold(0.9)
        .partition_last_round(true)
        .build();
    let partitioned_txns = partitioner.partition(transactions.clone(), num_shards);
    let sharded_txn_output = sharded_block_executor
        .execute_block(
            Arc::new(state_store.clone()),
            partitioned_txns.clone(),
            2,
            BlockExecutorConfigFromOnchain::new_no_block_limit(),
        )
        .unwrap();
    let txns: Vec<SignatureVerifiedTransaction> =
        PartitionedTransactions::flatten(partitioned_txns)
            .into_iter()
            .map(|t| t.into_txn())
            .collect();
    let txn_provider = DefaultTxnProvider::new_without_info(txns);
    let unsharded_txn_output = AptosVMBlockExecutor::new()
        .execute_block_no_limit(&txn_provider, &state_store)
        .unwrap();
    compare_txn_outputs(unsharded_txn_output, sharded_txn_output);
    sharded_block_executor.shutdown();
}

pub fn sharded_block_executor_with_conflict<E: ExecutorClient<InMemoryStateStore>>(
    mut sharded_block_executor: ShardedBlockExecutor<InMemoryStateStore, E>,
    concurrency: usize,
) {
    let num_txns = 800;
    let num_shards = sharded_block_executor.num_shards();
    let num_accounts = 80;
    let state_store = InMemoryStateStore::from_head_genesis();
    let mut transactions = Vec::new();
    let mut accounts = Vec::new();
    let mut txn_hash_to_account = HashMap::new();
    for _ in 0..num_accounts {
        let account = generate_account_at(&state_store, AccountAddress::random());
        accounts.push(Mutex::new(account));
    }
    for i in 1..num_txns / num_accounts {
        for j in 0..num_accounts {
            let sender = &mut accounts[j].lock().unwrap();
            let sender_addr = *sender.address();
            let receiver = &accounts[(j + i) % num_accounts].lock().unwrap();
            let transfer_amount = 1_000;
            let txn = generate_p2p_txn(sender, receiver, transfer_amount);
            txn_hash_to_account.insert(txn.transaction().hash(), sender_addr);
            transactions.push(txn)
        }
    }

    let partitioner = PartitionerV2Config::default()
        .max_partitioning_rounds(2)
        .cross_shard_dep_avoid_threshold(0.9)
        .partition_last_round(true)
        .build();
    let partitioned_txns = partitioner.partition(transactions.clone(), num_shards);

    let execution_ordered_txns: Vec<SignatureVerifiedTransaction> =
        PartitionedTransactions::flatten(partitioned_txns.clone())
            .into_iter()
            .map(|t| t.into_txn())
            .collect();
    let sharded_txn_output = sharded_block_executor
        .execute_block(
            Arc::new(state_store.clone()),
            partitioned_txns,
            concurrency,
            BlockExecutorConfigFromOnchain::new_no_block_limit(),
        )
        .unwrap();

    let txn_provider = DefaultTxnProvider::new_without_info(execution_ordered_txns);
    let unsharded_txn_output = AptosVMBlockExecutor::new()
        .execute_block_no_limit(&txn_provider, &state_store)
        .unwrap();
    compare_txn_outputs(unsharded_txn_output, sharded_txn_output);
    sharded_block_executor.shutdown();
}
