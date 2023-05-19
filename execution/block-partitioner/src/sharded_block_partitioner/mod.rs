// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    sharded_block_partitioner::{
        dependency_analysis::RWSetWithTxnIndex,
        messages::{
            AddTxnsWithCrossShardDep, ControlMsg,
            ControlMsg::{AddCrossShardDepReq, DiscardCrossShardDepReq},
            CrossShardMsg, DiscardTxnsWithCrossShardDep, PartitioningBlockResponse,
        },
        partitioning_shard::PartitioningShard,
    },
    types::{ShardId, TransactionsChunk},
};
use aptos_logger::{error, info};
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use std::{
    collections::HashMap,
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
    thread,
};

mod conflict_detector;
mod dependency_analysis;
mod messages;
mod partitioning_shard;

/// A sharded block partitioner that partitions a block into multiple transaction chunks.
/// On a high level, the partitioning process is as follows:
/// ```plaintext
/// 1. A block is partitioned into equally sized transaction chunks and sent to each shard.
///
///    Block:
///
///    T1 {write set: A, B}
///    T2 {write set: B, C}
///    T3 {write set: C, D}
///    T4 {write set: D, E}
///    T5 {write set: E, F}
///    T6 {write set: F, G}
///    T7 {write set: G, H}
///    T8 {write set: H, I}
///    T9 {write set: I, J}
///
/// 2. Discard a bunch of transactions from the chunks and create new chunks so that
///    there is no cross-shard dependency between transactions in a chunk.
///   2.1 Following information is passed to each shard:
///      - candidate transaction chunks to be partitioned
///      - previously frozen transaction chunks (if any)
///      - read-write set index mapping from previous iteration (if any) - this contains the maximum absolute index
///        of the transaction that read/wrote to a storage location indexed by the storage location.
///   2.1 Each shard creates a read-write set for all transactions in the chunk and broadcasts it to all other shards.
///      Shard 0                          Shard 1                           Shard 2
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///    |        Read-Write Set      |  |         Read-Write Set         |  |         Read-Write Set         |
///    |                            |  |                               |  |                               |
///    |   T1 {A, B}                |  |   T4 {D, E}                   |  |   T7 {G, H}                   |
///    |   T2 {B, C}                |  |   T5 {E, F}                   |  |   T8 {H, I}                   |
///    |   T3 {C, D}                |  |   T6 {F, G}                   |  |   T9 {I, J}                   |
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///   2.2 Each shard collects read-write sets from all other shards and discards transactions that have cross-shard dependencies.
///              Shard 0                          Shard 1                           Shard 2
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///    |        Discarded Txns      |  |         Discarded Txns         |  |         Discarded Txns         |
///    |                            |  |                               |  |                               |
///    |   - T3 (cross-shard dependency with T4) |  |   - T6 (cross-shard dependency with T7) |  | No discard |
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///   2.3 Each shard broadcasts the number of transactions that it plans to put in the current chunk.
///      Shard 0                          Shard 1                           Shard 2
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///    |          Chunk Count       |  |          Chunk Count          |  |          Chunk Count          |
///    |                            |  |                               |  |                               |
///    |   2                        |  |   2                           |  |      3                        |
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///   2.4 Each shard collects the number of transactions that all other shards plan to put in the current chunk and based
///      on that, it finalizes the absolute index offset of the current chunk. It uses this information to create a read-write set
///      index, which is a mapping of all the storage location to the maximum absolute index of the transaction that read/wrote to that location.
///      Shard 0                          Shard 1                           Shard 2
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///    |          Index Offset      |  |          Index Offset         |  |          Index Offset         |
///    |                            |  |                               |  |                               |
///    |   0                        |  |   2                           |  |   4                           |
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///   2.5 It also uses the read-write set index mapping passed in previous iteration to add cross-shard dependencies to the transactions. This is
///     done by looking up the read-write set index for each storage location that a transaction reads/writes to and adding a cross-shard dependency
///   2.6 Returns two lists of transactions: one list of transactions that are discarded and another list of transactions that are kept.
/// 3. Use the discarded transactions to create new chunks and repeat the step 2 until N iterations.
/// 4. For remaining transaction chunks, add cross-shard dependencies to the transactions. This is done as follows:
///   4.1 Create a read-write set with index mapping for all the transactions in the remaining chunks.
///   4.2 Broadcast and collect read-write set with index mapping from all shards.
///   4.3 Add cross-shard dependencies to the transactions in the remaining chunks by looking up the read-write set index
///       for each storage location that a transaction reads/writes to. The idea is to find the maximum transaction index
///       that reads/writes to the same location and add that as a dependency. This can be done as follows: First look up the read-write set index
///       mapping received from other shards in current iteration in descending order of shard id. If the read-write set index is not found,
///       look up the read-write set index mapping received from other shards in previous iteration(s) in descending order of shard id.
/// ```
///
pub struct ShardedBlockPartitioner {
    num_shards: usize,
    control_txs: Vec<Sender<ControlMsg>>,
    result_rxs: Vec<Receiver<PartitioningBlockResponse>>,
    shard_threads: Vec<thread::JoinHandle<()>>,
}

