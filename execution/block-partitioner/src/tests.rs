// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    PartitionerConfig,
    test_utils::{
        create_non_conflicting_p2p_transaction, create_signed_p2p_transaction,
        generate_test_account, verify_partitioner_output,
    },
    v2::config::PartitionerV2Config,
};
use aptos_types::{block_executor::partitioner::SubBlocksForShard, transaction::Transaction};
use move_core_types::account_address::AccountAddress;
use rand::{Rng, rngs::OsRng};
use std::{collections::HashMap, sync::Mutex};

#[test]
// Test that the partitioner works correctly for no conflict transactions. In this case, the
// expectation is that no transaction is reordered.
fn test_non_conflicting_txns() {
    let num_txns = 4;
    let num_shards = 2;
    let mut transactions = Vec::new();
    for _ in 0..num_txns {
        transactions.push(create_non_conflicting_p2p_transaction())
    }
    let partitioner = PartitionerV2Config::default().build();
    let partitioned_txns = partitioner.partition(transactions.clone(), num_shards);
    verify_partitioner_output(&transactions, &partitioned_txns);
    assert_eq!(partitioned_txns.num_shards(), num_shards);
    // Verify that the transactions are in the same order as the original transactions and cross shard
    // dependencies are empty.
    let (sub_blocks, _) = partitioned_txns.into();
    for sub_blocks_for_shard in sub_blocks.into_iter() {
        assert_eq!(sub_blocks_for_shard.num_txns(), num_txns / num_shards);
        for txn in sub_blocks_for_shard.iter() {
            assert_eq!(txn.cross_shard_dependencies().num_required_edges(), 0);
        }
    }
}

fn get_account_seq_number(txn: &Transaction) -> (AccountAddress, u64) {
    match txn {
        Transaction::UserTransaction(txn) => (txn.sender(), txn.sequence_number()),
        _ => unreachable!("Only user transaction can be executed in executor"),
    }
}

#[test]
// Ensures that transactions from the same sender are not reordered.
fn test_relative_ordering_for_sender() {
    let mut rng = OsRng;
    let num_shards = 8;
    let num_accounts = 50;
    let num_txns = 500;
    let mut accounts = Vec::new();
    for _ in 0..num_accounts {
        accounts.push(Mutex::new(generate_test_account()));
    }
    let mut transactions = Vec::new();

    for _ in 0..num_txns {
        let indices = rand::seq::index::sample(&mut rng, num_accounts, 2);
        let sender = &mut accounts[indices.index(0)].lock().unwrap();
        let receiver = &accounts[indices.index(1)].lock().unwrap();
        let txn = create_signed_p2p_transaction(sender, vec![receiver]).remove(0);
        transactions.push(txn.clone());
        transactions.push(create_signed_p2p_transaction(sender, vec![receiver]).remove(0));
    }

    let partitioner = PartitionerV2Config::default().build();
    let (sub_blocks, _) = partitioner
        .partition(transactions.clone(), num_shards)
        .into();

    let mut account_to_expected_seq_number: HashMap<AccountAddress, u64> = HashMap::new();
    SubBlocksForShard::flatten(sub_blocks)
        .iter()
        .for_each(|txn| {
            let (sender, seq_number) = get_account_seq_number(txn.transaction().expect_valid());
            if account_to_expected_seq_number.contains_key(&sender) {
                assert_eq!(
                    account_to_expected_seq_number.get(&sender).unwrap(),
                    &seq_number
                );
            }
            account_to_expected_seq_number.insert(sender, seq_number + 1);
        });
}

#[test]
// Generates a bunch of random transactions and ensures that after the partitioning, there is
// no conflict across shards.
fn test_no_conflict_across_shards_in_non_last_rounds() {
    let mut rng = OsRng;
    let max_accounts = 500;
    let max_txns = 5000;
    let max_num_shards = 64;
    let num_accounts = rng.gen_range(1, max_accounts);
    let mut accounts = Vec::new();
    for _ in 0..num_accounts {
        accounts.push(generate_test_account());
    }
    let num_txns = rng.gen_range(1, max_txns);
    let mut transactions = Vec::new();
    let mut txns_by_hash = HashMap::new();
    let num_shards = rng.gen_range(1, max_num_shards);

    for _ in 0..num_txns {
        // randomly select a sender and receiver from accounts
        let sender_index = rng.gen_range(0, accounts.len());
        let mut sender = accounts.swap_remove(sender_index);
        let receiver_index = rng.gen_range(0, accounts.len());
        let receiver = accounts.get(receiver_index).unwrap();
        let analyzed_txn = create_signed_p2p_transaction(&mut sender, vec![receiver]).remove(0);
        txns_by_hash.insert(analyzed_txn.transaction().hash(), analyzed_txn.clone());
        transactions.push(analyzed_txn);
        accounts.push(sender)
    }
    let partitioner = PartitionerV2Config::default().build();
    let (sub_blocks, _) = partitioner.partition(transactions, num_shards).into();
    // Build a map of storage location to corresponding shards in first round
    // and ensure that no storage location is present in more than one shard.
    let num_partitioning_rounds = sub_blocks[0].num_sub_blocks() - 1;
    for round in 0..num_partitioning_rounds {
        let mut storage_location_to_shard_map = HashMap::new();
        for (shard_id, sub_blocks_for_shard) in sub_blocks.iter().enumerate() {
            let sub_block_for_round = sub_blocks_for_shard.get_sub_block(round).unwrap();
            for txn in sub_block_for_round.iter() {
                let analyzed_txn = txns_by_hash.get(&txn.txn().transaction().hash()).unwrap();
                let storage_locations = analyzed_txn.write_hints().iter();
                for storage_location in storage_locations {
                    if storage_location_to_shard_map.contains_key(storage_location) {
                        assert_eq!(
                            storage_location_to_shard_map.get(storage_location).unwrap(),
                            &shard_id
                        );
                    } else {
                        storage_location_to_shard_map.insert(storage_location, shard_id);
                    }
                }
            }
        }
    }
}
