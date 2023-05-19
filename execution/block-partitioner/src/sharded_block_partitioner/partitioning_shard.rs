// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    sharded_block_partitioner::{
        conflict_detector::CrossShardConflictDetector,
        dependency_analysis::{RWSet, RWSetWithTxnIndex},
        messages::{
            AddTxnsWithCrossShardDep, ControlMsg, CrossShardMsg, DiscardTxnsWithCrossShardDep,
            PartitioningBlockResponse,
        },
    },
    types::{ShardId, TransactionWithDependencies, TransactionsChunk},
};
use aptos_logger::trace;
use std::sync::{
    mpsc::{Receiver, Sender},
    Arc,
};

pub struct PartitioningShard {
    shard_id: ShardId,
    control_rx: Receiver<ControlMsg>,
    result_tx: Sender<PartitioningBlockResponse>,
    message_rxs: Vec<Receiver<CrossShardMsg>>,
    message_txs: Vec<Sender<CrossShardMsg>>,
}

impl PartitioningShard {
    pub fn new(
        shard_id: ShardId,
        control_rx: Receiver<ControlMsg>,
        result_tx: Sender<PartitioningBlockResponse>,
        message_rxs: Vec<Receiver<CrossShardMsg>>,
        messages_txs: Vec<Sender<CrossShardMsg>>,
    ) -> Self {
        Self {
            shard_id,
            control_rx,
            result_tx,
            message_rxs,
            message_txs: messages_txs,
        }
    }

    fn broadcast_rw_set(&self, rw_set: RWSet) {
        let num_shards = self.message_txs.len();
        for i in 0..num_shards {
            if i != self.shard_id {
                self.message_txs[i]
                    .send(CrossShardMsg::RWSetMsg(rw_set.clone()))
                    .unwrap();
            }
        }
    }

    fn collect_rw_set(&self) -> Vec<RWSet> {
        let mut rw_set_vec = vec![RWSet::default(); self.message_txs.len()];
        for (i, msg_rx) in self.message_rxs.iter().enumerate() {
            if i == self.shard_id {
                continue;
            }

            let msg = msg_rx.recv().unwrap();
            match msg {
                CrossShardMsg::RWSetMsg(rw_set) => {
                    rw_set_vec[i] = rw_set;
                },
                _ => panic!("Unexpected message"),
            }
        }
        rw_set_vec
    }

    fn broadcast_rw_set_with_index(&self, rw_set_with_index: RWSetWithTxnIndex) {
        let num_shards = self.message_txs.len();
        for i in 0..num_shards {
            if i != self.shard_id {
                self.message_txs[i]
                    .send(CrossShardMsg::RWSetWithTxnIndexMsg(
                        rw_set_with_index.clone(),
                    ))
                    .unwrap();
            }
        }
    }

    fn collect_rw_set_with_index(&self) -> Vec<RWSetWithTxnIndex> {
        let mut rw_set_with_index_vec = vec![RWSetWithTxnIndex::default(); self.message_txs.len()];
        for (i, msg_rx) in self.message_rxs.iter().enumerate() {
            if i == self.shard_id {
                continue;
            }
            let msg = msg_rx.recv().unwrap();
            match msg {
                CrossShardMsg::RWSetWithTxnIndexMsg(rw_set_with_index) => {
                    rw_set_with_index_vec[i] = rw_set_with_index;
                },
                _ => panic!("Unexpected message"),
            }
        }
        rw_set_with_index_vec
    }

    fn broadcast_num_accepted_txns(&self, num_accepted_txns: usize) {
        let num_shards = self.message_txs.len();
        for i in 0..num_shards {
            if i != self.shard_id {
                self.message_txs[i]
                    .send(CrossShardMsg::AcceptedTxnsMsg(num_accepted_txns))
                    .unwrap();
            }
        }
    }

    fn collect_num_accepted_txns(&self) -> Vec<usize> {
        let mut accepted_txns_vec = vec![0; self.message_txs.len()];
        for (i, msg_rx) in self.message_rxs.iter().enumerate() {
            if i == self.shard_id {
                continue;
            }
            let msg = msg_rx.recv().unwrap();
            match msg {
                CrossShardMsg::AcceptedTxnsMsg(num_accepted_txns) => {
                    accepted_txns_vec[i] = num_accepted_txns;
                },
                _ => panic!("Unexpected message"),
            }
        }
        accepted_txns_vec
    }

