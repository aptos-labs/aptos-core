// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_scratchpad::SparseMerkleTree;
use aptos_types::{
    state_store::{
        combine_sharded_state_updates, create_empty_sharded_state_updates,
        state_storage_usage::StateStorageUsage, state_value::StateValue, ShardedStateUpdates,
    },
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
    pub base: SparseMerkleTree<StateValue>,
    pub base_version: Option<Version>,
    pub current: SparseMerkleTree<StateValue>,
    pub current_version: Option<Version>,
    pub updates_since_base: ShardedStateUpdates,
}

impl StateDelta {
    pub fn new(
        base: SparseMerkleTree<StateValue>,
        base_version: Option<Version>,
        current: SparseMerkleTree<StateValue>,
        current_version: Option<Version>,
        updates_since_base: ShardedStateUpdates,
    ) -> Self {
        assert!(base.is_family(&current));
        assert!(base_version.map_or(0, |v| v + 1) <= current_version.map_or(0, |v| v + 1));
        Self {
            base,
            base_version,
            current,
            current_version,
            updates_since_base,
        }
    }

    pub fn new_empty() -> Self {
        let smt = SparseMerkleTree::new_empty();
        Self::new(
            smt.clone(),
            None,
            smt,
            None,
            create_empty_sharded_state_updates(),
        )
    }

    pub fn new_at_checkpoint(
        root_hash: HashValue,
        usage: StateStorageUsage,
        checkpoint_version: Option<Version>,
    ) -> Self {
        let smt = SparseMerkleTree::new(root_hash, usage);
        Self::new(
            smt.clone(),
            checkpoint_version,
            smt,
            checkpoint_version,
            create_empty_sharded_state_updates(),
        )
    }

    pub fn merge(&mut self, other: StateDelta) {
        assert!(other.follow(self));
        combine_sharded_state_updates(&mut self.updates_since_base, other.updates_since_base);

        self.current = other.current;
        self.current_version = other.current_version;
    }

    pub fn follow(&self, other: &StateDelta) -> bool {
        self.base_version == other.current_version && other.current.has_same_root_hash(&self.base)
    }

    pub fn has_same_current_state(&self, other: &StateDelta) -> bool {
        self.current_version == other.current_version
            && self.current.has_same_root_hash(&other.current)
    }

    pub fn base_root_hash(&self) -> HashValue {
        self.base.root_hash()
    }

    pub fn root_hash(&self) -> HashValue {
        self.current.root_hash()
    }

    pub fn next_version(&self) -> Version {
        self.current_version.map_or(0, |v| v + 1)
    }

    pub fn replace_with(&mut self, mut rhs: Self) -> Self {
        std::mem::swap(self, &mut rhs);
        rhs
    }
}

impl Default for StateDelta {
    fn default() -> Self {
        Self::new_empty()
    }
}
