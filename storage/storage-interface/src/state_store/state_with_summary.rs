// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::{
    state::{LedgerState, State},
    state_summary::{LedgerStateSummary, StateSummary},
};
use aptos_crypto::HashValue;
use aptos_scratchpad::SparseMerkleTree;
use aptos_types::{
    state_store::{hot_state::HotStateConfig, state_storage_usage::StateStorageUsage},
    transaction::Version,
};
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

    pub fn new_empty(hot_state_config: HotStateConfig) -> Self {
        Self::new(
            State::new_empty(hot_state_config),
            StateSummary::new_empty(),
        )
    }

    pub fn new_at_version(
        version: Option<Version>,
        hot_state_root_hash: HashValue,
        global_state_root_hash: HashValue,
        usage: StateStorageUsage,
        hot_state_config: HotStateConfig,
    ) -> Self {
        Self::new(
            State::new_at_version(version, usage, hot_state_config),
            StateSummary::new_at_version(
                version,
                SparseMerkleTree::new(hot_state_root_hash),
                SparseMerkleTree::new(global_state_root_hash),
            ),
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

    pub fn into_inner(self) -> (State, StateSummary) {
        let Self { state, summary } = self;

        (state, summary)
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

    pub fn new_empty(hot_state_config: HotStateConfig) -> Self {
        let empty = StateWithSummary::new_empty(hot_state_config);
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

    pub fn is_at_checkpoint(&self) -> bool {
        self.latest.next_version() == self.last_checkpoint.next_version()
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
