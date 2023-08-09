// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::sharded_block_partitioner::{
    counters::BLOCK_PARTITIONING_MISC_TIMERS_SECONDS,
    cross_shard_messages::CrossShardMsg,
    dependency_analysis::WriteSetWithTxnIndex,
    messages::{
        AddWithCrossShardDep, ControlMsg,
        ControlMsg::{AddCrossShardDepReq, DiscardCrossShardDepReq},
        DiscardCrossShardDep, PartitioningResp,
    },
    partitioning_shard::PartitioningShard,
};
use aptos_logger::{error, info};
use aptos_types::{
    block_executor::partitioner::{
        PartitionedTransactions, RoundId, ShardId, SubBlocksForShard, TxnIndex, GLOBAL_ROUND_ID,
        MAX_ALLOWED_PARTITIONING_ROUNDS,
    },
    transaction::analyzed_transaction::AnalyzedTransaction,
};
use counters::BLOCK_PARTITIONING_SECONDS;
use itertools::Itertools;
use std::{
    collections::HashMap,
    sync::{
        mpsc::{Receiver, Sender},
        Arc,
    },
    thread,
};

mod conflict_detector;
mod counters;
mod cross_shard_messages;
mod dependency_analysis;
mod dependent_edges;
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
///   2.2 Each shard creates a read-write set for all transactions in the chunk and broadcasts it to all other shards.
///              Shard 0                          Shard 1                           Shard 2
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///    |        Read-Write Set      |  |         Read-Write Set         |  |         Read-Write Set         |
///    |                            |  |                               |  |                               |
///    |   T1 {A, B}                |  |   T4 {D, E}                   |  |   T7 {G, H}                   |
///    |   T2 {B, C}                |  |   T5 {E, F}                   |  |   T8 {H, I}                   |
///    |   T3 {C, D}                |  |   T6 {F, G}                   |  |   T9 {I, J}                   |
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///   2.3 Each shard collects read-write sets from all other shards and discards transactions that have cross-shard dependencies.
///              Shard 0                          Shard 1                           Shard 2
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///    |        Discarded Txns      |  |         Discarded Txns         |  |         Discarded Txns         |
///    |                            |  |                               |  |                               |
///    |   - T3 (cross-shard dependency with T4) |  |   - T6 (cross-shard dependency with T7) |  | No discard |
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///   2.4 Each shard broadcasts the number of transactions that it plans to put in the current chunk.
///              Shard 0                          Shard 1                           Shard 2
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///    |          Chunk Count       |  |          Chunk Count          |  |          Chunk Count          |
///    |                            |  |                               |  |                               |
///    |   2                        |  |   2                           |  |      3                        |
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///   2.5 Each shard collects the number of transactions that all other shards plan to put in the current chunk and based
///      on that, it finalizes the absolute index offset of the current chunk. It uses this information to create a read-write set
///      index, which is a mapping of all the storage location to the maximum absolute index of the transaction that read/wrote to that location.
///             Shard 0                          Shard 1                           Shard 2
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///    |          Index Offset      |  |          Index Offset         |  |          Index Offset         |
///    |                            |  |                               |  |                               |
///    |   0                        |  |   2                           |  |   4                           |
///    +----------------------------+  +-------------------------------+  +-------------------------------+
///   2.6 It also uses the read-write set index mapping passed in previous iteration to add cross-shard dependencies to the transactions. This is
///     done by looking up the read-write set index for each storage location that a transaction reads/writes to and adding a cross-shard dependency
///   2.7 Returns two lists of transactions: one list of transactions that are discarded and another list of transactions that are kept.
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
///
pub struct ShardedBlockPartitioner {
    num_shards: usize,
    max_partitioning_rounds: RoundId,
    /// `cross_shard_dep_avoid_threshold` is the maximum fraction of transactions we try to avoid cross shard dependencies. Once we reach
    /// this fraction, we terminate early and add cross-shard dependencies to the remaining transactions.
    cross_shard_dep_avoid_threshold: f32,
    /// If true, the last round is partitioned and executed in a sharded manner. Otherwise, the last round is executed in global executor.
    partition_last_round: bool,
    control_txs: Vec<Sender<ControlMsg>>,
    result_rxs: Vec<Receiver<PartitioningResp>>,
    shard_threads: Vec<thread::JoinHandle<()>>,
}

