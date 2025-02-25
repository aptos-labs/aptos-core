// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::{
    state::{LedgerState, State},
    state_summary::{LedgerStateSummary, StateSummary},
};
use aptos_crypto::HashValue;
use aptos_scratchpad::SparseMerkleTree;
use aptos_types::{state_store::state_storage_usage::StateStorageUsage, transaction::Version};
use derive_more::{Deref, DerefMut};

#[derive(Clone, Debug, Deref)]
pub struct StateWithSummary {
    #[deref]
    state: State,
    summary: StateSummary,
}

impl StateWithSummary {
    pub fn new(state: State, summary: StateSummary) -> Self {
        assert_eq!(state.next_version(), summary.next_version());
        Self { state, summary }
    }

    pub fn new_empty() -> Self {
        Self::new(State::new_empty(), StateSummary::new_empty())
    }

    pub fn new_at_version(
        version: Option<Version>,
        global_state_root_hash: HashValue,
        usage: StateStorageUsage,
    ) -> Self {
        Self::new(
            State::new_at_version(version, usage),
            StateSummary::new_at_version(version, SparseMerkleTree::new(global_state_root_hash)),
        )
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn summary(&self) -> &StateSummary {
        &self.summary
    }

    pub fn is_descendant_of(&self, other: &Self) -> bool {
        self.state.is_descendant_of(&other.state) && self.summary.is_descendant_of(&other.summary)
    }
}

#[derive(Clone, Debug, Deref, DerefMut)]
pub struct LedgerStateWithSummary {
    #[deref]
    #[deref_mut]
    latest: StateWithSummary,
    last_checkpoint: StateWithSummary,
}

impl LedgerStateWithSummary {
    pub fn from_latest_and_last_checkpoint(
        latest: StateWithSummary,
        last_checkpoint: StateWithSummary,
    ) -> Self {
        assert!(latest.is_descendant_of(&last_checkpoint));
        Self {
            latest,
            last_checkpoint,
        }
    }

    pub fn new_at_checkpoint(checkpoint: StateWithSummary) -> Self {
        Self::from_latest_and_last_checkpoint(checkpoint.clone(), checkpoint)
    }

    pub fn new_dummy() -> Self {
        let empty = StateWithSummary::new_empty();
        Self::from_latest_and_last_checkpoint(empty.clone(), empty)
    }

    pub fn from_state_and_summary(state: LedgerState, summary: LedgerStateSummary) -> Self {
        Self::from_latest_and_last_checkpoint(
            StateWithSummary::new(state.latest().clone(), summary.latest().clone()),
            StateWithSummary::new(
                state.last_checkpoint().clone(),
                summary.last_checkpoint().clone(),
            ),
        )
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

    pub fn to_state_and_summary(&self) -> (LedgerState, LedgerStateSummary) {
        (self.ledger_state(), self.ledger_state_summary())
    }

    pub fn is_descendant_of(&self, rhs: &Self) -> bool {
        self.latest.is_descendant_of(&rhs.latest)
            && self.last_checkpoint.is_descendant_of(&rhs.last_checkpoint)
    }
}
