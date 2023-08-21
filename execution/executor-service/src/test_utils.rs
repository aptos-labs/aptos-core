// Copyright © Aptos Foundation

use aptos_block_partitioner::BlockPartitionerConfig;
use aptos_language_e2e_tests::{
    account::AccountData, common_transactions::peer_to_peer_txn, data_store::FakeDataStore,
    executor::FakeExecutor,
};
use aptos_types::{
    state_store::state_key::StateKeyInner,
    transaction::{analyzed_transaction::AnalyzedTransaction, Transaction, TransactionOutput},
};
use aptos_vm::{
    sharded_block_executor::{executor_client::ExecutorClient, ShardedBlockExecutor},
    AptosVM, VMExecutor,
};
use std::sync::Arc;

fn generate_non_conflicting_sender_receiver(
    executor: &mut FakeExecutor,
) -> (AccountData, AccountData) {
    let sender = executor.create_raw_account_data(3_000_000_000, 0);
    let receiver = executor.create_raw_account_data(3_000_000_000, 0);
    executor.add_account_data(&sender);
    executor.add_account_data(&receiver);
    (sender, receiver)
}

pub fn generate_non_conflicting_p2p(
    executor: &mut FakeExecutor,
) -> (AnalyzedTransaction, AccountData, AccountData) {
    let (mut sender, receiver) = generate_non_conflicting_sender_receiver(executor);
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
            .into_iter()
            .filter(|(k, _)| matches!(k.inner(), &StateKeyInner::AccessPath(_)))
            .collect::<Vec<_>>();
        let sharded_write_set_without_table_item = sharded_txn_output[i]
            .write_set()
            .into_iter()
            .filter(|(k, _)| matches!(k.inner(), &StateKeyInner::AccessPath(_)))
            .collect::<Vec<_>>();
        assert_eq!(
            unsharded_write_set_without_table_item,
            sharded_write_set_without_table_item
        );
    }
}

pub fn test_sharded_block_executor_no_conflict<E: ExecutorClient<FakeDataStore>>(
    sharded_block_executor: ShardedBlockExecutor<FakeDataStore, E>,
) {
    let num_txns = 400;
    let num_shards = sharded_block_executor.num_shards();
    let mut executor = FakeExecutor::from_head_genesis();
    let mut transactions = Vec::new();
    for _ in 0..num_txns {
        transactions.push(generate_non_conflicting_p2p(&mut executor).0)
    }
    let partitioner = BlockPartitionerConfig::default()
        .num_shards(num_shards)
        .max_partitioning_rounds(2)
        .cross_shard_dep_avoid_threshold(0.9)
        .partition_last_round(true)
        .build();
    let partitioned_txns = partitioner.partition(transactions.clone());
    let sharded_txn_output = sharded_block_executor
        .execute_block(
            Arc::new(executor.data_store().clone()),
            partitioned_txns,
            2,
            None,
        )
        .unwrap();
    let unsharded_txn_output = AptosVM::execute_block(
        transactions.into_iter().map(|t| t.into_txn()).collect(),
        &executor.data_store(),
        None,
    )
    .unwrap();
    compare_txn_outputs(unsharded_txn_output, sharded_txn_output);
}