impl ShardedBlockPartitioner {
    pub(crate) fn new(
        num_shards: usize,
        max_partitioning_rounds: RoundId,
        cross_shard_dep_avoid_threshold: f32,
        partition_last_round: bool,
    ) -> Self {
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
            max_partitioning_rounds,
            cross_shard_dep_avoid_threshold,
            partition_last_round,
            control_txs,
            result_rxs,
            shard_threads: shard_join_handles,
        }
    }

    // reorders the transactions so that transactions from the same sender always go to the same shard.
    // This places transactions from the same sender next to each other, which is not optimal for parallelism.
    // TODO(skedia): Improve this logic to shuffle senders
    fn partition_by_senders(
        &self,
        txns: Vec<AnalyzedTransaction>,
    ) -> Vec<Vec<AnalyzedTransaction>> {
        let approx_txns_per_shard = (txns.len() as f64 / self.num_shards as f64).ceil() as usize;
        let mut sender_to_txns = HashMap::new();
        let mut sender_order = Vec::new(); // Track sender ordering

        for txn in txns {
            let sender = txn.sender().unwrap();
            let entry = sender_to_txns.entry(sender).or_insert_with(Vec::new);
            entry.push(txn);
            if entry.len() == 1 {
                sender_order.push(sender); // Add sender to the order vector
            }
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
        Vec<SubBlocksForShard<AnalyzedTransaction>>,
        Vec<WriteSetWithTxnIndex>,
        Vec<Vec<AnalyzedTransaction>>,
    ) {
        let mut frozen_sub_blocks = Vec::new();
        let mut frozen_write_set_with_index = Vec::new();
        let mut rejected_txns_vec = Vec::new();
        for rx in &self.result_rxs {
            let PartitioningResp {
                frozen_sub_blocks: frozen_chunk,
                write_set_with_index,
                discarded_txns: rejected_txns,
            } = rx.recv().unwrap();
            frozen_sub_blocks.push(frozen_chunk);
            frozen_write_set_with_index.push(write_set_with_index);
            rejected_txns_vec.push(rejected_txns);
        }
        (
            frozen_sub_blocks,
            frozen_write_set_with_index,
            rejected_txns_vec,
        )
    }

    fn discard_txns_with_cross_shard_dependencies(
        &self,
        txns_to_partition: Vec<Vec<AnalyzedTransaction>>,
        current_round_start_index: TxnIndex,
        frozen_sub_blocks: Vec<SubBlocksForShard<AnalyzedTransaction>>,
        frozen_write_set_with_index: Arc<Vec<WriteSetWithTxnIndex>>,
        round_id: RoundId,
    ) -> (
        Vec<SubBlocksForShard<AnalyzedTransaction>>,
        Vec<WriteSetWithTxnIndex>,
        Vec<Vec<AnalyzedTransaction>>,
    ) {
        let partition_block_msgs = txns_to_partition
            .into_iter()
            .zip_eq(frozen_sub_blocks.into_iter())
            .map(|(txns, sub_blocks)| {
                DiscardCrossShardDepReq(DiscardCrossShardDep::new(
                    txns,
                    frozen_write_set_with_index.clone(),
                    current_round_start_index,
                    sub_blocks,
                    round_id,
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
        frozen_sub_blocks_by_shard: Vec<SubBlocksForShard<AnalyzedTransaction>>,
        frozen_write_set_with_index: Arc<Vec<WriteSetWithTxnIndex>>,
        round_id: RoundId,
    ) -> (
        Vec<SubBlocksForShard<AnalyzedTransaction>>,
        Vec<WriteSetWithTxnIndex>,
        Vec<Vec<AnalyzedTransaction>>,
    ) {
        let mut index_offset = index_offset;
        let partition_block_msgs = remaining_txns_vec
            .into_iter()
            .zip_eq(frozen_sub_blocks_by_shard.into_iter())
            .map(|(remaining_txns, frozen_sub_blocks)| {
                let remaining_txns_len = remaining_txns.len();
                let partitioning_msg = AddCrossShardDepReq(AddWithCrossShardDep::new(
                    remaining_txns,
                    index_offset,
                    frozen_write_set_with_index.clone(),
                    frozen_sub_blocks,
                    round_id,
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
    /// `max_partitioning_rounds` is the maximum number of partitioning rounds we allow.
    /// `cross_shard_dep_avoid_threshold` is the maximum fraction of transactions we try to avoid cross shard dependencies. Once we reach
    /// this fraction, we terminate early and add cross-shard dependencies to the remaining transactions.
    pub fn partition(&self, transactions: Vec<AnalyzedTransaction>) -> PartitionedTransactions {
        let _timer = BLOCK_PARTITIONING_SECONDS.start_timer();
        let total_txns = transactions.len();
        assert!(
            self.max_partitioning_rounds >= 1,
            "max_partitioning_rounds must be > 0"
        );
        assert!(
            self.max_partitioning_rounds <= MAX_ALLOWED_PARTITIONING_ROUNDS,
            "max_partitioning_rounds must be <= {}",
            MAX_ALLOWED_PARTITIONING_ROUNDS
        );
        if total_txns == 0 {
            return PartitionedTransactions::empty();
        }

        // First round, we filter all transactions with cross-shard dependencies
        let timer = BLOCK_PARTITIONING_MISC_TIMERS_SECONDS
            .with_label_values(&["partition_by_senders"])
            .start_timer();
        let mut txns_to_partition = self.partition_by_senders(transactions);
        let _duration = timer.stop_and_record();

        let timer = BLOCK_PARTITIONING_MISC_TIMERS_SECONDS
            .with_label_values(&["flatten_to_rounds"])
            .start_timer();
        let mut frozen_write_set_with_index = Arc::new(Vec::new());
        let mut current_round_start_index = 0;
        let mut frozen_sub_blocks: Vec<SubBlocksForShard<AnalyzedTransaction>> = vec![];
        for shard_id in 0..self.num_shards {
            frozen_sub_blocks.push(SubBlocksForShard::empty(shard_id))
        }
        let mut current_round = 0;
        for round_id in 0..self.max_partitioning_rounds - 1 {
            let _timer = BLOCK_PARTITIONING_MISC_TIMERS_SECONDS
                .with_label_values(&[format!("round_{round_id}").as_str()])
                .start_timer();
            let (
                updated_frozen_sub_blocks,
                current_frozen_rw_set_with_index_vec,
                discarded_txns_to_partition,
            ) = self.discard_txns_with_cross_shard_dependencies(
                txns_to_partition,
                current_round_start_index,
                frozen_sub_blocks,
                frozen_write_set_with_index.clone(),
                current_round,
            );
            // Current round start index is the sum of the number of transactions in the frozen sub-blocks
            current_round_start_index = updated_frozen_sub_blocks
                .iter()
                .map(|sub_blocks| sub_blocks.num_txns())
                .sum::<usize>();
            let mut prev_frozen_write_set_with_index =
                Arc::try_unwrap(frozen_write_set_with_index).unwrap();
            frozen_sub_blocks = updated_frozen_sub_blocks;
            prev_frozen_write_set_with_index.extend(current_frozen_rw_set_with_index_vec);
            frozen_write_set_with_index = Arc::new(prev_frozen_write_set_with_index);
            txns_to_partition = discarded_txns_to_partition;
            current_round += 1;
            let num_remaining_txns = txns_to_partition
                .iter()
                .map(|txns| txns.len())
                .sum::<usize>();
            // If there are no remaining transactions, we can stop partitioning
            if num_remaining_txns == 0 {
                return PartitionedTransactions::new(frozen_sub_blocks, vec![]);
            }

            if num_remaining_txns as f32 / total_txns as f32
                <= 1_f32 - self.cross_shard_dep_avoid_threshold
            {
                break;
            }
        }
        let _duration = timer.stop_and_record();
        let _timer = BLOCK_PARTITIONING_MISC_TIMERS_SECONDS
            .with_label_values(&["last_round"])
            .start_timer();
        // Assert rejected transactions are empty
        if self.partition_last_round {
            // We just add cross shard dependencies for remaining transactions.
            let (frozen_sub_blocks, _, rejected_txns) = self.add_cross_shard_dependencies(
                current_round_start_index,
                txns_to_partition,
                frozen_sub_blocks,
                frozen_write_set_with_index,
                current_round,
            );
            assert!(rejected_txns.iter().all(|txns| txns.is_empty()));
            PartitionedTransactions::new(frozen_sub_blocks, vec![])
        } else {
            // If we don't partition the last round, we just flatten all the transactions into one shard and send it to
            // the partitioner, so that we can add cross-shard dependencies.
            let remaining_txns = txns_to_partition.into_iter().flatten().collect::<Vec<_>>();

            let mut txns_to_partition = vec![];
            txns_to_partition.push(remaining_txns);
            for _ in 1..self.num_shards {
                txns_to_partition.push(vec![]);
            }
            let (mut frozen_sub_blocks, _, rejected_txns) = self.add_cross_shard_dependencies(
                current_round_start_index,
                txns_to_partition,
                frozen_sub_blocks,
                frozen_write_set_with_index,
                GLOBAL_ROUND_ID,
            );

            assert!(rejected_txns.iter().all(|txns| txns.is_empty()));
            let global_txns = frozen_sub_blocks[0]
                .remove_last_sub_block()
                .unwrap()
                .into_iter()
                .collect();
            // Make sure last sub block in all other shards are empty and remove them.
            frozen_sub_blocks[1..].iter_mut().for_each(|sub_blocks| {
                let last = sub_blocks.remove_last_sub_block().unwrap();
                assert!(last.is_empty());
            });

            PartitionedTransactions::new(frozen_sub_blocks, global_txns)
        }
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
    result_tx: Sender<PartitioningResp>,
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
        let partitioner = ShardedBlockPartitioner::new(4, 2, 0.9, true);
        let (sub_blocks, _) = partitioner.partition(transactions.clone()).into();
        assert_eq!(sub_blocks.len(), 4);
        // The first shard should contain all the transactions
        assert_eq!(sub_blocks[0].num_txns(), num_txns);
        // The rest of the shards should be empty
        for sub_blocks in sub_blocks.iter().take(4).skip(1) {
            assert_eq!(sub_blocks.num_txns(), 0);
        }
        // Verify that the transactions are in the same order as the original transactions and cross shard
        // dependencies are empty.
        for (i, txn) in sub_blocks[0].iter().enumerate() {
            assert_eq!(txn.txn().transaction(), transactions[i].transaction());
            assert_eq!(txn.cross_shard_dependencies().num_required_edges(), 0);
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
        let partitioner = ShardedBlockPartitioner::new(num_shards, 2, 0.9, true);
        let partitioned_txns = partitioner.partition(transactions.clone());
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

        let partitioner = ShardedBlockPartitioner::new(num_shards, 2, 0.9, true);
        let (sub_blocks, _) = partitioner.partition(transactions.clone()).into();
        assert_eq!(sub_blocks.len(), num_shards);
        assert_eq!(sub_blocks[0].num_sub_blocks(), 1);
        assert_eq!(sub_blocks[1].num_sub_blocks(), 1);
        assert_eq!(sub_blocks[2].num_sub_blocks(), 1);
        assert_eq!(sub_blocks[0].num_txns(), 6);
        assert_eq!(sub_blocks[1].num_txns(), 2);
        assert_eq!(sub_blocks[2].num_txns(), 0);

        // verify that all transactions from the sender end up in shard 0
        for (txn_from_sender, txn) in txns_from_sender.iter().zip(sub_blocks[0].iter().skip(1)) {
            assert_eq!(txn.txn().transaction(), txn_from_sender.transaction());
        }
        verify_no_cross_shard_dependency(
            sub_blocks
                .iter()
                .flat_map(|sub_blocks| sub_blocks.sub_block_iter())
                .cloned()
                .collect(),
        );
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

        let partitioner = ShardedBlockPartitioner::new(num_shards, 2, 0.9, true);
        let (sub_blocks, _) = partitioner.partition(transactions.clone()).into();

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

        let partitioner = ShardedBlockPartitioner::new(num_shards, 2, 0.9, true);
        let (partitioned_sub_blocks, _) = partitioner.partition(transactions).into();
        assert_eq!(partitioned_sub_blocks.len(), num_shards);

        // In first round of the partitioning, we should have txn0, txn1 and txn2 in shard 0 and
        // txn3, txn4, txn5 and txn8 in shard 1 and 0 in shard 2. Please note that txn8 is moved to
        // shard 1 because of sender based reordering.
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
            4
        );
        assert_eq!(
            partitioned_sub_blocks[2]
                .get_sub_block(0)
                .unwrap()
                .num_txns(),
            0
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
            vec![txn3, txn4, txn5, txn8]
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
            2
        );

        assert_eq!(
            partitioned_sub_blocks[2]
                .get_sub_block(1)
                .unwrap()
                .iter()
                .map(|x| x.txn.clone())
                .collect::<Vec<AnalyzedTransaction>>(),
            vec![txn6, txn7]
        );

        // Verify transaction dependencies
        verify_no_cross_shard_dependency(vec![
            partitioned_sub_blocks[0].get_sub_block(0).unwrap().clone(),
            partitioned_sub_blocks[1].get_sub_block(0).unwrap().clone(),
            partitioned_sub_blocks[2].get_sub_block(0).unwrap().clone(),
        ]);
        // Verify transaction depends_on and dependency list

        // txn6 (index 7) and txn7 (index 8) depends on txn8 (index 6)
        partitioned_sub_blocks[2]
            .get_sub_block(1)
            .unwrap()
            .iter()
            .for_each(|txn| {
                let required_deps = txn
                    .cross_shard_dependencies
                    .get_required_edge_for(ShardedTxnIndex::new(6, 1, 0))
                    .unwrap();
                // txn (6, 7) and 8 has conflict only on the coin store of account 7 as txn (6,7) are sending
                // from account 7 and txn 8 is receiving in account 7
                assert_eq!(required_deps.len(), 1);
                assert_eq!(
                    required_deps[0],
                    AnalyzedTransaction::coin_store_location(account7.account_address)
                );
            });

        // Verify the dependent edges, again the conflict is only on the coin store of account 7
        let required_deps = partitioned_sub_blocks[1]
            .get_sub_block(0)
            .unwrap()
            .transactions[3]
            .cross_shard_dependencies
            .get_dependent_edge_for(ShardedTxnIndex::new(7, 2, 1))
            .unwrap();
        assert_eq!(required_deps.len(), 1);
        assert_eq!(
            required_deps[0],
            AnalyzedTransaction::coin_store_location(account7.account_address)
        );

        let required_deps = partitioned_sub_blocks[1]
            .get_sub_block(0)
            .unwrap()
            .transactions[3]
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
        let max_partitioning_rounds = 8;
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
        let partitioner =
            ShardedBlockPartitioner::new(num_shards, max_partitioning_rounds, 0.9, true);
        let (sub_blocks, _) = partitioner.partition(transactions).into();
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
}
