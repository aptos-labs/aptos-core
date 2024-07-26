// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use anyhow::{anyhow, ensure, Result};
use aptos_executor_types::{state_checkpoint_output::StateCheckpointOutput, ExecutedChunk};
use aptos_storage_interface::{state_delta::StateDelta, DbReader, ExecutedTrees};
use aptos_types::{
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    proof::{accumulator::InMemoryTransactionAccumulator, TransactionInfoListWithProof},
    transaction::Version,
};
use std::{collections::VecDeque, sync::Arc};

pub(crate) struct ChunkToUpdateLedger {
    pub result_state: StateDelta,
    /// transactions sorted by status, state roots, state updates
    pub state_checkpoint_output: StateCheckpointOutput,
    /// If set, this is the new epoch info that should be changed to if this is committed.
    pub next_epoch_state: Option<EpochState>,
    /// the below are from the input -- can be checked / used only after the transaction accumulator
    /// is updated.
    pub verified_target_li: LedgerInfoWithSignatures,
    pub epoch_change_li: Option<LedgerInfoWithSignatures>,
    pub txn_infos_with_proof: TransactionInfoListWithProof,
}

/// It's a two stage pipeline:
///           (front)     (front)
///          /           /
///    ... | to_commit | to_update_ledger | ---> (txn version increases)
///         \           \                \
///          \           \                latest_state
///           \           latest_txn_accumulator
///            persisted_state
///
pub struct ChunkCommitQueue {
    persisted_state: StateDelta,
    /// Notice that latest_state and latest_txn_accumulator are at different versions.
    latest_state: StateDelta,
    latest_txn_accumulator: Arc<InMemoryTransactionAccumulator>,
    to_commit: VecDeque<Option<ExecutedChunk>>,
    to_update_ledger: VecDeque<Option<ChunkToUpdateLedger>>,
}

impl ChunkCommitQueue {
    pub(crate) fn new_from_db(db: &Arc<dyn DbReader>) -> Result<Self> {
        let ExecutedTrees {
            state,
            transaction_accumulator,
        } = db.get_latest_executed_trees()?;
        Ok(Self {
            persisted_state: state.clone(),
            latest_state: state,
            latest_txn_accumulator: transaction_accumulator,
            to_commit: VecDeque::new(),
            to_update_ledger: VecDeque::new(),
        })
    }

    pub(crate) fn latest_state(&self) -> StateDelta {
        self.latest_state.clone()
    }

    pub(crate) fn expecting_version(&self) -> Version {
        self.latest_txn_accumulator.num_leaves()
    }

    pub(crate) fn expect_latest_view(&self) -> Result<ExecutedTrees> {
        ensure!(
            self.to_update_ledger.is_empty(),
            "Pending chunk to update_ledger, can't construct latest ExecutedTrees."
        );
        Ok(ExecutedTrees::new(
            self.latest_state.clone(),
            self.latest_txn_accumulator.clone(),
        ))
    }

    pub(crate) fn enqueue_for_ledger_update(
        &mut self,
        chunk_to_update_ledger: ChunkToUpdateLedger,
    ) -> Result<()> {
        self.latest_state = chunk_to_update_ledger.result_state.clone();
        self.to_update_ledger
            .push_back(Some(chunk_to_update_ledger));
        Ok(())
    }

    pub(crate) fn next_chunk_to_update_ledger(
        &mut self,
    ) -> Result<(Arc<InMemoryTransactionAccumulator>, ChunkToUpdateLedger)> {
        let chunk_opt = self
            .to_update_ledger
            .front_mut()
            .ok_or_else(|| anyhow!("No chunk to update ledger."))?;
        let chunk = chunk_opt
            .take()
            .ok_or_else(|| anyhow!("Next chunk to update ledger has already been processed."))?;
        Ok((self.latest_txn_accumulator.clone(), chunk))
    }

    pub(crate) fn save_ledger_update_output(&mut self, chunk: ExecutedChunk) -> Result<()> {
        ensure!(
            !self.to_update_ledger.is_empty(),
            "to_update_ledger is empty."
        );
        ensure!(
            self.to_update_ledger.front().unwrap().is_none(),
            "Head of to_update_ledger has not been processed."
        );
        self.latest_txn_accumulator = chunk.ledger_update_output.transaction_accumulator.clone();
        self.to_update_ledger.pop_front();
        self.to_commit.push_back(Some(chunk));

        Ok(())
    }

    pub(crate) fn next_chunk_to_commit(&mut self) -> Result<(StateDelta, ExecutedChunk)> {
        let chunk_opt = self
            .to_commit
            .front_mut()
            .ok_or_else(|| anyhow!("No chunk to commit."))?;
        let chunk = chunk_opt
            .take()
            .ok_or_else(|| anyhow!("Next chunk to commit has already been processed."))?;
        Ok((self.persisted_state.clone(), chunk))
    }

    pub(crate) fn enqueue_chunk_to_commit_directly(&mut self, chunk: ExecutedChunk) -> Result<()> {
        ensure!(
            self.to_update_ledger.is_empty(),
            "Mixed usage of different modes."
        );
        self.latest_state = chunk.result_state.clone();
        self.latest_txn_accumulator = chunk.ledger_update_output.transaction_accumulator.clone();
        self.to_commit.push_back(Some(chunk));
        Ok(())
    }

    pub(crate) fn dequeue_committed(&mut self, latest_state: StateDelta) -> Result<()> {
        ensure!(!self.to_commit.is_empty(), "to_commit is empty.");
        ensure!(
            self.to_commit.front().unwrap().is_none(),
            "Head of to_commit has not been processed."
        );
        self.to_commit.pop_front();
        self.persisted_state = latest_state;
        self.persisted_state
            .current
            .log_generation("commit_queue_base");
        Ok(())
    }
}
