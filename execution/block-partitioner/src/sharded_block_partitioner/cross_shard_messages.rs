// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender};
use crate::sharded_block_partitioner::cross_shard_messages::CrossShardMsg::CrossShardDependentEdgesMsg;
use crate::sharded_block_partitioner::dependency_analysis::{RWSet, WriteSetWithTxnIndex};
use crate::types::{ShardId, TxnIndex};

#[derive(Clone, Debug)]
pub enum CrossShardMsg {
    WriteSetWithTxnIndexMsg(WriteSetWithTxnIndex),
    RWSetMsg(RWSet),
    // Number of accepted transactions in the shard for the current round.
    AcceptedTxnsMsg(usize),
    CrossShardDependentEdgesMsg(Vec<CrossShardDependentEdges>),
}

#[derive(Clone, Debug, Default)]
pub struct CrossShardDependentEdges {
    pub source_txn_index: TxnIndex,
    pub dependent_txn_indices: HashSet<TxnIndex>,
}

impl CrossShardDependentEdges {
    pub fn new(source_txn_index: TxnIndex, dependent_txn_indices: HashSet<TxnIndex>) -> Self {
        Self {
            source_txn_index,
            dependent_txn_indices,
        }
    }
}

pub struct CrossShardClient {
    shard_id: ShardId,
    message_rxs: Vec<Receiver<CrossShardMsg>>,
    message_txs: Vec<Sender<CrossShardMsg>>,
}

impl CrossShardClient {

    pub fn new(
        shard_id: ShardId,
        message_rxs: Vec<Receiver<CrossShardMsg>>,
        message_txs: Vec<Sender<CrossShardMsg>>,
    ) -> Self {
        Self {
            shard_id,
            message_rxs,
            message_txs,
        }
    }

    fn broadcast_and_collect<T, F, G>(&self, f: F, g: G) -> Vec<T>
        where
            F: Fn() -> CrossShardMsg,
            G: Fn(CrossShardMsg) -> Option<T>,
            T: Default + Clone,
    {
        let num_shards = self.message_txs.len();
        let mut vec = vec![T::default(); num_shards];

        for i in 0..num_shards {
            if i != self.shard_id {
                self.message_txs[i].send(f()).unwrap();
            }
        }

        for (i, msg_rx) in self.message_rxs.iter().enumerate() {
            if i == self.shard_id {
                continue;
            }
            let msg = msg_rx.recv().unwrap();
            vec[i] = g(msg).expect("Unexpected message");
        }
        vec
    }

    pub fn broadcast_and_collect_rw_set(&self, rw_set: RWSet) -> Vec<RWSet> {
        self.broadcast_and_collect(
            || CrossShardMsg::RWSetMsg(rw_set.clone()),
            |msg| match msg {
                CrossShardMsg::RWSetMsg(rw_set) => Some(rw_set),
                _ => None,
            },
        )
    }

    pub fn broadcast_and_collect_write_set_with_index(
        &self,
        rw_set_with_index: WriteSetWithTxnIndex,
    ) -> Vec<WriteSetWithTxnIndex> {
        self.broadcast_and_collect(
            || CrossShardMsg::WriteSetWithTxnIndexMsg(rw_set_with_index.clone()),
            |msg| match msg {
                CrossShardMsg::WriteSetWithTxnIndexMsg(rw_set_with_index) => {
                    Some(rw_set_with_index)
                },
                _ => None,
            },
        )
    }

    pub fn broadcast_and_collect_num_accepted_txns(&self, num_accepted_txns: usize) -> Vec<usize> {
        self.broadcast_and_collect(
            || CrossShardMsg::AcceptedTxnsMsg(num_accepted_txns),
            |msg| match msg {
                CrossShardMsg::AcceptedTxnsMsg(num_accepted_txns) => Some(num_accepted_txns),
                _ => None,
            },
        )
    }


    pub fn broadcast_and_collect_dependent_edges(&self, dependent_edges: Vec<Vec<CrossShardDependentEdges>>) -> Vec<Vec<CrossShardDependentEdges>> {
        let num_shards = self.message_txs.len();

        for (shard_id, dependent_edges) in dependent_edges.into_iter().enumerate() {
            if shard_id != self.shard_id {
                self.message_txs[shard_id].send(CrossShardDependentEdgesMsg(dependent_edges)).unwrap();
            }
        }

        let mut cross_shard_dependent_edges = vec![vec![]; num_shards];

        for (i, msg_rx) in self.message_rxs.iter().enumerate() {
            if i == self.shard_id {
                continue;
            }
            let msg = msg_rx.recv().unwrap();
            match msg {
                CrossShardDependentEdgesMsg(dependent_edges) => {
                    cross_shard_dependent_edges[i] = dependent_edges;
                }
                _ => panic!("Unexpected message")
            }
        }

        cross_shard_dependent_edges
    }

}
