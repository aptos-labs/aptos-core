// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::{
    state::LedgerState,
    state_summary::LedgerStateSummary,
    state_update_refs::StateUpdateRefs,
    state_view::cached_state_view::ShardedStateCache,
    state_with_summary::{LedgerStateWithSummary, StateWithSummary},
};
use aptos_types::transaction::{
    PersistedAuxiliaryInfo, Transaction, TransactionInfo, TransactionOutput, Version,
};

#[derive(Clone)]
pub struct ChunkToCommit<'a> {
    pub first_version: Version,
    pub transactions: &'a [Transaction],
    pub persisted_auxiliary_infos: &'a [PersistedAuxiliaryInfo],
    pub transaction_outputs: &'a [TransactionOutput],
    pub transaction_infos: &'a [TransactionInfo],
    pub state: &'a LedgerState,
    pub state_summary: &'a LedgerStateSummary,
    pub state_update_refs: &'a StateUpdateRefs<'a>,
    pub state_reads: &'a ShardedStateCache,
    pub is_reconfig: bool,
}

impl ChunkToCommit<'_> {
    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn next_version(&self) -> Version {
        self.first_version + self.len() as Version
    }

    pub fn expect_last_version(&self) -> Version {
        self.next_version() - 1
    }

    pub fn result_ledger_state_with_summary(&self) -> LedgerStateWithSummary {
        let latest = StateWithSummary::new(
            self.state.latest().clone(),
            self.state_summary.latest().clone(),
        );
        let last_checkpoint = StateWithSummary::new(
            self.state.last_checkpoint().clone(),
            self.state_summary.last_checkpoint().clone(),
        );
        LedgerStateWithSummary::from_latest_and_last_checkpoint(latest, last_checkpoint)
    }

    pub fn estimated_total_state_updates(&self) -> usize {
        let for_last_checkpoint = self
            .state_update_refs
            .for_last_checkpoint_batched()
            .map_or(0, |x| x.len());
        let for_latest = self
            .state_update_refs
            .for_latest_batched()
            .map_or(0, |x| x.len());

        for_latest + for_last_checkpoint
    }
}