impl ShardedBlockPartitioner {
    pub fn new(num_shards: usize) -> Self {
        info!(
            "Creating a new sharded block partitioner with {} shards",
            num_shards
        );
        assert!(num_shards > 0, "num_partitioning_shards must be > 0");
        // create channels for cross shard messages across all shards. This is a full mesh connection.
        // Each shard has a vector of channels for sending messages to other shards and
        // a vector of channels for receiving messages from other shards.
        let mut messages_txs = vec![];
        let mut messages_rxs = vec![];
        for _ in 0..num_shards {
            messages_txs.push(vec![]);
            messages_rxs.push(vec![]);
            for _ in 0..num_shards {
                let (messages_tx, messages_rx) = std::sync::mpsc::channel();
                messages_txs.last_mut().unwrap().push(messages_tx);
                messages_rxs.last_mut().unwrap().push(messages_rx);
            }
        }
        let mut control_txs = vec![];
        let mut result_rxs = vec![];
        let mut shard_join_handles = vec![];
        for (i, message_rxs) in messages_rxs.into_iter().enumerate() {
            let (control_tx, control_rx) = std::sync::mpsc::channel();
            let (result_tx, result_rx) = std::sync::mpsc::channel();
            control_txs.push(control_tx);
            result_rxs.push(result_rx);
            shard_join_handles.push(spawn_partitioning_shard(
                i,
                control_rx,
                result_tx,
                message_rxs,
                messages_txs.iter().map(|txs| txs[i].clone()).collect(),
            ));
        }
        Self {
            num_shards,
            control_txs,
            result_rxs,
            shard_threads: shard_join_handles,
        }
    }

    // reorders the transactions so that transactions from the same sender always go to the same shard.
    // This places transactions from the same sender next to each other, which is not optimal for parallelism.
    // TODO(skedia): Improve this logic to shuffle senders
    fn reorder_txns_by_senders(
        &self,
        txns: Vec<AnalyzedTransaction>,
    ) -> Vec<Vec<AnalyzedTransaction>> {
        let approx_txns_per_shard = (txns.len() as f64 / self.num_shards as f64).ceil() as usize;
        let mut sender_to_txns = HashMap::new();
        let mut sender_order = Vec::new(); // Track sender ordering

        for txn in txns {
            let sender = txn.sender().unwrap();
            if !sender_to_txns.contains_key(&sender) {
                sender_order.push(sender); // Add sender to the order vector
            }
            sender_to_txns
                .entry(sender)
                .or_insert_with(Vec::new)
                .push(txn);
        }

        let mut result = Vec::new();
        result.push(Vec::new());

        for sender in sender_order {
            let txns = sender_to_txns.remove(&sender).unwrap();
            let txns_in_shard = result.last().unwrap().len();

            if txns_in_shard < approx_txns_per_shard {
                result.last_mut().unwrap().extend(txns);
            } else {
                result.push(txns);
            }
        }

        // pad the rest of the shard with empty txns
        for _ in result.len()..self.num_shards {
            result.push(Vec::new());
        }

        result
    }

