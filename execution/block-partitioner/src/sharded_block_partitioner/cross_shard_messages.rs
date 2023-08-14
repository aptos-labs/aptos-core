// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::sharded_block_partitioner::{
    cross_shard_messages::CrossShardMsg::CrossShardDependentEdgesMsg,
    dependency_analysis::{RWSet, WriteSetWithTxnIndex},
};
use aptos_types::block_executor::partitioner::{CrossShardEdges, ShardId, TxnIndex};
use async_trait::async_trait;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

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
    pub dependent_edges: CrossShardEdges,
}

impl CrossShardDependentEdges {
    pub fn new(source_txn_index: TxnIndex, dependent_edges: CrossShardEdges) -> Self {
        Self {
            source_txn_index,
            dependent_edges,
        }
    }
}

// Define the interface for CrossShardClient
#[async_trait]
pub trait CrossShardClientInterface {
    async fn broadcast_and_collect_rw_set(&mut self, rw_set: RWSet) -> Vec<RWSet>;
    async fn broadcast_and_collect_write_set_with_index(
        &mut self,
        rw_set_with_index: WriteSetWithTxnIndex,
    ) -> Vec<WriteSetWithTxnIndex>;
    async fn broadcast_and_collect_num_accepted_txns(
        &mut self,
        num_accepted_txns: usize,
    ) -> Vec<usize>;
    async fn broadcast_and_collect_dependent_edges(
        &mut self,
        dependent_edges: Vec<Vec<CrossShardDependentEdges>>,
    ) -> Vec<Vec<CrossShardDependentEdges>>;
}

pub struct CrossShardClient {
    shard_id: ShardId,
    message_rxs: Vec<UnboundedReceiver<CrossShardMsg>>,
    message_txs: Vec<UnboundedSender<CrossShardMsg>>,
}

impl CrossShardClient {
    pub fn new(
        shard_id: ShardId,
        message_rxs: Vec<UnboundedReceiver<CrossShardMsg>>,
        message_txs: Vec<UnboundedSender<CrossShardMsg>>,
    ) -> Self {
        Self {
            shard_id,
            message_rxs,
            message_txs,
        }
    }

    async fn broadcast_and_collect<T, F, G>(&mut self, f: F, g: G) -> Vec<T>
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

        for (i, msg_rx) in self.message_rxs.iter_mut().enumerate() {
            if i == self.shard_id {
                continue;
            }
            let msg = msg_rx.recv().await.unwrap();
            vec[i] = g(msg).expect("Unexpected message");
        }
        vec
    }
}

#[async_trait]
impl CrossShardClientInterface for CrossShardClient {
    async fn broadcast_and_collect_rw_set(&mut self, rw_set: RWSet) -> Vec<RWSet> {
        self.broadcast_and_collect(
            || CrossShardMsg::RWSetMsg(rw_set.clone()),
            |msg| match msg {
                CrossShardMsg::RWSetMsg(rw_set) => Some(rw_set),
                _ => None,
            },
        )
        .await
    }

    async fn broadcast_and_collect_write_set_with_index(
        &mut self,
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
        .await
    }

    async fn broadcast_and_collect_num_accepted_txns(
        &mut self,
        num_accepted_txns: usize,
    ) -> Vec<usize> {
        self.broadcast_and_collect(
            || CrossShardMsg::AcceptedTxnsMsg(num_accepted_txns),
            |msg| match msg {
                CrossShardMsg::AcceptedTxnsMsg(num_accepted_txns) => Some(num_accepted_txns),
                _ => None,
            },
        )
        .await
    }

    async fn broadcast_and_collect_dependent_edges(
        &mut self,
        dependent_edges: Vec<Vec<CrossShardDependentEdges>>,
    ) -> Vec<Vec<CrossShardDependentEdges>> {
        let num_shards = self.message_txs.len();

        for (shard_id, dependent_edges) in dependent_edges.into_iter().enumerate() {
            self.message_txs[shard_id]
                .send(CrossShardDependentEdgesMsg(dependent_edges))
                .unwrap();
        }

        let mut cross_shard_dependent_edges = vec![vec![]; num_shards];

        for (i, msg_rx) in self.message_rxs.iter_mut().enumerate() {
            let msg = msg_rx.recv().await.unwrap();
            match msg {
                CrossShardDependentEdgesMsg(dependent_edges) => {
                    cross_shard_dependent_edges[i] = dependent_edges;
                },
                _ => panic!("Unexpected message"),
            }
        }

        cross_shard_dependent_edges
    }
}

// Create a mock implementation of CrossShardClientInterface for testing
#[cfg(test)]
pub struct MockCrossShardClient {
    pub rw_set_results: Vec<RWSet>,
    pub write_set_with_index_results: Vec<WriteSetWithTxnIndex>,
    pub num_accepted_txns_results: Vec<usize>,
    pub dependent_edges_results: Vec<Vec<CrossShardDependentEdges>>,
}

// Mock CrossShardClient used for testing purposes
#[cfg(test)]
impl CrossShardClientInterface for MockCrossShardClient {
    fn broadcast_and_collect_rw_set(&self, _rw_set: RWSet) -> Vec<RWSet> {
        self.rw_set_results.clone()
    }

    fn broadcast_and_collect_write_set_with_index(
        &self,
        _rw_set_with_index: WriteSetWithTxnIndex,
    ) -> Vec<WriteSetWithTxnIndex> {
        self.write_set_with_index_results.clone()
    }

    fn broadcast_and_collect_num_accepted_txns(&self, _num_accepted_txns: usize) -> Vec<usize> {
        self.num_accepted_txns_results.clone()
    }

    fn broadcast_and_collect_dependent_edges(
        &self,
        _dependent_edges: Vec<Vec<CrossShardDependentEdges>>,
    ) -> Vec<Vec<CrossShardDependentEdges>> {
        self.dependent_edges_results.clone()
    }
}
