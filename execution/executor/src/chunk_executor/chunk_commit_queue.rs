// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::{
    chunk_executor::chunk_result_verifier::ChunkResultVerifier,
    metrics::CHUNK_OTHER_TIMERS,
    types::{
        executed_chunk::ExecutedChunk, partial_state_compute_result::PartialStateComputeResult,
    },
};
use anyhow::{anyhow, ensure, Result};
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::{
    state_store::{
        positions::PositionOverlay, sharded_jmt_state::PositionStateWithSummary,
        state::LedgerState, state_summary::LedgerStateSummary,
        state_with_summary::LedgerWithSummary,
    },
    DbReader, LedgerSummary,
};
use aptos_types::{proof::accumulator::InMemoryTransactionAccumulator, transaction::Version};
use std::{collections::VecDeque, sync::Arc};

pub(crate) struct ChunkToUpdateLedger {
    pub output: PartialStateComputeResult,

    /// from the input -- can be checked / used only after the transaction accumulator
    /// is updated.
    pub chunk_verifier: Arc<dyn ChunkResultVerifier + Send + Sync>,
}

/// It's a two stage pipeline:
///           (front)     (front)
///          /           /
///    ... | to_commit | to_update_ledger | ---> (txn version increases)
///                     \                \
///                      \                latest_state
///                       latest_state_summary
///                       latest_txn_accumulator
///
pub struct ChunkCommitQueue {
    /// Notice that latest_state and latest_txn_accumulator are at different versions.
    latest_state: LedgerState,
    latest_state_summary: LedgerStateSummary,
    /// Native-position summary chained across chunks (the parent the next
    /// chunk rebases from). `None` when native position is disabled.
    latest_position_state_summary: Option<LedgerWithSummary<PositionStateWithSummary>>,
    /// `PositionOverlay` chained across chunks — the parent the next
    /// chunk extends from.
    ///
    /// Advanced at *enqueue*, unlike `latest_position_state_summary`,
    /// which is a checkpoint-stage product and advances at ledger
    /// update. They are deliberately out of step: a chunk's overlay
    /// exists as soon as it is executed, and the next chunk may be
    /// enqueued before this one's ledger update runs. `None` when native
    /// position is disabled.
    latest_positions: Option<LedgerWithSummary<PositionOverlay>>,
    /// Overlay of the last chunk actually committed, which is the safe
    /// target to fold into the position base — `latest_positions` runs
    /// ahead of it by whatever is still in the pipeline.
    committed_positions: Option<LedgerWithSummary<PositionOverlay>>,
    latest_txn_accumulator: Arc<InMemoryTransactionAccumulator>,
    to_commit: VecDeque<Option<ExecutedChunk>>,
    to_update_ledger: VecDeque<Option<ChunkToUpdateLedger>>,
}

impl ChunkCommitQueue {
    pub(crate) fn new_from_db(db: &Arc<dyn DbReader>) -> Result<Self> {
        let LedgerSummary {
            state,
            state_summary,
            transaction_accumulator,
            position_state_summary,
            positions,
        } = db.get_pre_committed_ledger_summary()?;

        Ok(Self {
            latest_state: state,
            latest_state_summary: state_summary,
            latest_position_state_summary: position_state_summary,
            latest_positions: positions.clone(),
            committed_positions: positions,
            latest_txn_accumulator: transaction_accumulator,
            to_commit: VecDeque::new(),
            to_update_ledger: VecDeque::new(),
        })
    }

    pub(crate) fn latest_state(&self) -> &LedgerState {
        &self.latest_state
    }

    pub(crate) fn expecting_version(&self) -> Version {
        self.latest_state.next_version()
    }

    pub(crate) fn enqueue_for_ledger_update(
        &mut self,
        chunk_to_update_ledger: ChunkToUpdateLedger,
    ) -> Result<()> {
        let _timer = CHUNK_OTHER_TIMERS.timer_with(&["enqueue_for_ledger_update"]);

        self.latest_state = chunk_to_update_ledger.output.result_state().clone();
        // Advance alongside `latest_state`, not at ledger update: both are
        // produced by execution, and the next chunk may be enqueued before
        // this one's ledger update runs. Leaving it behind would hand that
        // chunk a stale parent, so it would extend this chunk's parent
        // instead of this chunk and silently drop these writes.
        self.latest_positions = chunk_to_update_ledger
            .output
            .ensure_result_positions()?
            .cloned();
        self.to_update_ledger
            .push_back(Some(chunk_to_update_ledger));
        Ok(())
    }

    pub(crate) fn next_chunk_to_update_ledger(
        &mut self,
    ) -> Result<(
        LedgerStateSummary,
        Option<LedgerWithSummary<PositionStateWithSummary>>,
        Arc<InMemoryTransactionAccumulator>,
        ChunkToUpdateLedger,
    )> {
        let chunk_opt = self
            .to_update_ledger
            .front_mut()
            .ok_or_else(|| anyhow!("No chunk to update ledger."))?;
        let chunk = chunk_opt
            .take()
            .ok_or_else(|| anyhow!("Next chunk to update ledger has already been processed."))?;
        Ok((
            self.latest_state_summary.clone(),
            self.latest_position_state_summary.clone(),
            self.latest_txn_accumulator.clone(),
            chunk,
        ))
    }

    pub(crate) fn save_ledger_update_output(&mut self, chunk: ExecutedChunk) -> Result<()> {
        let _timer = CHUNK_OTHER_TIMERS.timer_with(&["save_ledger_update_output"]);

        ensure!(
            !self.to_update_ledger.is_empty(),
            "to_update_ledger is empty."
        );
        ensure!(
            self.to_update_ledger.front().unwrap().is_none(),
            "Head of to_update_ledger has not been processed."
        );
        self.latest_state_summary = chunk
            .output
            .ensure_state_checkpoint_output()?
            .state_summary
            .clone();
        self.latest_position_state_summary = chunk
            .output
            .ensure_state_checkpoint_output()?
            .position_state_summary
            .clone();
        self.latest_txn_accumulator = chunk
            .output
            .ensure_ledger_update_output()?
            .transaction_accumulator
            .clone();
        self.to_update_ledger.pop_front();
        self.to_commit.push_back(Some(chunk));

        Ok(())
    }

    pub(crate) fn latest_positions(&self) -> Option<LedgerWithSummary<PositionOverlay>> {
        self.latest_positions.clone()
    }

    pub(crate) fn next_chunk_to_commit(&mut self) -> Result<ExecutedChunk> {
        let chunk_opt = self
            .to_commit
            .front_mut()
            .ok_or_else(|| anyhow!("No chunk to commit."))?;
        let chunk = chunk_opt
            .take()
            .ok_or_else(|| anyhow!("Next chunk to commit has already been processed."))?;
        Ok(chunk)
    }

    pub(crate) fn dequeue_committed(
        &mut self,
        committed_positions: Option<LedgerWithSummary<PositionOverlay>>,
    ) -> Result<()> {
        ensure!(!self.to_commit.is_empty(), "to_commit is empty.");
        ensure!(
            self.to_commit.front().unwrap().is_none(),
            "Head of to_commit has not been processed."
        );
        self.to_commit.pop_front();
        // `None` throughout when native position is off, `Some` throughout
        // when it is on, so this needs no guard.
        self.committed_positions = committed_positions;
        Ok(())
    }

    /// The fold target: the last chunk whose write set is durable.
    pub(crate) fn committed_positions(&self) -> Option<LedgerWithSummary<PositionOverlay>> {
        self.committed_positions.clone()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.to_commit.is_empty() && self.to_update_ledger.is_empty()
    }
}
