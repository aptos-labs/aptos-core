// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::hash::SPARSE_MERKLE_PLACEHOLDER_HASH;
use aptos_storage_interface::state_store::{
    state::LedgerState,
    state_delta::StateDelta,
    state_summary::{LedgerStateSummary, StateWithSummary},
};
use aptos_types::{state_store::state_storage_usage::StateStorageUsage, transaction::Version};
use derive_more::{Deref, DerefMut};

#[derive(Clone, Debug, Deref, DerefMut)]
pub(crate) struct LedgerStateWithSummary {
    #[deref]
    #[deref_mut]
    latest: StateWithSummary,
    last_checkpoint: StateWithSummary,
}

impl LedgerStateWithSummary {
    pub fn new(latest: StateWithSummary, last_checkpoint: StateWithSummary) -> Self {
        assert!(latest.is_descendant_of(&last_checkpoint));
        Self {
            latest,
            last_checkpoint,
        }
    }

    pub fn new_at_checkpoint(checkpoint: StateWithSummary) -> Self {
        Self::new(checkpoint.clone(), checkpoint)
    }

    pub fn new_dummy() -> Self {
        let empty = StateWithSummary::new_empty();
        Self::new(empty.clone(), empty)
    }

    pub(crate) fn new_dummy_at_checkpoint_version(
        version: Option<Version>,
    ) -> LedgerStateWithSummary {
        let state = StateWithSummary::new_at_version(
            version,
            *SPARSE_MERKLE_PLACEHOLDER_HASH,
            StateStorageUsage::new_untracked(),
        );
        LedgerStateWithSummary::new_at_checkpoint(state)
    }

    pub fn from_state_and_summary(state: LedgerState, summary: LedgerStateSummary) -> Self {
        Self::new(
            StateWithSummary::new(state.latest().clone(), summary.latest().clone()),
            StateWithSummary::new(
                state.last_checkpoint().clone(),
                summary.last_checkpoint().clone(),
            ),
        )
    }

    pub fn set(&mut self, current_state: LedgerStateWithSummary) {
        *self = current_state;
    }

    pub fn last_checkpoint(&self) -> &StateWithSummary {
        &self.last_checkpoint
    }

    pub fn ledger_state(&self) -> LedgerState {
        LedgerState::new(
            self.latest.state().clone(),
            self.last_checkpoint.state().clone(),
        )
    }

    pub fn ledger_state_summary(&self) -> LedgerStateSummary {
        LedgerStateSummary::new(
            self.last_checkpoint.summary().clone(),
            self.latest.summary().clone(),
        )
    }

    pub fn transpose(&self) -> (LedgerState, LedgerStateSummary) {
        (self.ledger_state(), self.ledger_state_summary())
    }

    pub fn is_descendant_of(&self, rhs: &Self) -> bool {
        self.latest.is_descendant_of(&rhs.latest)
            && self.last_checkpoint.is_descendant_of(&rhs.last_checkpoint)
    }
}
