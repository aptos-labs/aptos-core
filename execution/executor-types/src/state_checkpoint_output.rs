// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use anyhow::{ensure, Result};
use aptos_crypto::HashValue;
use aptos_storage_interface::state_delta::StateDelta;
use aptos_types::state_store::ShardedStateUpdates;
use itertools::zip_eq;

// FIXME(aldenhu): remove Default?
#[derive(Debug, Default)]
pub struct StateCheckpointOutput {
    /// includes state updates between the last checkpoint version and the current version
    pub result_state: StateDelta,
    /// state updates between the base version and the last checkpoint version
    pub state_updates_before_last_checkpoint: Option<ShardedStateUpdates>,
    pub per_version_state_updates: Vec<ShardedStateUpdates>,
    pub state_checkpoint_hashes: Vec<Option<HashValue>>,
}

impl StateCheckpointOutput {
    pub fn new(
        result_state: StateDelta,
        state_updates_before_last_checkpoint: Option<ShardedStateUpdates>,
        state_checkpoint_hashes: Vec<Option<HashValue>>,
        per_version_state_updates: Vec<ShardedStateUpdates>,
    ) -> Self {
        Self {
            result_state,
            per_version_state_updates,
            state_checkpoint_hashes,
            state_updates_before_last_checkpoint,
        }
    }

    pub fn new_empty(parent_state: &StateDelta) -> Self {
        Self {
            result_state: parent_state.new_at_current(),
            // FIXME(aldenhu): is this right? try to comprehend "the tail"
            state_updates_before_last_checkpoint: None,
            per_version_state_updates: vec![],
            state_checkpoint_hashes: vec![],
        }
    }

    pub fn new_empty_following_this(&self) -> Self {
        Self::new_empty(&self.result_state)
    }

    pub fn check_and_update_state_checkpoint_hashes(
        &mut self,
        trusted_hashes: Vec<Option<HashValue>>,
    ) -> Result<()> {
        let len = self.state_checkpoint_hashes.len();
        ensure!(
            len == trusted_hashes.len(),
            "Number of txns doesn't match. self: {len}, trusted: {}",
            trusted_hashes.len()
        );

        zip_eq(
            self.state_checkpoint_hashes.iter_mut(),
            trusted_hashes.iter(),
        )
        .try_for_each(|(self_hash, trusted_hash)| {
            if self_hash.is_none() && trusted_hash.is_some() {
                *self_hash = *trusted_hash;
            } else {
                ensure!(self_hash == trusted_hash,
                    "State checkpoint hash doesn't match, self: {self_hash:?}, trusted: {trusted_hash:?}"
                );
            }
            Ok(())
        })
    }
}
