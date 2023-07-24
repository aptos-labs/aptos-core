// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{sharded_block_partitioner::{
    cross_shard_messages::CrossShardMsg,
    dependency_analysis::WriteSetWithTxnIndex,
    messages::{
        AddWithCrossShardDep, ControlMsg,
        ControlMsg::DiscardCrossShardDepReq,
        DiscardCrossShardDep, PartitioningResp,
    },
    partitioning_shard::PartitioningShard,
}, BlockPartitioner};
use aptos_logger::{error, info};
use aptos_types::{
    block_executor::partitioner::{RoundId, ShardId, SubBlocksForShard, TxnIndex},
    transaction::{analyzed_transaction::AnalyzedTransaction, Transaction},
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
use std::env::VarError;
use std::time::Instant;
use aptos_types::block_executor::partitioner::{CrossShardDependencies, ShardedTxnIndex, SubBlock, TransactionWithDependencies};
use aptos_types::state_store::state_key::StateKey;
use aptos_types::transaction::analyzed_transaction::StorageLocation;
use crate::sharded_block_partitioner::counters::{ADD_EDGES_MISC_SECONDS, FLATTEN_TO_ROUNDS_MISC_SECONDS, SHARDED_PARTITIONER_MISC_SECONDS};
use crate::simple_partitioner::SimplePartitioner;

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
    control_txs: Vec<Sender<ControlMsg>>,
    result_rxs: Vec<Receiver<PartitioningResp>>,
    shard_threads: Vec<thread::JoinHandle<()>>,
    helper: SimplePartitioner,
}

