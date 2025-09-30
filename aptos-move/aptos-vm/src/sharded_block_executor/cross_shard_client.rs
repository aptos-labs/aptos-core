// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::sharded_block_executor::{
    cross_shard_state_view::CrossShardStateView,
    messages::{CrossShardMsg, CrossShardMsg::RemoteTxnWriteMsg, RemoteTxnWrite},
};
use aptos_block_executor::txn_commit_hook::TransactionCommitHook;
use aptos_logger::trace;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    block_executor::partitioner::{RoundId, ShardId, SubBlock, GLOBAL_ROUND_ID},
    state_store::{state_key::StateKey, StateView},
    transaction::{analyzed_transaction::AnalyzedTransaction, TransactionOutput},
    write_set::TransactionWrite,
};
use once_cell::sync::OnceCell;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

pub struct CrossShardCommitReceiver {}

impl CrossShardCommitReceiver {
    pub fn start<S: StateView + Sync + Send>(
        cross_shard_state_view: Arc<CrossShardStateView<S>>,
        cross_shard_client: Arc<dyn CrossShardClient>,
        round: RoundId,
    ) {
        loop {
            let msg = cross_shard_client.receive_cross_shard_msg(round);
            match msg {
                RemoteTxnWriteMsg(txn_commit_msg) => {
                    let (state_key, write_op) = txn_commit_msg.take();
                    cross_shard_state_view
                        .set_value(&state_key, write_op.and_then(|w| w.as_state_value()));
                },
                CrossShardMsg::StopMsg => {
                    trace!("Cross shard commit receiver stopped for round {}", round);
                    break;
                },
            }
        }
    }
}

pub struct CrossShardCommitSender {
    shard_id: ShardId,
    cross_shard_client: Arc<dyn CrossShardClient>,
    // The hashmap of source txn index to hashmap of conflicting storage location to the
    // list shard id and round id. Please note that the transaction indices stored here is
    // global indices, so we need to convert the local index received from the parallel execution to
    // the global index.
    dependent_edges: HashMap<TxnIndex, HashMap<StateKey, HashSet<(ShardId, RoundId)>>>,
    // The offset of the first transaction in the sub-block. This is used to convert the local index
    // in parallel execution to the global index.
    index_offset: TxnIndex,
}

impl CrossShardCommitSender {
    pub fn new(
        shard_id: ShardId,
        cross_shard_client: Arc<dyn CrossShardClient>,
        sub_block: &SubBlock<AnalyzedTransaction>,
    ) -> Self {
        let mut dependent_edges = HashMap::new();
        let mut num_dependent_edges = 0;
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
                        .or_insert_with(HashSet::new)
                        .insert((txn_id_with_shard.shard_id, txn_id_with_shard.round_id));
                    num_dependent_edges += 1;
                }
            }
            if !storage_locations_to_target.is_empty() {
                dependent_edges.insert(txn_idx as TxnIndex, storage_locations_to_target);
            }
        }

        trace!(
            "CrossShardCommitSender::new: shard_id: {:?}, num_dependent_edges: {:?}",
            shard_id,
            num_dependent_edges
        );

        Self {
            shard_id,
            cross_shard_client,
            dependent_edges,
            index_offset: sub_block.start_index as TxnIndex,
        }
    }

    fn send_remote_update_for_success(
        &self,
        txn_idx: TxnIndex,
        txn_output: &OnceCell<TransactionOutput>,
    ) {
        let edges = self.dependent_edges.get(&txn_idx).unwrap();
        let write_set = txn_output
            .get()
            .expect("Committed output must be set")
            .write_set();

        for (state_key, write_op) in write_set.expect_write_op_iter() {
            if let Some(dependent_shard_ids) = edges.get(state_key) {
                for (dependent_shard_id, round_id) in dependent_shard_ids.iter() {
                    trace!("Sending remote update for success for shard id {:?} and txn_idx: {:?}, state_key: {:?}, dependent shard id: {:?}", self.shard_id, txn_idx, state_key, dependent_shard_id);
                    let message = RemoteTxnWriteMsg(RemoteTxnWrite::new(
                        state_key.clone(),
                        Some(write_op.clone()),
                    ));
                    if *round_id == GLOBAL_ROUND_ID {
                        self.cross_shard_client.send_global_msg(message);
                    } else {
                        self.cross_shard_client.send_cross_shard_msg(
                            *dependent_shard_id,
                            *round_id,
                            message,
                        );
                    }
                }
            }
        }
    }
}

impl TransactionCommitHook for CrossShardCommitSender {
    fn on_transaction_committed(
        &self,
        txn_idx: TxnIndex,
        txn_output: &OnceCell<TransactionOutput>,
    ) {
        let global_txn_idx = txn_idx + self.index_offset;
        if self.dependent_edges.contains_key(&global_txn_idx) {
            self.send_remote_update_for_success(global_txn_idx, txn_output);
        }
    }

    fn on_execution_aborted(&self, _txn_idx: TxnIndex) {
        todo!("on_transaction_aborted not supported for sharded execution yet")
    }
}

// CrossShardClient is a trait that defines the interface for sending and receiving messages across
// shards.
pub trait CrossShardClient: Send + Sync {
    fn send_global_msg(&self, msg: CrossShardMsg);

    fn send_cross_shard_msg(&self, shard_id: ShardId, round: RoundId, msg: CrossShardMsg);

    fn receive_cross_shard_msg(&self, current_round: RoundId) -> CrossShardMsg;
}
