// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{ensure, Result};
use aptos_crypto::HashValue;
use aptos_executor_types::{
    execution_output::ExecutionOutput, state_checkpoint_output::StateCheckpointOutput,
};
use aptos_storage_interface::{state_authenticator::StateAuthenticator, state_delta::InMemState};

pub struct DoStateCheckpoint;

impl DoStateCheckpoint {
    pub fn run(
        execution_output: &ExecutionOutput,
        parent_auth: &StateAuthenticator,
        persisted_auth: &StateAuthenticator,
        known_state_checkpoints: Option<impl IntoIterator<Item = Option<HashValue>>>,
    ) -> Result<StateCheckpointOutput> {
        let state = &execution_output.result_state;
        let parent_state = &execution_output.parent_state;
        let last_checkpoint_state = execution_output.last_checkpoint_state.as_ref();

        let last_checkpoint_auth: Option<_> = last_checkpoint_state.map(|state| {
            let updates = state.clone().into_delta(parent_state.clone());
            parent_auth.update(persisted_auth, &updates)
        });

        let state_auth = if Some(execution_output.next_version())
            == last_checkpoint_state.map(|s| s.next_version())
        {
            last_checkpoint_auth.as_ref().unwrap().clone()
        } else {
            let base = last_checkpoint_state.unwrap_or(&parent_state);
            let updates = state.clone().into_delta(base.clone());
            parent_auth.update(persisted_auth, &updates)
        };

        let num_txns = execution_output.num_transactions_to_commit();
        let mut state_checkpoint_hashes = known_state_checkpoints
            .map_or_else(|| vec![None; num_txns], |v| v.into_iter().collect());
        ensure!(
            state_checkpoint_hashes.len() == num_txns,
            "Bad number of known hashes."
        );
        if let Some(auth) = &last_checkpoint_auth {
            let index = auth.next_version() - parent_auth.next_version() - 1;
            if let Some(h) = state_checkpoint_hashes[index] {
                ensure!(h == auth.root_hash(), "Last checkpoint not expected.");
            } else {
                state_checkpoint_hashes[index] = Some(auth.root_hash());
            }
        }

        assert_eq!(
            last_checkpoint_state.as_ref().map(InMemState::next_version),
            last_checkpoint_auth
                .as_ref()
                .map(StateAuthenticator::next_version)
        );
        assert_eq!(state.next_version(), state_auth.next_version());

        Ok(StateCheckpointOutput::new(
            parent_auth.clone(),
            last_checkpoint_auth,
            state_auth,
            state_checkpoint_hashes,
        ))
    }
}
