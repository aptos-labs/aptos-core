// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::StateComputeResult;
use anyhow::{ensure, Result};
use aptos_crypto::HashValue;
use aptos_storage_interface::cached_state_view::ShardedStateCache;
use aptos_types::{
    contract_event::ContractEvent,
    epoch_state::EpochState,
    proof::accumulator::InMemoryTransactionAccumulator,
    state_store::ShardedStateUpdates,
    transaction::{
        block_epilogue::BlockEndInfo, TransactionInfo, TransactionStatus, TransactionToCommit,
        Version,
    },
};
use itertools::zip_eq;
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct LedgerUpdateOutput {
    pub statuses_for_input_txns: Vec<TransactionStatus>,
    pub to_commit: Vec<TransactionToCommit>,
    pub subscribable_events: Vec<ContractEvent>,
    pub transaction_info_hashes: Vec<HashValue>,
    pub state_updates_until_last_checkpoint: Option<ShardedStateUpdates>,
    pub sharded_state_cache: ShardedStateCache,
    /// The in-memory Merkle Accumulator representing a blockchain state consistent with the
    /// `state_tree`.
    pub transaction_accumulator: Arc<InMemoryTransactionAccumulator>,
    pub block_end_info: Option<BlockEndInfo>,
}

impl LedgerUpdateOutput {
    pub fn new_empty(transaction_accumulator: Arc<InMemoryTransactionAccumulator>) -> Self {
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

    pub fn txn_accumulator(&self) -> &Arc<InMemoryTransactionAccumulator> {
        &self.transaction_accumulator
    }

    pub fn transactions_to_commit(&self) -> &Vec<TransactionToCommit> {
        &self.to_commit
    }

    /// Ensure that every block committed by consensus ends with a state checkpoint. That can be
    /// one of the two cases: 1. a reconfiguration (txns in the proposed block after the txn caused
    /// the reconfiguration will be retried) 2. a Transaction::StateCheckpoint at the end of the
    /// block.
    pub fn ensure_ends_with_state_checkpoint(&self) -> Result<()> {
        ensure!(
            self.to_commit
                .last()
                .map_or(true, |txn| txn.transaction().is_non_reconfig_block_ending()),
            "Block not ending with a state checkpoint.",
        );
        Ok(())
    }

    pub fn ensure_transaction_infos_match(
        &self,
        transaction_infos: &[TransactionInfo],
    ) -> Result<()> {
        let first_version =
            self.transaction_accumulator.version() + 1 - self.to_commit.len() as Version;
        ensure!(
            self.transactions_to_commit().len() == transaction_infos.len(),
            "Lengths don't match. {} vs {}",
            self.transactions_to_commit().len(),
            transaction_infos.len(),
        );

        let mut version = first_version;
        for (txn_to_commit, expected_txn_info) in
            zip_eq(self.to_commit.iter(), transaction_infos.iter())
        {
            let txn_info = txn_to_commit.transaction_info();
            ensure!(
                txn_info == expected_txn_info,
                "Transaction infos don't match. version:{version}, txn_info:{txn_info}, expected_txn_info:{expected_txn_info}",
            );
            version += 1;
        }
        Ok(())
    }

    pub fn as_state_compute_result(
        &self,
        parent_accumulator: &Arc<InMemoryTransactionAccumulator>,
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
            self.statuses_for_input_txns.clone(),
            self.transaction_info_hashes.clone(),
            self.subscribable_events.clone(),
            self.block_end_info.clone(),
        )
    }

    pub fn next_version(&self) -> Version {
        self.transaction_accumulator.num_leaves() as Version
    }

    pub fn first_version(&self) -> Version {
        self.transaction_accumulator.num_leaves() - self.to_commit.len() as Version
    }

    pub fn num_txns(&self) -> usize {
        self.to_commit.len()
    }
}
