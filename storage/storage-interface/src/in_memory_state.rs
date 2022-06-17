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
    pub updated_since_checkpoint: HashSet<StateKey>,
}

impl InMemoryState {
    pub fn new(
        checkpoint: SparseMerkleTree<StateValue>,
        checkpoint_version: Option<Version>,
        current: SparseMerkleTree<StateValue>,
        updated_since_checkpoint: HashSet<StateKey>,
    ) -> Self {
        Self {
            checkpoint,
            checkpoint_version,
            current,
            updated_since_checkpoint,
        }
    }

    pub fn new_empty() -> Self {
        let smt = SparseMerkleTree::new_empty();
        Self::new(smt.clone(), None, smt, HashSet::new())
    }

    pub fn new_at_checkpoint(
        smt: SparseMerkleTree<StateValue>,
        checkpoint_version: Option<Version>,
    ) -> Self {
        Self::new(smt.clone(), checkpoint_version, smt, HashSet::new())
    }

    pub fn checkpoint_root_hash(&self) -> HashValue {
        self.checkpoint.root_hash()
    }

    pub fn root_hash(&self) -> HashValue {
        self.current.root_hash()
    }
}
