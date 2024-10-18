// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use anyhow::Result;
use aptos_executor_types::{state_compute_result::StateComputeResult, LedgerUpdateOutput};
use aptos_storage_interface::{
    async_proof_fetcher::AsyncProofFetcher, cached_state_view::CachedStateView,
    state_delta::StateDelta, DbReader,
};
use aptos_types::{
    epoch_state::EpochState, proof::accumulator::InMemoryTransactionAccumulator,
    state_store::StateViewId,
};
use once_cell::sync::OnceCell;
use std::sync::Arc;

#[derive(Debug)]
pub struct PartialStateComputeResult {
    pub parent_state: Arc<StateDelta>,
    pub result_state: Arc<StateDelta>,
    /// If set, this is the new epoch info that should be changed to if this is committed.
    pub next_epoch_state: Option<EpochState>,
    pub ledger_update_output: OnceCell<LedgerUpdateOutput>,
}

impl PartialStateComputeResult {
    pub fn new(
        parent_state: Arc<StateDelta>,
        result_state: Arc<StateDelta>,
        next_epoch_state: Option<EpochState>,
    ) -> Self {
        Self {
            parent_state,
            result_state,
            next_epoch_state,
            ledger_update_output: OnceCell::new(),
        }
    }

    pub fn new_empty_completed(
        state: Arc<StateDelta>,
        txn_accumulator: Arc<InMemoryTransactionAccumulator>,
        next_epoch_state: Option<EpochState>,
    ) -> Self {
        let ledger_update_output = OnceCell::new();
        ledger_update_output
            .set(LedgerUpdateOutput::new_empty(txn_accumulator))
            .expect("First set.");

        Self {
            parent_state: state.clone(),
            result_state: state,
            next_epoch_state,
            ledger_update_output,
        }
    }

    pub fn epoch_state(&self) -> &Option<EpochState> {
        &self.next_epoch_state
    }

    pub fn has_reconfiguration(&self) -> bool {
        self.next_epoch_state.is_some()
    }

    pub fn get_ledger_update_output(&self) -> Option<&LedgerUpdateOutput> {
        self.ledger_update_output.get()
    }

    pub fn expect_ledger_update_output(&self) -> &LedgerUpdateOutput {
        self.ledger_update_output
            .get()
            .expect("LedgerUpdateOutput not set")
    }

    pub fn set_ledger_update_output(&self, ledger_update_output: LedgerUpdateOutput) {
        self.ledger_update_output
            .set(ledger_update_output)
            .expect("LedgerUpdateOutput already set");
    }

    pub fn next_version(&self) -> u64 {
        self.state().current_version.unwrap() + 1
    }

    pub fn is_same_state(&self, rhs: &Self) -> bool {
        self.state().has_same_current_state(rhs.state())
    }

    pub fn state(&self) -> &Arc<StateDelta> {
        &self.result_state
    }

    pub fn get_complete_result(&self) -> Option<StateComputeResult> {
        self.ledger_update_output.get().map(|ledger_update_output| {
            StateComputeResult::new(
                self.parent_state.clone(),
                self.result_state.clone(),
                ledger_update_output.clone(),
                self.next_epoch_state.clone(),
            )
        })
    }

    pub fn expect_complete_result(&self) -> StateComputeResult {
        self.get_complete_result().expect("Result is not complete.")
    }

    pub fn verified_state_view(
        &self,
        id: StateViewId,
        reader: Arc<dyn DbReader>,
        proof_fetcher: Arc<AsyncProofFetcher>,
    ) -> Result<CachedStateView> {
        Ok(CachedStateView::new(
            id,
            reader,
            self.next_version(),
            self.result_state.current.clone(),
            proof_fetcher,
        )?)
    }
}
