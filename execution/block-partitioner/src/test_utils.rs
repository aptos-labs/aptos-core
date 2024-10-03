// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
use crate::{BlockPartitioner, Sender};
#[cfg(test)]
use aptos_crypto::hash::CryptoHash;
#[cfg(test)]
use aptos_crypto::hash::TestOnlyHash;
#[cfg(test)]
use aptos_crypto::HashValue;
use aptos_crypto::{ed25519::ed25519_keys::Ed25519PrivateKey, PrivateKey, SigningKey, Uniform};
#[cfg(test)]
use aptos_types::block_executor::partitioner::PartitionedTransactions;
#[cfg(test)]
use aptos_types::block_executor::partitioner::RoundId;
#[cfg(test)]
use aptos_types::block_executor::partitioner::ShardId;
#[cfg(test)]
use aptos_types::block_executor::partitioner::TransactionWithDependencies;
#[cfg(test)]
use aptos_types::block_executor::partitioner::GLOBAL_ROUND_ID;
#[cfg(test)]
use aptos_types::block_executor::partitioner::GLOBAL_SHARD_ID;
#[cfg(test)]
use aptos_types::state_store::state_key::StateKey;
use aptos_types::{
    chain_id::ChainId,
    transaction::{
        analyzed_transaction::AnalyzedTransaction, EntryFunction, RawTransaction,
        SignedTransaction, Transaction, TransactionPayload,
    },
    AptosCoinType, CoinType,
};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
};
#[cfg(test)]
use rand::thread_rng;
use rand::Rng;
use rayon::{iter::ParallelIterator, prelude::IntoParallelIterator};
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct TestAccount {
    pub account_address: AccountAddress,
    pub private_key: Ed25519PrivateKey,
    pub sequence_number: u64,
}

pub fn generate_test_account() -> TestAccount {
    TestAccount {
        account_address: AccountAddress::random(),
        private_key: Ed25519PrivateKey::generate_for_testing(),
        sequence_number: 0,
    }
}

pub fn generate_test_account_for_address(account_address: AccountAddress) -> TestAccount {
    TestAccount {
        account_address,
        private_key: Ed25519PrivateKey::generate_for_testing(),
        sequence_number: 0,
    }
}

pub fn create_non_conflicting_p2p_transaction() -> AnalyzedTransaction {
    // create unique sender and receiver accounts so that there is no conflict
    let mut sender = generate_test_account();
    let receiver = generate_test_account();
    create_signed_p2p_transaction(&mut sender, vec![&receiver]).remove(0)
}

pub fn create_signed_p2p_transaction(
    sender: &mut TestAccount,
    receivers: Vec<&TestAccount>,
) -> Vec<AnalyzedTransaction> {
    let mut transactions = Vec::new();
    for receiver in receivers.iter() {
        let transaction_payload = TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(AccountAddress::ONE, Identifier::new("coin").unwrap()),
            Identifier::new("transfer").unwrap(),
            vec![AptosCoinType::type_tag()],
            vec![
                bcs::to_bytes(&receiver.account_address).unwrap(),
                bcs::to_bytes(&1u64).unwrap(),
            ],
        ));

        let raw_transaction = RawTransaction::new(
            sender.account_address,
            sender.sequence_number,
            transaction_payload,
            0,
            0,
            0,
            ChainId::new(10),
        );
        sender.sequence_number += 1;
        let txn = Transaction::UserTransaction(SignedTransaction::new(
            raw_transaction.clone(),
            sender.private_key.public_key().clone(),
            sender.private_key.sign(&raw_transaction).unwrap(),
        ));
        transactions.push(txn.into())
    }
    transactions
}

pub struct P2PBlockGenerator {
    accounts: Arc<Vec<Mutex<TestAccount>>>,
}

impl P2PBlockGenerator {
    pub fn new(num_accounts: usize) -> Self {
        let accounts = (0..num_accounts)
            .into_par_iter()
            .map(|_i| Mutex::new(generate_test_account()))
            .collect();
        Self {
            accounts: Arc::new(accounts),
        }
    }

