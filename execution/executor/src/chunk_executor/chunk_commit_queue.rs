// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    chunk_executor::chunk_result_verifier::ChunkResultVerifier,
    types::{
        executed_chunk::ExecutedChunk, partial_state_compute_result::PartialStateComputeResult,
    },
};
use anyhow::{anyhow, ensure, Result};
use aptos_storage_interface::{state_store::state_delta::StateDelta, DbReader, LedgerSummary};
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
///                       latest_txn_accumulator
///
pub struct ChunkCommitQueue {
    /// Notice that latest_state and latest_txn_accumulator are at different versions.
    latest_state: Arc<StateDelta>,
    latest_txn_accumulator: Arc<InMemoryTransactionAccumulator>,
    to_commit: VecDeque<Option<ExecutedChunk>>,
    to_update_ledger: VecDeque<Option<ChunkToUpdateLedger>>,
}

impl ChunkCommitQueue {
    pub(crate) fn new_from_db(db: &Arc<dyn DbReader>) -> Result<Self> {
        let LedgerSummary {
            state,
            transaction_accumulator,
        } = db.get_pre_committed_ledger_summary()?;
        Ok(Self {
            latest_state: state,
            latest_txn_accumulator: transaction_accumulator,
            to_commit: VecDeque::new(),
            to_update_ledger: VecDeque::new(),
        })
    }

    pub(crate) fn latest_state(&self) -> Arc<StateDelta> {
        self.latest_state.clone()
    }

    pub(crate) fn expecting_version(&self) -> Version {
        self.latest_state.next_version()
    }

    pub(crate) fn enqueue_for_ledger_update(
        &mut self,
        chunk_to_update_ledger: ChunkToUpdateLedger,
    ) -> Result<()> {
        self.latest_state = chunk_to_update_ledger.output.expect_result_state().clone();
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
        self.latest_txn_accumulator = chunk
            .output
            .expect_ledger_update_output()
            .transaction_accumulator
            .clone();
        self.to_update_ledger.pop_front();
        self.to_commit.push_back(Some(chunk));

        Ok(())
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

    pub(crate) fn dequeue_committed(&mut self) -> Result<()> {
        ensure!(!self.to_commit.is_empty(), "to_commit is empty.");
        ensure!(
            self.to_commit.front().unwrap().is_none(),
            "Head of to_commit has not been processed."
        );
        self.to_commit.pop_front();
        Ok(())
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.to_commit.is_empty() && self.to_update_ledger.is_empty()
    }
}