    fn send_partition_msgs(&self, partition_msg: Vec<ControlMsg>) {
        for (i, msg) in partition_msg.into_iter().enumerate() {
            self.control_txs[i].send(msg).unwrap();
        }
    }

    fn collect_partition_block_response(
        &self,
    ) -> (
        Vec<TransactionsChunk>,
        Vec<RWSetWithTxnIndex>,
        Vec<Vec<AnalyzedTransaction>>,
    ) {
        let mut frozen_chunks = Vec::new();
        let mut frozen_rw_set_with_index = Vec::new();
        let mut rejected_txns_vec = Vec::new();
        for rx in &self.result_rxs {
            let PartitioningBlockResponse {
                frozen_chunk,
                rw_set_with_index,
                rejected_txns,
            } = rx.recv().unwrap();
            frozen_chunks.push(frozen_chunk);
            frozen_rw_set_with_index.push(rw_set_with_index);
            rejected_txns_vec.push(rejected_txns);
        }
        (frozen_chunks, frozen_rw_set_with_index, rejected_txns_vec)
    }

    fn discard_txns_with_cross_shard_dependencies(
        &self,
        txns_to_partition: Vec<Vec<AnalyzedTransaction>>,
        frozen_chunks: Arc<Vec<TransactionsChunk>>,
        frozen_rw_set_with_index: Arc<Vec<RWSetWithTxnIndex>>,
    ) -> (
        Vec<TransactionsChunk>,
        Vec<RWSetWithTxnIndex>,
        Vec<Vec<AnalyzedTransaction>>,
    ) {
        let partition_block_msgs = txns_to_partition
            .into_iter()
            .map(|txns| {
                DiscardCrossShardDepReq(DiscardTxnsWithCrossShardDep::new(
                    txns,
                    frozen_rw_set_with_index.clone(),
                    frozen_chunks.clone(),
                ))
            })
            .collect();
        self.send_partition_msgs(partition_block_msgs);
        self.collect_partition_block_response()
    }

    fn add_cross_shard_dependencies(
        &self,
        index_offset: usize,
        remaining_txns_vec: Vec<Vec<AnalyzedTransaction>>,
        frozen_chunks: Arc<Vec<TransactionsChunk>>,
        frozen_rw_set_with_index: Arc<Vec<RWSetWithTxnIndex>>,
    ) -> (
        Vec<TransactionsChunk>,
        Vec<RWSetWithTxnIndex>,
        Vec<Vec<AnalyzedTransaction>>,
    ) {
        let mut index_offset = index_offset;
        let partition_block_msgs = remaining_txns_vec
            .into_iter()
            .map(|remaining_txns| {
                let remaining_txns_len = remaining_txns.len();
                let partitioning_msg = AddCrossShardDepReq(AddTxnsWithCrossShardDep::new(
                    remaining_txns,
                    index_offset,
                    frozen_chunks.clone(),
                    frozen_rw_set_with_index.clone(),
                ));
                index_offset += remaining_txns_len;
                partitioning_msg
            })
            .collect::<Vec<ControlMsg>>();
        self.send_partition_msgs(partition_block_msgs);
        self.collect_partition_block_response()
    }

