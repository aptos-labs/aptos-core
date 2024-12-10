// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::{
    per_version_state_update_refs::PerVersionStateUpdateRefs,
    state::LedgerState,
    state_summary::{LedgerStateSummary, StateWithSummary},
    state_view::cached_state_view::ShardedStateCache,
};
use aptos_types::transaction::{Transaction, TransactionInfo, TransactionOutput, Version};

#[derive(Clone)]
pub struct ChunkToCommit<'a> {
    pub first_version: Version,
    pub transactions: &'a [Transaction],
    pub transaction_outputs: &'a [TransactionOutput],
    pub transaction_infos: &'a [TransactionInfo],
    pub state: &'a LedgerState,
    pub state_summary: &'a LedgerStateSummary,
    pub state_update_refs: &'a PerVersionStateUpdateRefs<'a>,
    pub state_reads: Option<&'a ShardedStateCache>,
    pub is_reconfig: bool,
}

impl<'a> ChunkToCommit<'a> {
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

    // FIXME(aldenhu): check efficiency (did we clone more than needed?)
    pub fn state(&self) -> StateWithSummary {
        StateWithSummary::new(
            self.state.latest().clone(),
            self.state_summary.latest().clone(),
        )
    }

    pub fn last_checkpoint_state(&self) -> StateWithSummary {
        StateWithSummary::new(
            self.state.last_checkpoint().clone(),
            self.state_summary.last_checkpoint().clone(),
        )
    }
}
