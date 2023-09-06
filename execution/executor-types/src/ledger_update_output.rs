// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::StateComputeResult;
use anyhow::{ensure, Result};
use aptos_crypto::{hash::TransactionAccumulatorHasher, HashValue};
use aptos_storage_interface::cached_state_view::ShardedStateCache;
use aptos_types::{
    contract_event::ContractEvent,
    epoch_state::EpochState,
    proof::accumulator::InMemoryAccumulator,
    state_store::ShardedStateUpdates,
    transaction::{Transaction, TransactionStatus, TransactionToCommit},
};
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct LedgerUpdateOutput {
    pub status: Vec<TransactionStatus>,
    pub to_commit: Vec<Arc<TransactionToCommit>>,
    pub reconfig_events: Vec<ContractEvent>,
    pub transaction_info_hashes: Vec<HashValue>,
    pub block_state_updates: ShardedStateUpdates,
    pub sharded_state_cache: ShardedStateCache,
    /// The in-memory Merkle Accumulator representing a blockchain state consistent with the
    /// `state_tree`.
    pub transaction_accumulator: Arc<InMemoryAccumulator<TransactionAccumulatorHasher>>,
}

impl LedgerUpdateOutput {
    pub fn new_empty(
        transaction_accumulator: Arc<InMemoryAccumulator<TransactionAccumulatorHasher>>,
    ) -> Self {
        Self {
            transaction_accumulator,
            ..Default::default()
        }
    }

    pub fn reconfig_suffix(&self) -> Self {
        Self {
            transaction_accumulator: Arc::clone(&self.transaction_accumulator),
            ..Default::default()
        }
    }

    pub fn txn_accumulator(&self) -> &Arc<InMemoryAccumulator<TransactionAccumulatorHasher>> {
        &self.transaction_accumulator
    }

    pub fn transactions_to_commit(&self) -> Vec<Arc<TransactionToCommit>> {
        self.to_commit.iter().map(Arc::clone).collect()
    }

    /// Ensure that every block committed by consensus ends with a state checkpoint. That can be
    /// one of the two cases: 1. a reconfiguration (txns in the proposed block after the txn caused
    /// the reconfiguration will be retried) 2. a Transaction::StateCheckpoint at the end of the
    /// block.
    pub fn ensure_ends_with_state_checkpoint(&self) -> Result<()> {
        ensure!(
            self.to_commit.last().map_or(true, |txn| matches!(
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
        next_epoch_state: Option<EpochState>,
    ) -> StateComputeResult {
        let txn_accu = self.txn_accumulator();

        StateComputeResult::new(
            txn_accu.root_hash(),
            txn_accu.frozen_subtree_roots().clone(),
            txn_accu.num_leaves(),
            parent_accumulator.frozen_subtree_roots().clone(),
            parent_accumulator.num_leaves(),
            next_epoch_state,
            self.status.clone(),
            self.transaction_info_hashes.clone(),
            self.reconfig_events.clone(),
        )
    }
}