    /// We repeatedly partition chunks, discarding a bunch of transactions with cross-shard dependencies. The set of discarded
    /// transactions are used as candidate chunks in the next round. This process is repeated until num_partitioning_rounds.
    /// The remaining transactions are then added to the chunks with cross-shard dependencies.
    pub fn partition(
        &self,
        transactions: Vec<AnalyzedTransaction>,
        num_partitioning_round: usize,
    ) -> Vec<TransactionsChunk> {
        let total_txns = transactions.len();
        if total_txns == 0 {
            return vec![];
        }

        // First round, we filter all transactions with cross-shard dependencies
        let mut txns_to_partition = self.reorder_txns_by_senders(transactions);
        let mut frozen_rw_set_with_index = Arc::new(Vec::new());
        let mut frozen_chunks = Arc::new(Vec::new());

        for _ in 0..num_partitioning_round {
            let (
                current_frozen_chunks_vec,
                current_frozen_rw_set_with_index_vec,
                latest_txns_to_partition,
            ) = self.discard_txns_with_cross_shard_dependencies(
                txns_to_partition,
                frozen_chunks.clone(),
                frozen_rw_set_with_index.clone(),
            );
            let mut prev_frozen_chunk = Arc::try_unwrap(frozen_chunks).unwrap();
            let mut prev_frozen_rw_set_with_index =
                Arc::try_unwrap(frozen_rw_set_with_index).unwrap();
            prev_frozen_chunk.extend(current_frozen_chunks_vec);
            prev_frozen_rw_set_with_index.extend(current_frozen_rw_set_with_index_vec);
            frozen_chunks = Arc::new(prev_frozen_chunk);
            frozen_rw_set_with_index = Arc::new(prev_frozen_rw_set_with_index);
            txns_to_partition = latest_txns_to_partition;
            if txns_to_partition
                .iter()
                .map(|txns| txns.len())
                .sum::<usize>()
                == 0
            {
                return Arc::try_unwrap(frozen_chunks).unwrap();
            }
        }

        // We just add cross shard dependencies for remaining transactions.
        let index_offset = frozen_chunks.iter().map(|chunk| chunk.len()).sum::<usize>();
        let (remaining_frozen_chunks, _, _) = self.add_cross_shard_dependencies(
            index_offset,
            txns_to_partition,
            frozen_chunks.clone(),
            frozen_rw_set_with_index,
        );

        Arc::try_unwrap(frozen_chunks)
            .unwrap()
            .into_iter()
            .chain(remaining_frozen_chunks.into_iter())
            .collect::<Vec<TransactionsChunk>>()
    }
}

impl Drop for ShardedBlockPartitioner {
    /// Best effort stops all the executor shards and waits for the thread to finish.
    fn drop(&mut self) {
        // send stop command to all executor shards
        for control_tx in self.control_txs.iter() {
            if let Err(e) = control_tx.send(ControlMsg::Stop) {
                error!("Failed to send stop command to executor shard: {:?}", e);
            }
        }

        // wait for all executor shards to stop
        for shard_thread in self.shard_threads.drain(..) {
            shard_thread.join().unwrap_or_else(|e| {
                error!("Failed to join executor shard thread: {:?}", e);
            });
        }
    }
}

fn spawn_partitioning_shard(
    shard_id: ShardId,
    control_rx: Receiver<ControlMsg>,
    result_tx: Sender<PartitioningBlockResponse>,
    message_rxs: Vec<Receiver<CrossShardMsg>>,
    messages_txs: Vec<Sender<CrossShardMsg>>,
) -> thread::JoinHandle<()> {
    // create and start a new executor shard in a separate thread
    thread::Builder::new()
        .name(format!("partitioning-shard-{}", shard_id))
        .spawn(move || {
            let partitioning_shard =
                PartitioningShard::new(shard_id, control_rx, result_tx, message_rxs, messages_txs);
            partitioning_shard.start();
        })
        .unwrap()
}

#[cfg(test)]
mod tests {
    use crate::{
        sharded_block_partitioner::ShardedBlockPartitioner,
        test_utils::{
            create_non_conflicting_p2p_transaction, create_signed_p2p_transaction,
            generate_test_account, generate_test_account_for_address, TestAccount,
        },
        types::TransactionsChunk,
    };
    use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
    use move_core_types::account_address::AccountAddress;
    use rand::{rngs::OsRng, Rng};
    use std::collections::HashMap;

