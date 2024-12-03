// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    state_store::{
        state::LedgerState,
        state_summary::LedgerStateSummary,
        state_view::{async_proof_fetcher::AsyncProofFetcher, cached_state_view::CachedStateView},
    },
    DbReader,
};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_types::{
    proof::accumulator::{InMemoryAccumulator, InMemoryTransactionAccumulator},
    state_store::{state_storage_usage::StateStorageUsage, StateViewId},
    transaction::Version,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct LedgerSummary {
    pub state: LedgerState,
    pub state_summary: LedgerStateSummary,
    pub transaction_accumulator: Arc<InMemoryTransactionAccumulator>,
}

impl LedgerSummary {
    pub fn new(
        state: LedgerState,
        state_summary: LedgerStateSummary,
        transaction_accumulator: Arc<InMemoryTransactionAccumulator>,
    ) -> Self {
        state_summary.assert_versions_match(&state);

        Self {
            state,
            state_summary,
            transaction_accumulator,
        }
    }

    pub fn next_version(&self) -> Version {
        self.transaction_accumulator.num_leaves()
    }

    pub fn version(&self) -> Option<Version> {
        self.next_version().checked_sub(1)
    }

    pub fn new_at_state_checkpoint(
        _state_root_hash: HashValue,
        _state_usage: StateStorageUsage,
        _frozen_subtrees_in_accumulator: Vec<HashValue>,
        _num_leaves_in_accumulator: u64,
    ) -> Self {
        todo!()
        /* FIXME(aldenhu)
        let state = Arc::new(StateDelta::new_at_checkpoint(
            state_root_hash,
            state_usage,
            num_leaves_in_accumulator.checked_sub(1),
        ));
        let transaction_accumulator = Arc::new(
            InMemoryAccumulator::new(frozen_subtrees_in_accumulator, num_leaves_in_accumulator)
                .expect("The startup info read from storage should be valid."),
        );

        Self::new(state, transaction_accumulator)
         */
    }

    pub fn new_empty() -> Self {
        let state = LedgerState::new_empty();
        let state_summary = LedgerStateSummary::new_empty();
        Self::new(
            state,
            state_summary,
            Arc::new(InMemoryAccumulator::new_empty()),
        )
    }

    pub fn is_same_view(&self, _rhs: &Self) -> bool {
        todo!()
        /* FIXME(aldenhu)
        self.state.has_same_current_state(rhs.state())
            && self.transaction_accumulator.root_hash() == rhs.transaction_accumulator.root_hash()

         */
    }

    pub fn verified_state_view(
        &self,
        _id: StateViewId,
        _reader: Arc<dyn DbReader>,
        _proof_fetcher: Arc<AsyncProofFetcher>,
    ) -> Result<CachedStateView> {
        todo!()
        /* FIXME(aldenhu)
        Ok(CachedStateView::new(
            id,
            reader,
            self.transaction_accumulator.num_leaves(),
            self.state.current.clone(),
            proof_fetcher,
        )?)
         */
    }
}

impl Default for LedgerSummary {
    fn default() -> Self {
        Self::new_empty()
    }
}
