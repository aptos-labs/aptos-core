// Copyright © Aptos Foundation

use crate::sharded_block_executor::messages::{
    CrossShardMsg, CrossShardMsg::RemoteTxnWriteMsg, RemoteTxnWrite,
};
use aptos_block_executor::txn_commit_listener::TransactionCommitListener;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    block_executor::partitioner::{SubBlock, TxnIdxWithShardId},
    state_store::state_key::StateKey,
    transaction::Transaction,
    write_set::WriteOp,
};
use std::{
    collections::HashMap,
    sync::{mpsc::Sender, Mutex},
};

pub struct CrossShardCommitSender {
    // The senders of cross-shard messages to other shards.
    message_txs: Vec<Mutex<Sender<CrossShardMsg>>>,
    // The hashmap of source txn index to hashmap of conflicting storage location to the
    // list of target txn index and shard id. Please note that the transaction indices stored here is
    // global indices, so we need to convert the local index received from the parallel execution to
    // the global index.
    dependent_edges: HashMap<TxnIndex, HashMap<StateKey, Vec<TxnIdxWithShardId>>>,
    // The offset of the first transaction in the sub-block. This is used to convert the local index
    // in parallel execution to the global index.
    index_offset: TxnIndex,
}

impl CrossShardCommitSender {
    pub fn new(message_txs: Vec<Sender<CrossShardMsg>>, sub_block: &SubBlock<Transaction>) -> Self {
        let mut dependent_edges = HashMap::new();
        for (txn_idx, txn_with_deps) in sub_block.txn_with_index_iter() {
            let mut storage_locations_to_target = HashMap::new();
            for (txn_id_with_shard, storage_locations) in txn_with_deps
                .cross_shard_dependencies
                .dependent_edges()
                .iter()
            {
                for storage_location in storage_locations {
                    storage_locations_to_target
                        .entry(storage_location.clone().into_state_key())
                        .or_insert_with(Vec::new)
                        .push(txn_id_with_shard.clone());
                }
            }
            dependent_edges.insert(txn_idx as TxnIndex, storage_locations_to_target);
        }

        Self {
            message_txs: message_txs.into_iter().map(Mutex::new).collect(),
            dependent_edges,
            index_offset: sub_block.start_index as TxnIndex,
        }
    }
}

impl TransactionCommitListener for CrossShardCommitSender {
    type TransactionWrites = Vec<(StateKey, WriteOp)>;

    fn on_transaction_committed(&self, txn_idx: TxnIndex, txn_writes: &Self::TransactionWrites) {
        let global_txn_idx = txn_idx + self.index_offset;
        if let Some(edges) = self.dependent_edges.get(&global_txn_idx) {
            for (state_key, write_op) in txn_writes.iter() {
                if let Some(dependent_txn_ids) = edges.get(state_key) {
                    for dependent_txn_id in dependent_txn_ids.iter() {
                        let message = RemoteTxnWriteMsg(RemoteTxnWrite::new(
                            dependent_txn_id.txn_index as TxnIndex,
                            state_key.clone(),
                            write_op.clone(),
                        ));
                        let sender = self.message_txs[dependent_txn_id.shard_id].lock().unwrap();
                        sender.send(message).unwrap();
                    }
                }
            }
        }
    }
}