    fn verify_no_cross_shard_dependency(partitioned_txns: Vec<TransactionsChunk>) {
        for chunk in partitioned_txns {
            for txn in chunk.transactions {
                assert_eq!(txn.cross_shard_dependencies().len(), 0);
            }
        }
    }

    #[test]
    // Test that the partitioner works correctly for a single sender and multiple receivers.
    // In this case the expectation is that only the first shard will contain transactions and all
    // other shards will be empty.
    fn test_single_sender_txns() {
        let mut sender = generate_test_account();
        let mut receivers = Vec::new();
        let num_txns = 10;
        for _ in 0..num_txns {
            receivers.push(generate_test_account());
        }
        let transactions = create_signed_p2p_transaction(
            &mut sender,
            receivers.iter().collect::<Vec<&TestAccount>>(),
        );
        let partitioner = ShardedBlockPartitioner::new(4);
        let partitioned_txns = partitioner.partition(transactions.clone(), 1);
        assert_eq!(partitioned_txns.len(), 4);
        // The first shard should contain all the transactions
        assert_eq!(partitioned_txns[0].len(), num_txns);
        // The rest of the shards should be empty
        for txns in partitioned_txns.iter().take(4).skip(1) {
            assert_eq!(txns.len(), 0);
        }
        // Verify that the transactions are in the same order as the original transactions and cross shard
        // dependencies are empty.
        for (i, txn) in partitioned_txns[0].transactions.iter().enumerate() {
            assert_eq!(txn.txn(), &transactions[i]);
            assert_eq!(txn.cross_shard_dependencies().len(), 0);
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
        let partitioner = ShardedBlockPartitioner::new(num_shards);
        let partitioned_txns = partitioner.partition(transactions.clone(), 1);
        assert_eq!(partitioned_txns.len(), num_shards);
        // Verify that the transactions are in the same order as the original transactions and cross shard
        // dependencies are empty.
        let mut current_index = 0;
        for analyzed_txns in partitioned_txns.into_iter() {
            assert_eq!(analyzed_txns.len(), num_txns / num_shards);
            for txn in analyzed_txns.transactions.iter() {
                assert_eq!(txn.txn(), &transactions[current_index]);
                assert_eq!(txn.cross_shard_dependencies().len(), 0);
                current_index += 1;
            }
        }
    }

    #[test]
    fn test_same_sender_in_one_shard() {
        let num_shards = 3;
        let mut sender = generate_test_account();
        let mut txns_from_sender = Vec::new();
        for _ in 0..5 {
            txns_from_sender.push(
                create_signed_p2p_transaction(&mut sender, vec![&generate_test_account()])
                    .remove(0),
            );
        }
        let mut non_conflicting_transactions = Vec::new();
        for _ in 0..5 {
            non_conflicting_transactions.push(create_non_conflicting_p2p_transaction());
        }

        let mut transactions = Vec::new();
        let mut txn_from_sender_index = 0;
        let mut non_conflicting_txn_index = 0;
        transactions.push(non_conflicting_transactions[non_conflicting_txn_index].clone());
        non_conflicting_txn_index += 1;
        transactions.push(txns_from_sender[txn_from_sender_index].clone());
        txn_from_sender_index += 1;
        transactions.push(txns_from_sender[txn_from_sender_index].clone());
        txn_from_sender_index += 1;
        transactions.push(non_conflicting_transactions[non_conflicting_txn_index].clone());
        non_conflicting_txn_index += 1;
        transactions.push(txns_from_sender[txn_from_sender_index].clone());
        txn_from_sender_index += 1;
        transactions.push(txns_from_sender[txn_from_sender_index].clone());
        txn_from_sender_index += 1;
        transactions.push(non_conflicting_transactions[non_conflicting_txn_index].clone());
        transactions.push(txns_from_sender[txn_from_sender_index].clone());

        let partitioner = ShardedBlockPartitioner::new(num_shards);
        let partitioned_txns = partitioner.partition(transactions.clone(), 1);
        assert_eq!(partitioned_txns.len(), num_shards);
        assert_eq!(partitioned_txns[0].len(), 6);
        assert_eq!(partitioned_txns[1].len(), 2);
        assert_eq!(partitioned_txns[2].len(), 0);

        // verify that all transactions from the sender end up in shard 0
        for (index, txn) in txns_from_sender.iter().enumerate() {
            assert_eq!(partitioned_txns[0].transactions[index + 1].txn(), txn);
        }
        verify_no_cross_shard_dependency(partitioned_txns);
    }

    #[test]
    fn test_cross_shard_dependencies() {
        let num_shards = 3;
        let mut account1 = generate_test_account_for_address(AccountAddress::new([0; 32]));
        let mut account2 = generate_test_account_for_address(AccountAddress::new([1; 32]));
        let mut account3 = generate_test_account_for_address(AccountAddress::new([2; 32]));
        let mut account4 = generate_test_account_for_address(AccountAddress::new([4; 32]));
        let mut account5 = generate_test_account_for_address(AccountAddress::new([5; 32]));
        let account6 = generate_test_account_for_address(AccountAddress::new([6; 32]));

        let txn0 = create_signed_p2p_transaction(&mut account1, vec![&account2]).remove(0); // txn 0
        let txn1 = create_signed_p2p_transaction(&mut account1, vec![&account3]).remove(0); // txn 1
        let txn2 = create_signed_p2p_transaction(&mut account2, vec![&account4]).remove(0); // txn 2
                                                                                            // Should go in shard 1
        let txn3 = create_signed_p2p_transaction(&mut account3, vec![&account4]).remove(0); // txn 3
        let txn4 = create_signed_p2p_transaction(&mut account3, vec![&account1]).remove(0); // txn 4
        let txn5 = create_signed_p2p_transaction(&mut account3, vec![&account2]).remove(0); // txn 5
                                                                                            // Should go in shard 2
        let txn6 = create_signed_p2p_transaction(&mut account4, vec![&account1]).remove(0); // txn 6
        let txn7 = create_signed_p2p_transaction(&mut account4, vec![&account2]).remove(0); // txn 7
        let txn8 = create_signed_p2p_transaction(&mut account5, vec![&account6]).remove(0); // txn 8

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

        let partitioner = ShardedBlockPartitioner::new(num_shards);
        let partitioned_chunks = partitioner.partition(transactions, 1);
        assert_eq!(partitioned_chunks.len(), 2 * num_shards);

        // In first round of the partitioning, we should have txn0, txn1 and txn2 in shard 0 and
        // 0 in shard 1 and txn8 in shard 2
        assert_eq!(partitioned_chunks[0].len(), 3);
        assert_eq!(partitioned_chunks[1].len(), 0);
        assert_eq!(partitioned_chunks[2].len(), 1);

        assert_eq!(
            partitioned_chunks[0]
                .transactions_with_deps()
                .iter()
                .map(|x| x.txn.clone())
                .collect::<Vec<AnalyzedTransaction>>(),
            vec![txn0, txn1, txn2]
        );
        assert_eq!(
            partitioned_chunks[2]
                .transactions_with_deps()
                .iter()
                .map(|x| x.txn.clone())
                .collect::<Vec<AnalyzedTransaction>>(),
            vec![txn8]
        );

        // Rest of the transactions will be added in round 2 along with their dependencies
        assert_eq!(partitioned_chunks[3].len(), 0);
        assert_eq!(partitioned_chunks[4].len(), 3);
        assert_eq!(partitioned_chunks[5].len(), 2);

        assert_eq!(
            partitioned_chunks[4]
                .transactions_with_deps()
                .iter()
                .map(|x| x.txn.clone())
                .collect::<Vec<AnalyzedTransaction>>(),
            vec![txn3, txn4, txn5]
        );
        assert_eq!(
            partitioned_chunks[5]
                .transactions_with_deps()
                .iter()
                .map(|x| x.txn.clone())
                .collect::<Vec<AnalyzedTransaction>>(),
            vec![txn6, txn7]
        );

        // Verify transaction dependencies
        verify_no_cross_shard_dependency(vec![
            partitioned_chunks[0].clone(),
            partitioned_chunks[1].clone(),
            partitioned_chunks[2].clone(),
        ]);
        // txn3 depends on txn1 (index 1) and txn2 (index 2)
        assert!(partitioned_chunks[4].transactions_with_deps()[0]
            .cross_shard_dependencies
            .is_depends_on(1));
        assert!(partitioned_chunks[4].transactions_with_deps()[0]
            .cross_shard_dependencies
            .is_depends_on(2));

        // txn4 depends on txn1 (index 1)
        assert!(partitioned_chunks[4].transactions_with_deps()[1]
            .cross_shard_dependencies
            .is_depends_on(1));
        // txn5 depends on txn1 (index 1) and txn2 (index 2)
        assert!(partitioned_chunks[4].transactions_with_deps()[2]
            .cross_shard_dependencies
            .is_depends_on(1));
        assert!(partitioned_chunks[4].transactions_with_deps()[2]
            .cross_shard_dependencies
            .is_depends_on(2));
        // txn6 depends on txn3 (index 4) and txn4 (index 5)
        assert!(partitioned_chunks[5].transactions_with_deps()[0]
            .cross_shard_dependencies
            .is_depends_on(4));
        assert!(partitioned_chunks[5].transactions_with_deps()[0]
            .cross_shard_dependencies
            .is_depends_on(5));
        // txn7 depends on txn3 (index 4) and txn5 (index 5)
        assert!(partitioned_chunks[5].transactions_with_deps()[1]
            .cross_shard_dependencies
            .is_depends_on(4));
        assert!(partitioned_chunks[5].transactions_with_deps()[1]
            .cross_shard_dependencies
            .is_depends_on(6));
    }

    #[test]
    // Generates a bunch of random transactions and ensures that after the partitioning, there is
    // no conflict across shards.
    fn test_no_conflict_across_shards_in_first_round() {
        let mut rng = OsRng;
        let max_accounts = 500;
        let max_txns = 2000;
        let max_num_shards = 64;
        let num_accounts = rng.gen_range(1, max_accounts);
        let mut accounts = Vec::new();
        for _ in 0..num_accounts {
            accounts.push(generate_test_account());
        }
        let num_txns = rng.gen_range(1, max_txns);
        let mut transactions = Vec::new();
        let num_shards = rng.gen_range(1, max_num_shards);

        for _ in 0..num_txns {
            // randomly select a sender and receiver from accounts
            let sender_index = rng.gen_range(0, accounts.len());
            let mut sender = accounts.swap_remove(sender_index);
            let receiver_index = rng.gen_range(0, accounts.len());
            let receiver = accounts.get(receiver_index).unwrap();
            transactions.push(create_signed_p2p_transaction(&mut sender, vec![receiver]).remove(0));
            accounts.push(sender)
        }
        let partitioner = ShardedBlockPartitioner::new(num_shards);
        let partitioned_txns = partitioner.partition(transactions, 1);
        // Build a map of storage location to corresponding shards in first round
        // and ensure that no storage location is present in more than one shard.
        let mut storage_location_to_shard_map = HashMap::new();
        for (shard_id, txns) in partitioned_txns.iter().enumerate().take(num_shards) {
            for txn in txns.transactions_with_deps().iter() {
                let storage_locations = txn
                    .txn()
                    .read_set()
                    .iter()
                    .chain(txn.txn().write_set().iter());
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