    fn discard_txns_with_cross_shard_deps(&self, partition_msg: DiscardTxnsWithCrossShardDep) {
        let DiscardTxnsWithCrossShardDep {
            transactions,
            prev_rounds_rw_set_with_index,
            prev_rounds_frozen_chunks,
        } = partition_msg;
        let num_shards = self.message_txs.len();
        let mut conflict_detector = CrossShardConflictDetector::new(self.shard_id, num_shards);
        // If transaction filtering is allowed, we need to prepare the dependency analysis and broadcast it to other shards
        // Based on the dependency analysis received from other shards, we will reject transactions that are conflicting with
        // transactions in other shards
        let read_write_set = RWSet::new(&transactions);
        self.broadcast_rw_set(read_write_set);
        let cross_shard_rw_set = self.collect_rw_set();
        let (accepted_txns, accepted_cross_shard_dependencies, rejected_txns) = conflict_detector
            .discard_txns_with_cross_shard_deps(
                transactions,
                &cross_shard_rw_set,
                prev_rounds_rw_set_with_index,
            );
        // Broadcast and collect the stats around number of accepted and rejected transactions from other shards
        // this will be used to determine the absolute index of accepted transactions in this shard.
        self.broadcast_num_accepted_txns(accepted_txns.len());
        let accepted_txns_vec = self.collect_num_accepted_txns();
        // Calculate the absolute index of accepted transactions in this shard, which is the sum of all accepted transactions
        // from other shards whose shard id is smaller than the current shard id and the number of accepted transactions in the
        // previous rounds
        let mut index_offset = prev_rounds_frozen_chunks
            .iter()
            .map(|chunk| chunk.len())
            .sum::<usize>();
        for num_accepted_txns in accepted_txns_vec.iter().take(self.shard_id) {
            index_offset += num_accepted_txns;
        }

        // Calculate the RWSetWithTxnIndex for the accepted transactions
        let current_rw_set_with_index = RWSetWithTxnIndex::new(&accepted_txns, index_offset);

        let accepted_txns_with_dependencies = accepted_txns
            .into_iter()
            .zip(accepted_cross_shard_dependencies.into_iter())
            .map(|(txn, dependencies)| TransactionWithDependencies::new(txn, dependencies))
            .collect::<Vec<TransactionWithDependencies>>();

        let frozen_chunk = TransactionsChunk::new(index_offset, accepted_txns_with_dependencies);
        drop(prev_rounds_frozen_chunks);
        // send the result back to the controller
        self.result_tx
            .send(PartitioningBlockResponse::new(
                frozen_chunk,
                current_rw_set_with_index,
                rejected_txns,
            ))
            .unwrap();
    }

    fn add_txns_with_cross_shard_deps(&self, partition_msg: AddTxnsWithCrossShardDep) {
        let AddTxnsWithCrossShardDep {
            transactions,
            index_offset,
            prev_rounds_frozen_chunks,
            // The frozen dependencies in previous chunks.
            prev_rounds_rw_set_with_index,
        } = partition_msg;
        let num_shards = self.message_txs.len();
        let conflict_detector = CrossShardConflictDetector::new(self.shard_id, num_shards);

        // Since txn filtering is not allowed, we can create the RW set with maximum txn
        // index with the index offset passed.
        let rw_set_with_index_for_shard = RWSetWithTxnIndex::new(&transactions, index_offset);

        self.broadcast_rw_set_with_index(rw_set_with_index_for_shard.clone());
        let current_round_rw_set_with_index = self.collect_rw_set_with_index();
        let frozen_chunk = conflict_detector.get_frozen_chunk(
            transactions,
            Arc::new(current_round_rw_set_with_index),
            prev_rounds_rw_set_with_index,
            index_offset,
        );

        drop(prev_rounds_frozen_chunks);

        self.result_tx
            .send(PartitioningBlockResponse::new(
                frozen_chunk,
                rw_set_with_index_for_shard,
                vec![],
            ))
            .unwrap();
    }

    pub fn start(&self) {
        loop {
            let command = self.control_rx.recv().unwrap();
            match command {
                ControlMsg::DiscardCrossShardDepReq(msg) => {
                    self.discard_txns_with_cross_shard_deps(msg);
                },
                ControlMsg::AddCrossShardDepReq(msg) => {
                    self.add_txns_with_cross_shard_deps(msg);
                },
                ControlMsg::Stop => {
                    break;
                },
            }
        }
        trace!("Shard {} is shutting down", self.shard_id);
    }
}
