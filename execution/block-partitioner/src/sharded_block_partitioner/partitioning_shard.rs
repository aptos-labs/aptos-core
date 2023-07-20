// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::sharded_block_partitioner::{
    conflict_detector::CrossShardConflictDetector,
    counters::NUM_PARTITIONED_TXNS,
    cross_shard_messages::{CrossShardClient, CrossShardClientInterface, CrossShardMsg},
    dependency_analysis::{RWSet, WriteSetWithTxnIndex},
    dependent_edges::DependentEdgeCreator,
    messages::{AddWithCrossShardDep, ControlMsg, DiscardCrossShardDep, PartitioningResp},
};
use aptos_logger::trace;
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use aptos_types::block_executor::partitioner::{ShardId, SubBlock, TransactionWithDependencies};
use aptos_types::transaction::Transaction;
use std::sync::{
    mpsc::{Receiver, Sender},
    Arc,
};

pub struct PartitioningShard {
    num_shards: usize,
    shard_id: ShardId,
    control_rx: Receiver<ControlMsg>,
    result_tx: Sender<PartitioningResp>,
    cross_shard_client: Arc<CrossShardClient>,
}

impl PartitioningShard {
    pub fn new(
        shard_id: ShardId,
        control_rx: Receiver<ControlMsg>,
        result_tx: Sender<PartitioningResp>,
        message_rxs: Vec<Receiver<CrossShardMsg>>,
        message_txs: Vec<Sender<CrossShardMsg>>,
    ) -> Self {
        let num_shards = message_txs.len();
        let cross_shard_client =
            Arc::new(CrossShardClient::new(shard_id, message_rxs, message_txs));
        Self {
            num_shards,
            shard_id,
            control_rx,
            result_tx,
            cross_shard_client,
        }
    }

    fn discard_txns_with_cross_shard_deps(&self, partition_msg: DiscardCrossShardDep) {
        let DiscardCrossShardDep {
            transactions,
            prev_rounds_write_set_with_index,
            current_round_start_index,
            frozen_sub_blocks,
            round_id,
        } = partition_msg;
        let mut conflict_detector =
            CrossShardConflictDetector::new(self.shard_id, self.num_shards, round_id);
        // If transaction filtering is allowed, we need to prepare the dependency analysis and broadcast it to other shards
        // Based on the dependency analysis received from other shards, we will reject transactions that are conflicting with
        // transactions in other shards
        let read_write_set = RWSet::new(&transactions);
        let cross_shard_rw_set = self
            .cross_shard_client
            .broadcast_and_collect_rw_set(read_write_set);
        let (accepted_txns, accepted_cross_shard_dependencies, rejected_txns) = conflict_detector
            .discard_txns_with_cross_shard_deps(
                transactions,
                &cross_shard_rw_set,
                prev_rounds_write_set_with_index,
            );

        // Broadcast and collect the stats around number of accepted and rejected transactions from other shards
        // this will be used to determine the absolute index of accepted transactions in this shard.
        let accepted_txns_vec = self
            .cross_shard_client
            .broadcast_and_collect_num_accepted_txns(accepted_txns.len());
        // Calculate the absolute index of accepted transactions in this shard, which is the sum of all accepted transactions
        // from other shards whose shard id is smaller than the current shard id and the current round start index
        let num_accepted_txns = accepted_txns_vec.iter().take(self.shard_id).sum::<usize>();
        let index_offset = current_round_start_index + num_accepted_txns;

        // Now that we have finalized the global transaction index, we can add the dependent txn edges.
        let mut dependent_edge_creator = DependentEdgeCreator::new(
            self.shard_id,
            self.cross_shard_client.clone(),
            frozen_sub_blocks,
            self.num_shards,
            round_id,
        );
        dependent_edge_creator
            .create_dependent_edges(&accepted_cross_shard_dependencies, index_offset);

        // Calculate the RWSetWithTxnIndex for the accepted transactions
        let current_rw_set_with_index = WriteSetWithTxnIndex::new(&accepted_txns, index_offset);

        let accepted_txns_with_dependencies = accepted_txns
            .into_iter()
            .zip(accepted_cross_shard_dependencies.into_iter())
            .map(|(txn, dependencies)| {
                TransactionWithDependencies::new(txn.into_txn(), dependencies)
            })
            .collect::<Vec<TransactionWithDependencies<Transaction>>>();

        let mut frozen_sub_blocks = dependent_edge_creator.into_frozen_sub_blocks();
        NUM_PARTITIONED_TXNS
            .with_label_values(&[&self.shard_id.to_string(), &round_id.to_string()])
            .set(accepted_txns_with_dependencies.len() as i64);
        let current_frozen_sub_block = SubBlock::new(index_offset, accepted_txns_with_dependencies);
        frozen_sub_blocks.add_sub_block(current_frozen_sub_block);
        // send the result back to the controller
        self.result_tx
            .send(PartitioningResp::new(
                frozen_sub_blocks,
                current_rw_set_with_index,
                rejected_txns,
            ))
            .unwrap();
    }

    fn add_txns_with_cross_shard_deps(&self, partition_msg: AddWithCrossShardDep) {
        let AddWithCrossShardDep {
            transactions,
            index_offset,
            // The frozen dependencies in previous chunks.
            prev_rounds_write_set_with_index,
            mut frozen_sub_blocks,
            round_id,
        } = partition_msg;
        let conflict_detector =
            CrossShardConflictDetector::new(self.shard_id, self.num_shards, round_id);

        // Since txn filtering is not allowed, we can create the RW set with maximum txn
        // index with the index offset passed.
        NUM_PARTITIONED_TXNS
            .with_label_values(&[&self.shard_id.to_string(), &round_id.to_string()])
            .set(transactions.len() as i64);
        let write_set_with_index_for_shard = WriteSetWithTxnIndex::new(&transactions, index_offset);

        let current_round_rw_set_with_index = self
            .cross_shard_client
            .broadcast_and_collect_write_set_with_index(write_set_with_index_for_shard.clone());
        let (current_frozen_sub_block, current_cross_shard_deps) = conflict_detector
            .add_deps_for_frozen_sub_block(
                transactions,
                Arc::new(current_round_rw_set_with_index),
                prev_rounds_write_set_with_index,
                index_offset,
            );

        frozen_sub_blocks.add_sub_block(current_frozen_sub_block);

        let mut dependent_edge_creator = DependentEdgeCreator::new(
            self.shard_id,
            self.cross_shard_client.clone(),
            frozen_sub_blocks,
            self.num_shards,
            round_id,
        );
        dependent_edge_creator.create_dependent_edges(&current_cross_shard_deps, index_offset);

        self.result_tx
            .send(PartitioningResp::new(
                dependent_edge_creator.into_frozen_sub_blocks(),
                write_set_with_index_for_shard,
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
