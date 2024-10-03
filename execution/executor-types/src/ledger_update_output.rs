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
    ledger_info::LedgerInfoWithSignatures,
    proof::accumulator::InMemoryTransactionAccumulator,
    state_store::{combine_or_add_sharded_state_updates, ShardedStateUpdates},
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

    pub fn maybe_select_chunk_ending_ledger_info(
        &self,
        verified_target_li: &LedgerInfoWithSignatures,
        epoch_change_li: Option<&LedgerInfoWithSignatures>,
        next_epoch_state: Option<&EpochState>,
    ) -> Result<Option<LedgerInfoWithSignatures>> {
        if verified_target_li.ledger_info().version() + 1
            == self.transaction_accumulator.num_leaves()
        {
            // If the chunk corresponds to the target LI, the target LI can be added to storage.
            ensure!(
                verified_target_li
                    .ledger_info()
                    .transaction_accumulator_hash()
                    == self.transaction_accumulator.root_hash(),
                "Root hash in target ledger info does not match local computation. {:?} != {:?}",
                verified_target_li,
                self.transaction_accumulator,
            );
            Ok(Some(verified_target_li.clone()))
        } else if let Some(epoch_change_li) = epoch_change_li {
            // If the epoch change LI is present, it must match the version of the chunk:

            // Verify that the given ledger info corresponds to the new accumulator.
            ensure!(
                epoch_change_li.ledger_info().transaction_accumulator_hash()
                    == self.transaction_accumulator.root_hash(),
                "Root hash of a given epoch LI does not match local computation. {:?} vs {:?}",
                epoch_change_li,
                self.transaction_accumulator,
            );
            ensure!(
                epoch_change_li.ledger_info().version() + 1
                    == self.transaction_accumulator.num_leaves(),
                "Version of a given epoch LI does not match local computation. {:?} vs {:?}",
                epoch_change_li,
                self.transaction_accumulator,
            );
            ensure!(
                epoch_change_li.ledger_info().ends_epoch(),
                "Epoch change LI does not carry validator set. version:{}",
                epoch_change_li.ledger_info().version(),
            );
            ensure!(
                epoch_change_li.ledger_info().next_epoch_state() == next_epoch_state,
                "New validator set of a given epoch LI does not match local computation. {:?} vs {:?}",
                epoch_change_li.ledger_info().next_epoch_state(),
                next_epoch_state,
            );
            Ok(Some(epoch_change_li.clone()))
        } else {
            ensure!(
                next_epoch_state.is_none(),
                "End of epoch chunk based on local computation but no EoE LedgerInfo provided. version: {:?}",
                self.transaction_accumulator.num_leaves().checked_sub(1),
            );
            Ok(None)
        }
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

    pub fn combine(&mut self, rhs: Self) {
        assert!(self.block_end_info.is_none());
        assert!(rhs.block_end_info.is_none());
        let Self {
            statuses_for_input_txns,
            to_commit,
            subscribable_events,
            transaction_info_hashes,
            state_updates_until_last_checkpoint: state_updates_before_last_checkpoint,
            sharded_state_cache,
            transaction_accumulator,
            block_end_info: _block_end_info,
        } = rhs;

        if let Some(updates) = state_updates_before_last_checkpoint {
            combine_or_add_sharded_state_updates(
                &mut self.state_updates_until_last_checkpoint,
                updates,
            );
        }

        self.statuses_for_input_txns.extend(statuses_for_input_txns);
        self.to_commit.extend(to_commit);
        self.subscribable_events.extend(subscribable_events);
        self.transaction_info_hashes.extend(transaction_info_hashes);
        self.sharded_state_cache.combine(sharded_state_cache);
        self.transaction_accumulator = transaction_accumulator;
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
