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
use aptos_logger::{info, trace};
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use aptos_types::block_executor::partitioner::{ShardId, SubBlock, TransactionWithDependencies};
use aptos_types::transaction::Transaction;
use std::sync::{
    mpsc::{Receiver, Sender},
    Arc,
};
use std::time::Instant;

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
        let (accepted_txns, discarded_txns) = conflict_detector
            .discard_txns_with_cross_shard_deps(
                transactions,
                &cross_shard_rw_set,
            );

        // send the result back to the controller
        self.result_tx
            .send(PartitioningResp{ accepted_txns, discarded_txns })
            .unwrap();
    }

    pub fn start(&self) {
        loop {
            let command = self.control_rx.recv().unwrap();
            match command {
                ControlMsg::DiscardCrossShardDepReq(msg) => {
                    self.discard_txns_with_cross_shard_deps(msg);
                },
                ControlMsg::Stop => {
                    break;
                },
            }
        }
        trace!("Shard {} is shutting down", self.shard_id);
    }
}
