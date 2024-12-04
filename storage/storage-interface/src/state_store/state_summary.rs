// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::{
    state::{LedgerState, State},
    state_delta::StateDelta,
};
use aptos_crypto::HashValue;
use aptos_scratchpad::{ProofRead, SparseMerkleTree};
use aptos_types::{
    state_store::{state_storage_usage::StateStorageUsage, state_value::StateValue},
    transaction::Version,
};
use derive_more::Deref;
use std::sync::Arc;

/// The data structure through which the entire state at a given
/// version can be summarized to a concise digest (the root hash).
#[derive(Clone, Debug)]
pub struct StateSummary {
    /// The next version. If this is 0, the state is the "pre-genesis" empty state.
    next_version: Version,
    pub global_state_summary: SparseMerkleTree<StateValue>,
}

impl StateSummary {
    pub fn new_at_version(
        version: Option<Version>,
        global_state_summary: SparseMerkleTree<StateValue>,
    ) -> Self {
        Self {
            next_version: version.map_or(0, |v| v + 1),
            global_state_summary,
        }
    }

    pub fn new_empty() -> Self {
        Self {
            next_version: 0,
            global_state_summary: SparseMerkleTree::new_empty(),
        }
    }

    pub fn root_hash(&self) -> HashValue {
        self.global_state_summary.root_hash()
    }

    pub fn next_version(&self) -> Version {
        self.next_version
    }

    pub fn is_the_same(&self, _rhs: &Self) -> bool {
        // FIXME(aldenhu)
        todo!()
    }

    pub fn is_family(&self, _rhs: &Self) -> bool {
        // FIXME(aldenhu)
        todo!()
    }

    pub fn update(
        &self,
        _persisted: &StateSummary,
        _base: &StateSummary,
        _updates: &StateDelta,
        _proof_reader: Arc<dyn ProofRead>,
    ) -> Self {
        // FIXME(aldenhu)
        todo!()
    }
}

/// At a given version, the summaries of the state and the last checkpoint state at or before the version.
#[derive(Clone, Debug, Deref)]
pub struct LedgerStateSummary {
    #[deref]
    latest: StateSummary,
    last_checkpoint: StateSummary,
}

impl LedgerStateSummary {
    pub fn new(last_checkpoint_summary: StateSummary, state_summary: StateSummary) -> Self {
        assert!(last_checkpoint_summary.next_version() <= state_summary.next_version());

        Self {
            last_checkpoint: last_checkpoint_summary,
            latest: state_summary,
        }
    }

    pub fn new_empty() -> Self {
        let state_summary = StateSummary::new_empty();
        Self::new(state_summary.clone(), state_summary)
    }

    pub fn next_version(&self) -> Version {
        self.latest.next_version()
    }

    pub fn assert_versions_match(&self, latest_state: &LedgerState) {
        assert_eq!(self.next_version(), latest_state.next_version());
        assert_eq!(
            self.last_checkpoint.next_version(),
            latest_state.last_checkpoint().next_version()
        );
    }

    pub fn latest(&self) -> &StateSummary {
        &self.latest
    }

    pub fn last_checkpoint(&self) -> &StateSummary {
        &self.last_checkpoint
    }

    pub fn update(
        &self,
        _persisted: &LedgerStateSummary,
        _state_delta: &StateDelta,
        _proof_reader: Arc<dyn ProofRead>,
    ) -> Self {
        // FIXME(aldenhu)
        todo!()
    }
}

#[derive(Clone, Debug, Deref)]
pub struct StateWithSummary {
    #[deref]
    state: State,
    summary: StateSummary,
}

impl StateWithSummary {
    pub fn new_empty() -> Self {
        Self {
            state: State::new_empty(),
            summary: StateSummary::new_empty(),
        }
    }

    // FIXME(aldenhu): rename
    pub fn new_at_version(
        version: Option<Version>,
        global_state_root_hash: HashValue,
        state_usage: StateStorageUsage,
    ) -> Self {
        Self {
            state: State::new_empty_at_version(version, state_usage),
            summary: StateSummary::new_at_version(
                version,
                SparseMerkleTree::new(global_state_root_hash, state_usage),
            ),
        }
    }

    pub fn new(state: State, summary: StateSummary) -> Self {
        assert_eq!(state.next_version(), summary.next_version());
        Self { state, summary }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn summary(&self) -> &StateSummary {
        &self.summary
    }
}
