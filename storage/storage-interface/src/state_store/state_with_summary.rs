// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::state_store::{
    state::{HotStateMetadata, LedgerState, State},
    state_summary::{LedgerStateSummary, StateSummary},
};
use aptos_config::config::HotStateConfig;
use aptos_crypto::HashValue;
use aptos_scratchpad::SparseMerkleTree;
use aptos_types::{
    state_store::{state_storage_usage::StateStorageUsage, NUM_STATE_SHARDS},
    transaction::Version,
};
use derive_more::{Deref, DerefMut};

#[derive(Clone, Debug, Deref)]
pub struct StateAndSummary<S> {
    #[deref]
    state: S,
    summary: StateSummary,
}

impl<S> StateAndSummary<S> {
    pub fn new(state: S, summary: StateSummary) -> Self {
        Self { state, summary }
    }

    pub fn state(&self) -> &S {
        &self.state
    }

    pub fn summary(&self) -> &StateSummary {
        &self.summary
    }

    pub fn into_inner(self) -> (S, StateSummary) {
        let Self { state, summary } = self;
        (state, summary)
    }
}

pub type StateWithSummary = StateAndSummary<State>;

impl StateWithSummary {
    pub fn new_empty(hot_state_config: HotStateConfig) -> Self {
        Self::new(
            State::new_empty(hot_state_config),
            StateSummary::new_empty(hot_state_config),
        )
    }

    pub fn new_at_version(
        version: Option<Version>,
        hot_state_root_hash: HashValue,
        global_state_root_hash: HashValue,
        usage: StateStorageUsage,
        hot_state_config: HotStateConfig,
    ) -> Self {
        Self::new_at_version_with_hot_state_metadata(
            version,
            hot_state_root_hash,
            global_state_root_hash,
            usage,
            hot_state_config,
            Default::default(),
        )
    }

    pub fn new_at_version_with_hot_state_metadata(
        version: Option<Version>,
        hot_state_root_hash: HashValue,
        global_state_root_hash: HashValue,
        usage: StateStorageUsage,
        hot_state_config: HotStateConfig,
        hot_state_metadata: [HotStateMetadata; NUM_STATE_SHARDS],
    ) -> Self {
        Self::new(
            State::new_at_version_with_hot_state_metadata(
                version,
                usage,
                hot_state_config,
                hot_state_metadata,
            ),
            StateSummary::new_at_version(
                version,
                SparseMerkleTree::new(hot_state_root_hash),
                SparseMerkleTree::new(global_state_root_hash),
                hot_state_config,
            ),
        )
    }

    pub fn is_descendant_of(&self, other: &Self) -> bool {
        self.state().is_descendant_of(other.state())
            && self.summary().is_descendant_of(other.summary())
    }
}

#[derive(Clone, Debug, Deref, DerefMut)]
pub struct LedgerWithSummary<W: Clone> {
    #[deref]
    #[deref_mut]
    latest: W,
    last_checkpoint: W,
}

impl<W: Clone> LedgerWithSummary<W> {
    pub fn from_latest_and_last_checkpoint(latest: W, last_checkpoint: W) -> Self {
        Self {
            latest,
            last_checkpoint,
        }
    }

    pub fn new_at_checkpoint(checkpoint: W) -> Self {
        Self::from_latest_and_last_checkpoint(checkpoint.clone(), checkpoint)
    }

    pub fn latest(&self) -> &W {
        &self.latest
    }

    pub fn last_checkpoint(&self) -> &W {
        &self.last_checkpoint
    }
}

pub type LedgerStateWithSummary = LedgerWithSummary<StateWithSummary>;

impl LedgerStateWithSummary {
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
        self.latest().next_version() == self.last_checkpoint().next_version()
    }

    pub fn ledger_state(&self) -> LedgerState {
        LedgerState::new(
            self.latest().state().clone(),
            self.last_checkpoint().state().clone(),
        )
    }

    pub fn ledger_state_summary(&self) -> LedgerStateSummary {
        LedgerStateSummary::new(
            self.last_checkpoint().summary().clone(),
            self.latest().summary().clone(),
        )
    }

    pub fn to_state_and_summary(&self) -> (LedgerState, LedgerStateSummary) {
        (self.ledger_state(), self.ledger_state_summary())
    }

    pub fn is_descendant_of(&self, other: &Self) -> bool {
        self.latest().is_descendant_of(other.latest())
            && self
                .last_checkpoint()
                .is_descendant_of(other.last_checkpoint())
    }
}