pub static MAX_ALLOWED_PARTITIONING_ROUNDS: usize = 8;

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
            helper: SimplePartitioner::new(8),
        }
    }

    // reorders the transactions so that transactions from the same sender always go to the same shard.
    // This places transactions from the same sender next to each other, which is not optimal for parallelism.
    // TODO(skedia): Improve this logic to shuffle senders
    fn partition_by_senders(
        &self,
        txns: Vec<AnalyzedTransaction>,
    ) -> Vec<Vec<AnalyzedTransaction>> {
        let _timer = SHARDED_PARTITIONER_MISC_SECONDS.with_label_values(&["partition_by_senders"]).start_timer();
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
        Vec<Vec<AnalyzedTransaction>>,
        Vec<Vec<AnalyzedTransaction>>,
    ) {
        let mut accepted_txns_vec: Vec<Vec<AnalyzedTransaction>> = Vec::with_capacity(self.num_shards);
        let mut rejected_txns_vec: Vec<Vec<AnalyzedTransaction>> = Vec::with_capacity(self.num_shards);
        for rx in &self.result_rxs {
            let PartitioningResp {
                accepted_txns,
                discarded_txns,
            } = rx.recv().unwrap();
            accepted_txns_vec.push(accepted_txns);
            rejected_txns_vec.push(discarded_txns);
        }
        (accepted_txns_vec, rejected_txns_vec)
    }

    fn discard_txns_with_cross_shard_dependencies(
        &self,
        txns_to_partition: Vec<Vec<AnalyzedTransaction>>,
        round_id: RoundId,
    ) -> (
        Vec<Vec<AnalyzedTransaction>>,
        Vec<Vec<AnalyzedTransaction>>,
    ) {
        let partition_block_msgs = txns_to_partition
            .into_iter()
            .map(|transactions| {
                DiscardCrossShardDepReq(DiscardCrossShardDep {
                    transactions,
                    round_id,
                })
            })
            .collect();
        self.send_partition_msgs(partition_block_msgs);
        self.collect_partition_block_response()
    }

    /// We repeatedly partition chunks, discarding a bunch of transactions with cross-shard dependencies. The set of discarded
    /// transactions are used as candidate chunks in the next round. This process is repeated until num_partitioning_rounds.
    /// The remaining transactions are then added to the chunks with cross-shard dependencies.
    /// `max_partitioning_rounds` is the maximum number of partitioning rounds we allow.
    /// `cross_shard_dep_avoid_threshold` is the maximum fraction of transactions we try to avoid cross shard dependencies. Once we reach
    /// this fraction, we terminate early and add cross-shard dependencies to the remaining transactions.
    pub fn partition(
        &self,
        transactions: Vec<AnalyzedTransaction>,
        max_partitioning_rounds: RoundId,
        cross_shard_dep_avoid_threshold: f32,
    ) -> Vec<SubBlocksForShard<Transaction>> {
        let total_txns = transactions.len();
        assert!(
            max_partitioning_rounds >= 1,
            "max_partitioning_rounds must be > 0"
        );
        assert!(
            max_partitioning_rounds <= MAX_ALLOWED_PARTITIONING_ROUNDS,
            "max_partitioning_rounds must be <= {}",
            MAX_ALLOWED_PARTITIONING_ROUNDS
        );
        if total_txns == 0 {
            return vec![];
        }

        // First round, we filter all transactions with cross-shard dependencies
        let timer = SHARDED_PARTITIONER_MISC_SECONDS.with_label_values(&["partition_by_senders"]).start_timer();
        let mut txns_to_partition = self.partition_by_senders(transactions);
        let duration = timer.stop_and_record();
        let timer = SHARDED_PARTITIONER_MISC_SECONDS.with_label_values(&["flatten_to_rounds"]).start_timer();
        let matrix = self.flatten_to_rounds(max_partitioning_rounds, cross_shard_dep_avoid_threshold, txns_to_partition);
        let duration = timer.stop_and_record();
        let timer = SHARDED_PARTITIONER_MISC_SECONDS.with_label_values(&["add_edges"]).start_timer();
        let augmented_matrix = self.add_edges(matrix, None);
        let duration = timer.stop_and_record();
        augmented_matrix
    }

    pub fn flatten_to_rounds(
        &self,
        max_partitioning_rounds: RoundId,
        cross_shard_dep_avoid_threshold: f32,
        mut txns_by_shard: Vec<Vec<AnalyzedTransaction>>
    ) -> Vec<Vec<Vec<AnalyzedTransaction>>> {
        let total_txns: usize = txns_by_shard.iter().map(|txns_for_shard|txns_for_shard.len()).sum();
        let mut txn_matrix: Vec<Vec<Vec<AnalyzedTransaction>>> = Vec::new();
        let mut current_round = 0;
        for round_id in 0..max_partitioning_rounds - 1 {
            let timer = FLATTEN_TO_ROUNDS_MISC_SECONDS.with_label_values(&[format!("round_{round_id}").as_str()]).start_timer();
            let (
                accepted_txns,
                discarded_txns,
            ) = self.discard_txns_with_cross_shard_dependencies(
                txns_by_shard,
                current_round,
            );
            txn_matrix.push(accepted_txns);
            txns_by_shard = discarded_txns;
            current_round += 1;
            let num_remaining_txns: usize = txns_by_shard.iter().map(|txns| txns.len()).sum();
            timer.stop_and_record();

            if num_remaining_txns as f32 / total_txns as f32 <= 1 as f32 - cross_shard_dep_avoid_threshold {
                break;
            }
        }

        let _timer = FLATTEN_TO_ROUNDS_MISC_SECONDS.with_label_values(&[format!("last_round").as_str()]).start_timer();
        match std::env::var("SHARDED_PARTITIONER__MERGE_LAST_ROUND") {
            Ok(v) if v.as_str() == "1" => {
                info!("Let the the last shard handle the leftover.");
                let last_round_txns: Vec<AnalyzedTransaction> = txns_by_shard.into_iter().flatten().collect();
                txns_by_shard = vec![vec![]; self.num_shards];
                *txns_by_shard.get_mut(self.num_shards - 1).unwrap() = last_round_txns;
            }
            _ => {}
        }

        txn_matrix.push(txns_by_shard);
        txn_matrix
    }

    fn add_edges(
        &self,
        matrix: Vec<Vec<Vec<AnalyzedTransaction>>>,
        num_keys: Option<usize>,
    ) -> Vec<SubBlocksForShard<Transaction>> {
        let timer = ADD_EDGES_MISC_SECONDS.with_label_values(&["main"]).start_timer();
        let num_keys = num_keys.unwrap();
        let mut ret: Vec<SubBlocksForShard<Transaction>> = (0..self.num_shards).map(|shard_id| SubBlocksForShard { shard_id, sub_blocks: vec![] }).collect();
        let mut global_txn_counter: usize = 0;
        let mut global_owners_by_loc_id: Vec<Option<ShardedTxnIndex>> = vec![None; num_keys];// HashMap<StateKey, ShardedDTxnIndex>;
        for (round_id, row) in matrix.into_iter().enumerate() {
            for (shard_id, txns) in row.into_iter().enumerate() {
                let start_index_for_cur_sub_block = global_txn_counter;
                let mut twds_for_cur_sub_block: Vec<TransactionWithDependencies<Transaction>> = Vec::with_capacity(txns.len());
                let mut local_owners_by_loc_id: HashMap<usize, ShardedTxnIndex> = HashMap::new();
                for txn in txns {
                    let cur_sharded_txn_idx = ShardedTxnIndex {
                        txn_index: global_txn_counter,
                        shard_id,
                        round_id,
                    };
                    let mut cur_txn_csd = CrossShardDependencies::default();
                    for loc in txn.read_hints().iter() {
                        match &global_owners_by_loc_id[*loc.maybe_id_in_partition_session.as_ref().unwrap()] {
                            Some(owner) => {
                                ret.get_mut(owner.shard_id).unwrap()
                                    .get_sub_block_mut(owner.round_id).unwrap()
                                    .add_dependent_edge(owner.txn_index, cur_sharded_txn_idx, vec![loc.clone()]);
                                cur_txn_csd.add_required_edge(*owner, loc.clone());
                            },
                            None => {},
                        }
                    }

                    for loc in txn.write_hints().iter() {
                        let loc_id = *loc.maybe_id_in_partition_session.as_ref().unwrap();
                        local_owners_by_loc_id.insert(loc_id, cur_sharded_txn_idx);
                        match &global_owners_by_loc_id[loc_id] {
                            Some(owner) => {
                                ret.get_mut(owner.shard_id).unwrap()
                                    .get_sub_block_mut(owner.round_id).unwrap()
                                    .add_dependent_edge(owner.txn_index, cur_sharded_txn_idx, vec![loc.clone()]);
                                cur_txn_csd.add_required_edge(*owner, loc.clone());
                            },
                            None => {},
                        }
                    }
                    let twd = TransactionWithDependencies::new(txn.into_txn(), cur_txn_csd);
                    twds_for_cur_sub_block.push(twd);
                    global_txn_counter += 1;
                }

                let cur_sub_block = SubBlock::new(start_index_for_cur_sub_block, twds_for_cur_sub_block);
                ret.get_mut(shard_id).unwrap().add_sub_block(cur_sub_block);

                for (key, owner) in local_owners_by_loc_id {
                    global_owners_by_loc_id[key] = Some(owner);
                }
            }
        }
        let duration = timer.stop_and_record();
        let timer = ADD_EDGES_MISC_SECONDS.with_label_values(&["drop"]).start_timer();
        drop(global_owners_by_loc_id);
        let duration = timer.stop_and_record();
        ret
    }
}

