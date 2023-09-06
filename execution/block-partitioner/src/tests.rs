// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    test_utils::{
        create_non_conflicting_p2p_transaction, create_signed_p2p_transaction,
        generate_test_account, generate_test_account_for_address,
    },
    v2::config::PartitionerV2Config,
};
use aptos_crypto::hash::CryptoHash;
use aptos_types::{
    block_executor::partitioner::{ShardedTxnIndex, SubBlock, SubBlocksForShard},
    transaction::{analyzed_transaction::AnalyzedTransaction, Transaction},
};
use move_core_types::account_address::AccountAddress;
use rand::{rngs::OsRng, Rng};
use std::{collections::HashMap, sync::Mutex};

fn verify_no_cross_shard_dependency(sub_blocks_for_shards: Vec<SubBlock<AnalyzedTransaction>>) {
    for sub_blocks in sub_blocks_for_shards {
        for txn in sub_blocks.iter() {
            assert_eq!(txn.cross_shard_dependencies().num_required_edges(), 0);
        }
    }
}

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
    assert_eq!(partitioned_txns.num_shards(), num_shards);
    // Verify that the transactions are in the same order as the original transactions and cross shard
    // dependencies are empty.
    let (sub_blocks, _) = partitioned_txns.into();
    let mut current_index = 0;
    for sub_blocks_for_shard in sub_blocks.into_iter() {
        assert_eq!(sub_blocks_for_shard.num_txns(), num_txns / num_shards);
        for txn in sub_blocks_for_shard.iter() {
            assert_eq!(
                txn.txn().transaction(),
                transactions[current_index].transaction()
            );
            assert_eq!(txn.cross_shard_dependencies().num_required_edges(), 0);
            current_index += 1;
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
            let (sender, seq_number) = get_account_seq_number(txn.transaction());
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
fn test_cross_shard_dependencies() {
    let num_shards = 3;
    let mut account1 = generate_test_account_for_address(AccountAddress::new([0; 32]));
    let mut account2 = generate_test_account_for_address(AccountAddress::new([1; 32]));
    let account3 = generate_test_account_for_address(AccountAddress::new([2; 32]));
    let mut account4 = generate_test_account_for_address(AccountAddress::new([4; 32]));
    let account5 = generate_test_account_for_address(AccountAddress::new([5; 32]));
    let account6 = generate_test_account_for_address(AccountAddress::new([6; 32]));
    let mut account7 = generate_test_account_for_address(AccountAddress::new([7; 32]));
    let account8 = generate_test_account_for_address(AccountAddress::new([8; 32]));
    let account9 = generate_test_account_for_address(AccountAddress::new([9; 32]));

    let txn0 = create_signed_p2p_transaction(&mut account1, vec![&account2]).remove(0); // txn 0
    let txn1 = create_signed_p2p_transaction(&mut account1, vec![&account3]).remove(0); // txn 1
    let txn2 = create_signed_p2p_transaction(&mut account2, vec![&account3]).remove(0); // txn 2
                                                                                        // Should go in shard 1
    let txn3 = create_signed_p2p_transaction(&mut account4, vec![&account5]).remove(0); // txn 3
    let txn4 = create_signed_p2p_transaction(&mut account4, vec![&account6]).remove(0); // txn 4
    let txn5 = create_signed_p2p_transaction(&mut account4, vec![&account6]).remove(0); // txn 5
                                                                                        // Should go in shard 2
    let txn6 = create_signed_p2p_transaction(&mut account7, vec![&account8]).remove(0); // txn 6
    let txn7 = create_signed_p2p_transaction(&mut account7, vec![&account9]).remove(0); // txn 7
    let txn8 = create_signed_p2p_transaction(&mut account4, vec![&account7]).remove(0); // txn 8

    let transactions = vec![
        txn0.clone(),
        txn1.clone(),
        txn2.clone(),
        txn3.clone(),
        txn4.clone(),
        txn5.clone(),
        txn6.clone(),
        txn7.clone(),
        txn8.clone(),
    ];

    let partitioner = PartitionerV2Config::default().build();
    let (partitioned_sub_blocks, _) = partitioner.partition(transactions, num_shards).into();
    assert_eq!(partitioned_sub_blocks.len(), num_shards);

    // In first round of the partitioning, we should have txn0, txn1 and txn2 in shard 0 and
    // txn3, txn4, txn5 in shard 1 and txn6 and txn7 in shard 2.
    assert_eq!(
        partitioned_sub_blocks[0]
            .get_sub_block(0)
            .unwrap()
            .num_txns(),
        3
    );
    assert_eq!(
        partitioned_sub_blocks[1]
            .get_sub_block(0)
            .unwrap()
            .num_txns(),
        3
    );
    assert_eq!(
        partitioned_sub_blocks[2]
            .get_sub_block(0)
            .unwrap()
            .num_txns(),
        2
    );

    assert_eq!(
        partitioned_sub_blocks[0]
            .get_sub_block(0)
            .unwrap()
            .iter()
            .map(|x| x.txn.clone())
            .collect::<Vec<AnalyzedTransaction>>(),
        vec![txn0, txn1, txn2]
    );
    assert_eq!(
        partitioned_sub_blocks[1]
            .get_sub_block(0)
            .unwrap()
            .iter()
            .map(|x| x.txn.clone())
            .collect::<Vec<AnalyzedTransaction>>(),
        vec![txn3, txn4, txn5]
    );

    assert_eq!(
        partitioned_sub_blocks[2]
            .get_sub_block(0)
            .unwrap()
            .iter()
            .map(|x| x.txn.clone())
            .collect::<Vec<AnalyzedTransaction>>(),
        vec![txn6.clone(), txn7.clone()]
    );
    //
    // // Rest of the transactions will be added in round 2 along with their dependencies
    assert_eq!(
        partitioned_sub_blocks[0]
            .get_sub_block(1)
            .unwrap()
            .num_txns(),
        0
    );
    assert_eq!(
        partitioned_sub_blocks[1]
            .get_sub_block(1)
            .unwrap()
            .num_txns(),
        0
    );
    assert_eq!(
        partitioned_sub_blocks[2]
            .get_sub_block(1)
            .unwrap()
            .num_txns(),
        1
    );

    assert_eq!(
        partitioned_sub_blocks[2]
            .get_sub_block(1)
            .unwrap()
            .iter()
            .map(|x| x.txn.clone())
            .collect::<Vec<AnalyzedTransaction>>(),
        vec![txn8.clone()]
    );

    // Verify transaction dependencies
    verify_no_cross_shard_dependency(vec![
        partitioned_sub_blocks[0].get_sub_block(0).unwrap().clone(),
        partitioned_sub_blocks[1].get_sub_block(0).unwrap().clone(),
        partitioned_sub_blocks[2].get_sub_block(0).unwrap().clone(),
    ]);

    // Verify transaction depends_on and dependency list
    // txn8 (index 8) depends on txn5 (index 5) and txn7 (index 7)
    partitioned_sub_blocks[2]
        .get_sub_block(1)
        .unwrap()
        .iter()
        .for_each(|txn| {
            let required_deps = txn
                .cross_shard_dependencies
                .get_required_edge_for(ShardedTxnIndex::new(5, 1, 0))
                .unwrap();
            assert_eq!(required_deps.len(), 2);
            assert!(
                required_deps.contains(&AnalyzedTransaction::coin_store_location(
                    account4.account_address
                ))
            );
            assert!(
                required_deps.contains(&AnalyzedTransaction::account_resource_location(
                    account4.account_address
                ))
            );
        });

    // Verify the dependent edges, again the conflict is only on the coin store of account 7
    let required_deps = partitioned_sub_blocks[1]
        .get_sub_block(0)
        .unwrap()
        .transactions[2]
        .cross_shard_dependencies
        .get_dependent_edge_for(ShardedTxnIndex::new(8, 2, 1))
        .unwrap();
    assert_eq!(required_deps.len(), 2);
    assert!(
        required_deps.contains(&AnalyzedTransaction::coin_store_location(
            account4.account_address
        ))
    );
    assert!(
        required_deps.contains(&AnalyzedTransaction::account_resource_location(
            account4.account_address
        ))
    );

    let required_deps = partitioned_sub_blocks[2]
        .get_sub_block(0)
        .unwrap()
        .transactions[1]
        .cross_shard_dependencies
        .get_dependent_edge_for(ShardedTxnIndex::new(8, 2, 1))
        .unwrap();
    assert_eq!(required_deps.len(), 1);
    assert_eq!(
        required_deps[0],
        AnalyzedTransaction::coin_store_location(account7.account_address)
    );
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
                let storage_locations = analyzed_txn
                    .read_hints()
                    .iter()
                    .chain(analyzed_txn.write_hints().iter());
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
