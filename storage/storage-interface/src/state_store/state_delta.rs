// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::{
    sharded_state_updates::ShardedStateUpdates, state::State, state_update::StateWrite,
};
use aptos_crypto::HashValue;
use aptos_types::{
    state_store::{state_key::StateKey, state_storage_usage::StateStorageUsage},
    transaction::Version,
};

/// This represents two state sparse merkle trees at their versions in memory with the updates
/// reflecting the difference of `current` on top of `base`.
///
/// The `base` is the state SMT that current is based on.
/// The `current` is the state SMT that results from applying updates_since_base on top of `base`.
/// `updates_since_base` tracks all those key-value pairs that's changed since `base`, useful
///  when the next checkpoint is calculated.
#[derive(Clone, Debug)]
pub struct StateDelta {
    pub base: State,
    pub current: State,
    pub updates: ShardedStateUpdates,
}

impl StateDelta {
    pub fn new(_base: State, _current: State) -> Self {
        todo!()
        /* FIXME(aldenhu):
        assert!(base.is_family(&current));
        assert!(base_version.map_or(0, |v| v + 1) <= current_version.map_or(0, |v| v + 1));
        Self {
            base,
            base_version,
            current,
            current_version,
            updates_since_base: DropHelper::new(updates_since_base),
        }
         */
    }

    pub fn new_empty_with_version(_version: Option<u64>) -> StateDelta {
        /* FIXME(aldenhu):
        let smt = SparseMerkleTree::new_empty();
        Self::new(
            smt.clone(),
            version,
            smt,
            version,
            ShardedStateUpdates::new_empty(),
        )
         */
        todo!()
    }

    pub fn new_empty() -> Self {
        Self::new_empty_with_version(None)
    }

    pub fn new_at_checkpoint(
        _root_hash: HashValue,
        _usage: StateStorageUsage,
        _checkpoint_version: Option<Version>,
    ) -> Self {
        /* FIXME(aldenhu):
        let smt = SparseMerkleTree::new(root_hash, usage);
        Self::new(
            smt.clone(),
            checkpoint_version,
            smt,
            checkpoint_version,
            ShardedStateUpdates::new_empty(),
        )

         */
        todo!()
    }

    pub fn merge(&mut self, _other: StateDelta) {
        /* FIXME(aldenhu):
        assert!(other.follow(self));

        self.current = other.current;
        self.current_version = other.current_version;
        self.updates_since_base
            .merge(other.updates_since_base.into_inner());

         */
        todo!()
    }

    pub fn follow(&self, _other: &StateDelta) -> bool {
        /* FIXME(aldenhu):
        self.base_version == other.current_version && other.current.has_same_root_hash(&self.base)
         */
        todo!()
    }

    pub fn has_same_current_state(&self, _other: &StateDelta) -> bool {
        /* FIXME(aldenhu):
        self.current_version == other.current_version
            && self.current.has_same_root_hash(&other.current)
         */
        todo!()
    }

    pub fn next_version(&self) -> Version {
        self.current.next_version()
    }

    pub fn parent_version(&self) -> Option<Version> {
        self.base.next_version().checked_sub(1)
    }

    /// Get the state update for a given state key.
    /// `None` indicates the key is not updated in the delta.
    pub fn get_state_update(&self, _state_key: &StateKey) -> Option<&StateWrite> {
        // FIXME(aldenhu)
        todo!()
    }
}

impl Default for StateDelta {
    fn default() -> Self {
        Self::new_empty()
    }
}