impl BlockPartitioner for ShardedBlockPartitioner {
    fn partition(
        &self,
        transactions: Vec<AnalyzedTransaction>,
        num_executor_shards: usize,
    ) -> Vec<SubBlocksForShard<Transaction>> {
        let timer = SHARDED_PARTITIONER_MISC_SECONDS.with_label_values(&["env_stuff"]).start_timer();
        assert_eq!(self.num_shards, num_executor_shards);
        let max_partitioning_rounds = std::env::var("SHARDED_PARTITIONER__MAX_ROUNDS").ok()
            .map_or(None, |v|v.parse::<usize>().ok())
            .unwrap_or(4);
        let cross_shard_dep_avoid_threshold = std::env::var("SHARDED_PARTITIONER__CROSS_SHARD_DEP_AVOID_THRESHOLD").ok()
            .map_or(None, |v|v.parse::<usize>().ok())
            .unwrap_or(95) as f32 / 100.0;
        timer.stop_and_record();
        let ret = match std::env::var("SHARDED_PARTITIONER__INIT_WITH_SIMPLE_PARTITIONER") {
            Ok(v) if v.as_str() == "1" => {
                let timer = SHARDED_PARTITIONER_MISC_SECONDS.with_label_values(&["init_with_simple"]).start_timer();
                let (txns_by_shard_id, num_keys) = self.helper.partition(transactions, self.num_shards);
                let duration = timer.stop_and_record();
                println!("init_with_simple={duration:?}");
                let timer = SHARDED_PARTITIONER_MISC_SECONDS.with_label_values(&["flatten_to_rounds"]).start_timer();
                let matrix = self.flatten_to_rounds(max_partitioning_rounds, cross_shard_dep_avoid_threshold, txns_by_shard_id);
                let duration = timer.stop_and_record();
                println!("flatten_to_rounds={duration:?}");
                let timer = SHARDED_PARTITIONER_MISC_SECONDS.with_label_values(&["add_edges"]).start_timer();
                let ret = self.add_edges(matrix, Some(num_keys));
                let duration = timer.stop_and_record();
                println!("add_edges={duration:?}");
                ret
            }
            _ => {
                let ret = self.partition(transactions, max_partitioning_rounds, cross_shard_dep_avoid_threshold);
                ret
            }
        };
        ret
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

    fn verify_no_cross_shard_dependency(sub_blocks_for_shards: Vec<SubBlock<Transaction>>) {
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
        let transactions: Vec<AnalyzedTransaction> = create_signed_p2p_transaction(
            &mut sender,
            receivers.iter().collect::<Vec<&TestAccount>>(),
        ).into_iter().map(|t|t.into()).collect();
        let partitioner = ShardedBlockPartitioner::new(4);
        let sub_blocks = partitioner.partition(transactions.clone(), 2, 0.9);
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
            assert_eq!(txn.txn(), transactions[i].transaction());
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
            let txn = create_non_conflicting_p2p_transaction();
            let txn: AnalyzedTransaction = txn.into();
            transactions.push(txn)
        }
        let partitioner = ShardedBlockPartitioner::new(num_shards);
        let partitioned_txns = partitioner.partition(transactions.clone(), 2, 0.9);
        assert_eq!(partitioned_txns.len(), num_shards);
        // Verify that the transactions are in the same order as the original transactions and cross shard
        // dependencies are empty.
        let mut current_index = 0;
        for sub_blocks_for_shard in partitioned_txns.into_iter() {
            assert_eq!(sub_blocks_for_shard.num_txns(), num_txns / num_shards);
            for txn in sub_blocks_for_shard.iter() {
                assert_eq!(txn.txn(), transactions[current_index].transaction());
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
        let transactions: Vec<AnalyzedTransaction> = transactions.into_iter().map(|t|t.into()).collect();
        let partitioner = ShardedBlockPartitioner::new(num_shards);
        let sub_blocks = partitioner.partition(transactions.clone(), 2, 0.9);
        assert_eq!(sub_blocks.len(), num_shards);
        assert_eq!(sub_blocks[0].num_sub_blocks(), 1);
        assert_eq!(sub_blocks[1].num_sub_blocks(), 1);
        assert_eq!(sub_blocks[2].num_sub_blocks(), 1);
        assert_eq!(sub_blocks[0].num_txns(), 6);
        assert_eq!(sub_blocks[1].num_txns(), 2);
        assert_eq!(sub_blocks[2].num_txns(), 0);

        // verify that all transactions from the sender end up in shard 0
        for (txn_from_sender, txn) in txns_from_sender.iter().zip(sub_blocks[0].iter().skip(1)) {
            assert_eq!(txn.txn(), txn_from_sender);
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
        let transactions: Vec<AnalyzedTransaction> = transactions.into_iter().map(|t| t.into()).collect();
        let partitioner = ShardedBlockPartitioner::new(num_shards);
        let sub_blocks = partitioner.partition(transactions, 2, 0.9);

        let mut account_to_expected_seq_number: HashMap<AccountAddress, u64> = HashMap::new();
        SubBlocksForShard::flatten(sub_blocks)
            .iter()
            .for_each(|txn| {
                let (sender, seq_number) = get_account_seq_number(txn);
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

        let transactions: Vec<AnalyzedTransaction> = vec![
            txn0.clone().into(),
            txn1.clone().into(),
            txn2.clone().into(),
            txn3.clone().into(),
            txn4.clone().into(),
            txn5.clone().into(),
            txn6.clone().into(),
            txn7.clone().into(),
            txn8.clone().into(),
        ];

        let partitioner = ShardedBlockPartitioner::new(num_shards);
        let partitioned_sub_blocks = partitioner.partition(transactions, 2, 0.9);
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
                .collect::<Vec<Transaction>>(),
            vec![txn0, txn1, txn2]
        );
        assert_eq!(
            partitioned_sub_blocks[1]
                .get_sub_block(0)
                .unwrap()
                .iter()
                .map(|x| x.txn.clone())
                .collect::<Vec<Transaction>>(),
            vec![
                txn3,
                txn4,
                txn5,
                txn8,
            ]
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
                .collect::<Vec<Transaction>>(),
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
            let analyzed_txn: AnalyzedTransaction = create_signed_p2p_transaction(&mut sender, vec![receiver]).remove(0).into();
            txns_by_hash.insert(analyzed_txn.transaction().hash(), analyzed_txn.clone());
            transactions.push(analyzed_txn);
            accounts.push(sender)
        }
        let partitioner = ShardedBlockPartitioner::new(num_shards);
        let partitioned_txns = partitioner.partition(transactions, max_partitioning_rounds, 0.9);
        // Build a map of storage location to corresponding shards in first round
        // and ensure that no storage location is present in more than one shard.
        let num_partitioning_rounds = partitioned_txns[0].num_sub_blocks() - 1;
        for round in 0..num_partitioning_rounds {
            let mut storage_location_to_shard_map = HashMap::new();
            for (shard_id, sub_blocks_for_shard) in partitioned_txns.iter().enumerate() {
                let sub_block_for_round = sub_blocks_for_shard.get_sub_block(round).unwrap();
                for txn in sub_block_for_round.iter() {
                    let analyzed_txn = txns_by_hash.get(&txn.txn.hash()).unwrap();
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
