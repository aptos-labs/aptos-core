// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::HashValue;
use aptos_types::state_store::{state_key::StateKey, state_slot::StateSlot};

/// A view into the hot state store, whose content overlays on top of the cold state store content.
pub trait HotStateView: Send + Sync {
    fn get_state_slot(&self, state_key: &StateKey) -> Option<StateSlot>;

    /// Look up a hot state entry by the key's crypto hash. Returns both the `StateKey` and
    /// `StateSlot` so the LRU can follow prev/next pointers without having the full key upfront.
    fn get_state_entry_by_hash(
        &self,
        shard_id: usize,
        key_hash: &HashValue,
    ) -> Option<(StateKey, StateSlot)>;
}

pub struct EmptyHotState;

impl HotStateView for EmptyHotState {
    fn get_state_slot(&self, _state_key: &StateKey) -> Option<StateSlot> {
        None
    }

    fn get_state_entry_by_hash(
        &self,
        _shard_id: usize,
        _key_hash: &HashValue,
    ) -> Option<(StateKey, StateSlot)> {
        None
    }
}
