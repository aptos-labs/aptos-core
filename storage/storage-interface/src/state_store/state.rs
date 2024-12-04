// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::{state_delta::StateDelta, state_update::StateWrite, NUM_STATE_SHARDS};
use aptos_experimental_layered_map::MapLayer;
use aptos_types::{
    state_store::{state_key::StateKey, state_storage_usage::StateStorageUsage},
    transaction::Version,
};
use derive_more::Deref;
use std::sync::Arc;

/// Represents the blockchain state at a given version.
/// n.b. the state can be either persisted or speculative.
#[derive(Clone, Debug)]
pub struct State {
    /// The next version. If this is 0, the state is the "pre-genesis" empty state.
    next_version: Version,
    /// The updates made to the state at the current version.
    ///  N.b. this is not directly iteratable, one needs to make a `StateDelta`
    ///       between this and a `base_version` to list the updates or create a
    ///       new `State` at a descendant version.
    pub shards: Arc<[MapLayer<StateKey, StateWrite>; NUM_STATE_SHARDS]>,
    /// The total usage of the state at the current version.
    usage: StateStorageUsage,
}

impl State {
    pub fn new_empty() -> Self {
        Self {
            next_version: 0,
            shards: Arc::new(arr_macro::arr![MapLayer::new_family("state"); 16]),
            usage: StateStorageUsage::zero(),
        }
    }

    pub fn next_version(&self) -> Version {
        self.next_version
    }

    pub fn version(&self) -> Option<Version> {
        self.next_version.checked_sub(1)
    }

    pub fn usage(&self) -> StateStorageUsage {
        self.usage
    }

    pub fn shards(&self) -> &[MapLayer<StateKey, StateWrite>; NUM_STATE_SHARDS] {
        &self.shards
    }

    pub fn into_delta(self, _base: State) -> StateDelta {
        // FIXME(aldenhu)
        todo!()
    }

    pub fn make_delta(&self, _base: &State) -> StateDelta {
        // FIXME(aldenhu)
        todo!()
    }

    pub fn is_the_same(&self, _rhs: &Self) -> bool {
        // FIXME(aldenhu)
        todo!()
    }

    pub fn is_family(&self, _rhs: &State) -> bool {
        // FIXME(aldenhu)
        todo!()
    }

    pub fn follows(&self, _rhs: &State) -> bool {
        // FIXME(aldenhu)
        todo!()
    }

    pub fn count_items_heavy(&self) -> usize {
        // FIXME(aldenhu)
        todo!()
    }
}

/// At a given version, the state and the last checkpoint state at or before the version.
#[derive(Clone, Debug, Deref)]
pub struct LedgerState {
    last_checkpoint_state: State,
    #[deref]
    state: State,
}

impl LedgerState {
    pub fn new(last_checkpoint_state: State, state: State) -> Self {
        assert!(last_checkpoint_state.next_version() <= state.next_version());
        assert!(last_checkpoint_state.is_family(&state));

        Self {
            last_checkpoint_state,
            state,
        }
    }

    pub fn new_empty() -> Self {
        let state = State::new_empty();
        Self::new(state.clone(), state)
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn last_checkpoint_state(&self) -> &State {
        &self.last_checkpoint_state
    }

    pub fn is_checkpoint(&self) -> bool {
        self.state.is_the_same(&self.last_checkpoint_state)
    }
}
