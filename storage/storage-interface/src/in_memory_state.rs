// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use scratchpad::SparseMerkleTree;
use std::collections::HashSet;

/// This represents the state at a certain version in memory.
///
/// The `checkpoint` is deemed persisted in DB.
/// The `current` is in memory only.
/// `updated_since_checkpoint` tracks all those keys that's changed since the checkpoint, useful
///   when the next checkpoint is calculated (the state values are assumed to be on the SMT nodes,
///   so only the keys are tracked).
#[derive(Clone, Debug)]
pub struct InMemoryState {
    pub checkpoint: SparseMerkleTree<StateValue>,
    pub checkpoint_version: Option<Version>,
    pub current: SparseMerkleTree<StateValue>,
    pub current_version: Option<Version>,
    pub updated_since_checkpoint: HashSet<StateKey>,
}

impl PartialEq for InMemoryState {
    fn eq(&self, other: &InMemoryState) -> bool {
        self.current_version == other.current_version && self.current == other.current
    }
}

impl Eq for InMemoryState {}

impl InMemoryState {
    pub fn new(
        checkpoint: SparseMerkleTree<StateValue>,
        checkpoint_version: Option<Version>,
        current: SparseMerkleTree<StateValue>,
        current_version: Option<Version>,
        updated_since_checkpoint: HashSet<StateKey>,
    ) -> Self {
        assert!(checkpoint_version.map_or(0, |v| v + 1) <= current_version.map_or(0, |v| v + 1));
        Self {
            checkpoint,
            checkpoint_version,
            current,
            current_version,
            updated_since_checkpoint,
        }
    }

    pub fn new_empty() -> Self {
        let smt = SparseMerkleTree::new_empty();
        Self::new(smt.clone(), None, smt, None, HashSet::new())
    }

    pub fn new_at_checkpoint(root_hash: HashValue, checkpoint_version: Option<Version>) -> Self {
        let smt = SparseMerkleTree::new(root_hash);
        Self::new(
            smt.clone(),
            checkpoint_version,
            smt,
            checkpoint_version,
            HashSet::new(),
        )
    }

    pub fn checkpoint_root_hash(&self) -> HashValue {
        self.checkpoint.root_hash()
    }

    pub fn root_hash(&self) -> HashValue {
        self.current.root_hash()
    }
}
