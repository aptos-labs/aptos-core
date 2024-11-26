// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::{state_delta::StateDelta, state_update::StateWrite, NUM_STATE_SHARDS};
use aptos_experimental_layered_map::MapLayer;
use aptos_types::{
    state_store::{state_key::StateKey, state_storage_usage::StateStorageUsage},
    transaction::Version,
};
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
    shards: Arc<[MapLayer<StateKey, StateWrite>; NUM_STATE_SHARDS]>,
    /// The total usage of the state at the current version.
    usage: StateStorageUsage,
}

impl State {
    pub fn new_empty() -> Self {
        // FIXME(aldenhu): check call site and implement
        todo!()
    }

    pub fn next_version(&self) -> Version {
        self.next_version
    }

    pub fn usage(&self) -> StateStorageUsage {
        self.usage
    }

    pub fn shards(&self) -> &[MapLayer<StateKey, StateWrite>; NUM_STATE_SHARDS] {
        &self.shards
    }

    pub fn into_delta(self, _base: State) -> StateDelta {
        // FIXME(aldnehu)
        todo!()
    }
}