    pub fn rand_block<R>(&self, rng: &mut R, block_size: usize) -> Vec<AnalyzedTransaction>
    where
        R: Rng,
    {
        (0..block_size)
            .map(|_| {
                let indices = rand::seq::index::sample(rng, self.accounts.len(), 2);
                let receiver = self.accounts[indices.index(1)].lock().unwrap();
                let mut sender = self.accounts[indices.index(0)].lock().unwrap();
                create_signed_p2p_transaction(&mut sender, vec![&receiver]).remove(0)
            })
            .collect()
    }
}

/// Assert partitioner correctness for `ShardedBlockPartitioner` and `V2Partitioner`:
/// - Transaction set remains the same after partitioning.
/// - The relative order of the txns from the same sender
/// - For a cross-shard dependency, the consumer txn always comes after the provider txn in the sharded block.
/// - Required edge set matches dependency edge set.
/// - Before the last round, there is no in-round cross-shard dependency.
///
/// Also print a summary of the partitioning result.
#[cfg(test)]
pub fn verify_partitioner_output(input: &[AnalyzedTransaction], output: &PartitionedTransactions) {
    let old_txn_id_by_txn_hash: HashMap<HashValue, usize> = HashMap::from_iter(
        input
            .iter()
            .enumerate()
            .map(|(tid, txn)| (txn.test_only_hash(), tid)),
    );

    let mut total_comm_cost = 0;
    let num_txns = input.len();
    let num_shards = output.sharded_txns().len();
    let num_rounds = output
        .sharded_txns()
        .first()
        .map(|sbs| sbs.sub_blocks.len())
        .unwrap_or(0);
    for sub_block_list in output.sharded_txns().iter().take(num_shards).skip(1) {
        assert_eq!(num_rounds, sub_block_list.sub_blocks.len());
    }
    let mut old_txn_idxs_by_sender: HashMap<Sender, Vec<usize>> = HashMap::new();
    let mut old_txn_idxs_seen: HashSet<usize> = HashSet::new();
    let mut edge_set_from_src_view: HashSet<(usize, usize, usize, HashValue, usize, usize, usize)> =
        HashSet::new();
    let mut edge_set_from_dst_view: HashSet<(usize, usize, usize, HashValue, usize, usize, usize)> =
        HashSet::new();

    let mut for_each_sub_block = |round_id: usize,
                                  shard_id: usize,
                                  start_txn_idx: usize,
                                  sub_block_txns: &[TransactionWithDependencies<
        AnalyzedTransaction,
    >]| {
        let mut cur_sub_block_inbound_costs: HashMap<(RoundId, ShardId, StateKey), u64> =
            HashMap::new();
        let mut cur_sub_block_outbound_costs: HashMap<(RoundId, ShardId, StateKey), u64> =
            HashMap::new();
        for (pos_in_sub_block, txn_with_dep) in sub_block_txns.iter().enumerate() {
            let sender = txn_with_dep.txn.sender();
            let old_txn_idx = *old_txn_id_by_txn_hash
                .get(&txn_with_dep.txn().test_only_hash())
                .unwrap();
            old_txn_idxs_seen.insert(old_txn_idx);
            old_txn_idxs_by_sender
                .entry(sender)
                .or_default()
                .push(old_txn_idx);
            let new_txn_idx = start_txn_idx + pos_in_sub_block;
            for loc in txn_with_dep.txn.write_hints().iter() {
                let key = loc.clone().into_state_key();
                let key_str = CryptoHash::hash(&key).to_hex();
                println!(
                    "MATRIX_REPORT - round={}, shard={}, old_tid={}, new_tid={}, write_hint={}",
                    round_id, shard_id, old_txn_idx, new_txn_idx, key_str
                );
            }
            for (src_txn_idx, locs) in txn_with_dep
                .cross_shard_dependencies
                .required_edges()
                .iter()
            {
                for loc in locs.iter() {
                    let key = loc.clone().into_state_key();
                    let key_str = CryptoHash::hash(&key).to_hex();
                    println!(
                        "MATRIX_REPORT - round={}, shard={}, old_tid={}, new_tid={}, wait for key={} from round={}, shard={}, new_tid={}",
                        round_id, shard_id, old_txn_idx, new_txn_idx, key_str, src_txn_idx.round_id, src_txn_idx.shard_id, src_txn_idx.txn_index
                    );

                    if round_id != num_rounds - 1 {
                        assert_ne!(src_txn_idx.round_id, round_id);
                    }
                    assert!((src_txn_idx.round_id, src_txn_idx.shard_id) < (round_id, shard_id));
                    edge_set_from_dst_view.insert((
                        src_txn_idx.round_id,
                        src_txn_idx.shard_id,
                        src_txn_idx.txn_index,
                        CryptoHash::hash(&key),
                        round_id,
                        shard_id,
                        new_txn_idx,
                    ));
                    let value = cur_sub_block_inbound_costs
                        .entry((src_txn_idx.round_id, src_txn_idx.shard_id, key))
                        .or_insert_with(|| 0);
                    *value += 1;
                }
            }
            for (dst_tid, locs) in txn_with_dep
                .cross_shard_dependencies
                .dependent_edges()
                .iter()
            {
                for loc in locs.iter() {
                    let key = loc.clone().into_state_key();
                    let key_str = CryptoHash::hash(&key).to_hex();
                    println!(
                        "MATRIX_REPORT - round={}, shard={}, old_tid={}, new_tid={}, send key={} to round={}, shard={}, new_tid={}",
                        round_id, shard_id, old_txn_idx, new_txn_idx, key_str, dst_tid.round_id, dst_tid.shard_id, dst_tid.txn_index
                    );

                    if round_id != num_rounds - 1 {
                        assert_ne!(dst_tid.round_id, round_id);
                    }
                    assert!((round_id, shard_id) < (dst_tid.round_id, dst_tid.shard_id));
                    edge_set_from_src_view.insert((
                        round_id,
                        shard_id,
                        new_txn_idx,
                        CryptoHash::hash(&key),
                        dst_tid.round_id,
                        dst_tid.shard_id,
                        dst_tid.txn_index,
                    ));
                    let value = cur_sub_block_outbound_costs
                        .entry((dst_tid.round_id, dst_tid.shard_id, key))
                        .or_insert_with(|| 0);
                    *value += 1;
                }
            }
        }
        let inbound_cost: u64 = cur_sub_block_inbound_costs.values().copied().sum();
        let outbound_cost: u64 = cur_sub_block_outbound_costs.values().copied().sum();
        println!("MATRIX_REPORT: round={}, shard={}, sub_block_size={}, inbound_cost={}, outbound_cost={}", round_id, shard_id, sub_block_txns.len(), inbound_cost, outbound_cost);
        if round_id == 0 {
            assert_eq!(0, inbound_cost);
        }
        total_comm_cost += inbound_cost + outbound_cost;
    };

    for round_id in 0..num_rounds {
        for (shard_id, sub_block_list) in output.sharded_txns().iter().enumerate() {
            let sub_block = sub_block_list.get_sub_block(round_id).unwrap();
            for_each_sub_block(
                round_id,
                shard_id,
                sub_block.start_index,
                sub_block.transactions_with_deps().as_slice(),
            )
        }
    }
    for_each_sub_block(
        GLOBAL_ROUND_ID,
        GLOBAL_SHARD_ID,
        output.num_sharded_txns(),
        output.global_txns.as_slice(),
    );

    assert_eq!(HashSet::from_iter(0..num_txns), old_txn_idxs_seen);
    assert_eq!(edge_set_from_src_view, edge_set_from_dst_view);
    for (_sender, old_tids) in old_txn_idxs_by_sender {
        assert!(is_sorted(&old_tids));
    }
    println!("MATRIX_REPORT: total_comm_cost={}", total_comm_cost);
}

#[cfg(test)]
fn is_sorted(arr: &[usize]) -> bool {
    let num = arr.len();
    for i in 1..num {
        if arr[i - 1] >= arr[i] {
            return false;
        }
    }
    true
}

#[cfg(test)]
pub fn assert_deterministic_result(partitioner: Arc<dyn BlockPartitioner>) {
    let mut rng = thread_rng();
    let block_gen = P2PBlockGenerator::new(1000);
    for _ in 0..10 {
        let txns = block_gen.rand_block(&mut rng, 100);
        let result_0 = partitioner.partition(txns.clone(), 10);
        for _ in 0..2 {
            let result_1 = partitioner.partition(txns.clone(), 10);
            assert_eq!(result_1, result_0);
        }
    }
}
