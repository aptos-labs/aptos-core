// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::StateComputeResult;
use anyhow::{ensure, Result};
use aptos_crypto::{hash::TransactionAccumulatorHasher, HashValue};
use aptos_storage_interface::{cached_state_view::ShardedStateCache, ExecutedTrees};
use aptos_types::{
    contract_event::ContractEvent,
    epoch_state::EpochState,
    proof::accumulator::InMemoryAccumulator,
    state_store::ShardedStateUpdates,
    transaction::{Transaction, TransactionStatus, TransactionToCommit},
};
use std::sync::Arc;

#[derive(Default)]
pub struct ExecutedBlock {
    pub status: Vec<TransactionStatus>,
    pub to_commit: Vec<Arc<TransactionToCommit>>,
    pub result_view: ExecutedTrees,
    /// If set, this is the new epoch info that should be changed to if this is committed.
    pub next_epoch_state: Option<EpochState>,
    pub reconfig_events: Vec<ContractEvent>,
    pub transaction_info_hashes: Vec<HashValue>,
    pub block_state_updates: ShardedStateUpdates,
    pub sharded_state_cache: ShardedStateCache,
}

impl ExecutedBlock {
    pub fn new_empty(result_view: ExecutedTrees) -> Self {
        Self {
            result_view,
            ..Default::default()
        }
    }

    pub fn reconfig_suffix(&self) -> Self {
        assert!(self.next_epoch_state.is_some());
        Self {
            result_view: self.result_view.clone(),
            next_epoch_state: self.next_epoch_state.clone(),
            ..Default::default()
        }
    }

    pub fn transactions_to_commit(&self) -> Vec<Arc<TransactionToCommit>> {
        self.to_commit.iter().map(Arc::clone).collect()
    }

    pub fn has_reconfiguration(&self) -> bool {
        self.next_epoch_state.is_some()
    }

    /// Ensure that every block committed by consensus ends with a state checkpoint. That can be
    /// one of the two cases: 1. a reconfiguration (txns in the proposed block after the txn caused
    /// the reconfiguration will be retried) 2. a Transaction::StateCheckpoint at the end of the
    /// block.
    pub fn ensure_ends_with_state_checkpoint(&self) -> Result<()> {
        ensure!(
            self.next_epoch_state.is_some()
                || self.to_commit.last().map_or(true, |txn| matches!(
                    txn.transaction(),
                    Transaction::StateCheckpoint(_)
                )),
            "Block not ending with a state checkpoint.",
        );
        Ok(())
    }

    pub fn as_state_compute_result(
        &self,
        parent_accumulator: &Arc<InMemoryAccumulator<TransactionAccumulatorHasher>>,
    ) -> StateComputeResult {
        let txn_accu = self.result_view.txn_accumulator();

        StateComputeResult::new(
            txn_accu.root_hash(),
            txn_accu.frozen_subtree_roots().clone(),
            txn_accu.num_leaves(),
            parent_accumulator.frozen_subtree_roots().clone(),
            parent_accumulator.num_leaves(),
            self.next_epoch_state.clone(),
            self.status.clone(),
            self.transaction_info_hashes.clone(),
            self.reconfig_events.clone(),
        )
    }
}
