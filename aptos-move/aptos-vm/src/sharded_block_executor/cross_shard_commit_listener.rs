// Copyright © Aptos Foundation

use crate::sharded_block_executor::messages::{
    CrossShardMsg, CrossShardMsg::RemoteTxnCommitMsg, RemoteTxnCommit,
};
use aptos_block_executor::txn_commit_listener::TransactionCommitListener;
use aptos_infallible::Mutex;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    block_executor::partitioner::{CrossShardEdges, SubBlocksForShard},
    state_store::state_key::StateKey,
    transaction::Transaction,
    write_set::WriteOp,
};
use std::{collections::HashMap, sync::mpsc::Sender};

pub struct CrossShardCommitListener {
    // The senders of cross-shard messages to other shards.
    message_txs: Vec<Mutex<Sender<CrossShardMsg>>>,
    // The hashmap of source txn index to corresponding cross shard dependent edges.
    dependent_edges: HashMap<TxnIndex, CrossShardEdges>,
}

impl CrossShardCommitListener {
    pub fn new(
        message_txs: &[Sender<CrossShardMsg>],
        transactions: &SubBlocksForShard<Transaction>,
    ) -> Self {
        let mut dependent_edges = HashMap::new();
        for (txn_idx, txn_with_deps) in transactions.txn_with_index_iter() {
            dependent_edges.insert(
                txn_idx as TxnIndex,
                txn_with_deps
                    .cross_shard_dependencies
                    .required_edges()
                    .clone(),
            );
        }

        Self {
            message_txs: message_txs
                .iter()
                .map(|sender| Mutex::new(sender.clone()))
                .collect(),
            dependent_edges,
        }
    }
}

impl TransactionCommitListener for CrossShardCommitListener {
    type TransactionWrites = Vec<(StateKey, WriteOp)>;

    fn on_transaction_committed(&self, txn_idx: TxnIndex, txn_writes: &Self::TransactionWrites) {
        if let Some(edges) = self.dependent_edges.get(&txn_idx) {
            for (txn_id_with_shard, _) in edges.iter() {
                let message = RemoteTxnCommitMsg(RemoteTxnCommit::new(
                    txn_idx,
                    // Note that its possible that the remote shard doesn't need all the writes
                    // from the transaction but we send them all anyway.
                    txn_writes.to_vec(),
                ));
                let sender = self.message_txs[txn_id_with_shard.shard_id].lock();
                sender.send(message).unwrap();
            }
        }
    }
}
