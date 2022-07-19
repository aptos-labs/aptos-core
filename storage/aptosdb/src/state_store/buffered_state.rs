// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This file defines state store buffered state that has been committed.

use crate::{
    state_merkle_db::StateMerkleDb, state_store::state_snapshot_committer::StateSnapshotCommitter,
};
use anyhow::{ensure, Result};
use aptos_types::state_store::state_key::StateKey;
use aptos_types::state_store::state_value::StateValue;
use std::collections::HashMap;
use std::mem::swap;
use std::sync::mpsc::{Sender, SyncSender};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use storage_interface::state_delta::StateDelta;

const SNAPSHOT_SIZE_THRESHOLD: usize = 100_000;

/// The in-memory buffered state that consists of two pieces:
/// `state_until_checkpoint`: The ready-to-commit data from the last snapshot to the latest
/// /// checkpoint(inclusive).
/// `state_after_checkpoint`: The pending data from the latest checkpoint until the latest version
/// committed, which has not reached the next checkpoint.
/// Since these are divided by the latest checkpoint, it is guaranteed
/// state_until_checkpoint.current = state_after_checkpoint.base, same for their versions.
#[derive(Debug)]
pub struct BufferedState {
    // state until the latest checkpoint.
    state_until_checkpoint: Option<Box<StateDelta>>,
    // state after the latest checkpoint.
    state_after_checkpoint: StateDelta,
    state_commit_sender: SyncSender<(Option<Arc<StateDelta>>, Option<Sender<()>>)>,
    join_handle: Option<JoinHandle<()>>,
}

impl BufferedState {
    pub fn new(state_merkle_db: &Arc<StateMerkleDb>, state_after_checkpoint: StateDelta) -> Self {
        let (state_commit_sender, state_commit_receiver) = mpsc::sync_channel(1 /* bound */);
        let arc_state_merkle_db = Arc::clone(state_merkle_db);
        let join_handle = std::thread::Builder::new()
            .name("state_snapshot_committer".to_string())
            .spawn(move || {
                let committer =
                    StateSnapshotCommitter::new(arc_state_merkle_db, state_commit_receiver);
                committer.run();
            })
            .expect("Failed to spawn state committer thread.");
        Self {
            state_until_checkpoint: None,
            state_after_checkpoint,
            state_commit_sender,
            // The join handle of the async state commit thread for graceful drop.
            join_handle: Some(join_handle),
        }
    }

    pub fn current_state(&self) -> &StateDelta {
        &self.state_after_checkpoint
    }

    fn maybe_commit(&mut self, sync_commit: bool) {
        let to_commit = if self.state_until_checkpoint.is_some()
            && (sync_commit
                || self
                    .state_until_checkpoint
                    .as_ref()
                    .expect("Must exist")
                    .updates_since_base
                    .len()
                    >= SNAPSHOT_SIZE_THRESHOLD)
        {
            Some(Arc::from(
                self.state_until_checkpoint.take().expect("Must exist"),
            ))
        } else {
            None
        };
        if sync_commit {
            let (commit_sync_sender, commit_sync_receiver) = mpsc::channel();
            self.state_commit_sender
                .send((to_commit, Some(commit_sync_sender)))
                .unwrap();
            commit_sync_receiver.recv().unwrap();
        } else {
            self.state_commit_sender.send((to_commit, None)).unwrap();
        }
    }

    pub fn update(
        &mut self,
        updates_until_next_checkpoint_since_current_option: Option<HashMap<StateKey, StateValue>>,
        mut new_state_after_checkpoint: StateDelta,
        sync_commit: bool,
    ) -> Result<()> {
        ensure!(
            new_state_after_checkpoint.base_version >= self.state_after_checkpoint.base_version
        );
        if let Some(updates_until_next_checkpoint_since_current) =
            updates_until_next_checkpoint_since_current_option
        {
            self.state_after_checkpoint
                .updates_since_base
                .extend(updates_until_next_checkpoint_since_current);
            self.state_after_checkpoint.current = new_state_after_checkpoint.base.clone();
            self.state_after_checkpoint.current_version = new_state_after_checkpoint.base_version;
            swap(
                &mut self.state_after_checkpoint,
                &mut new_state_after_checkpoint,
            );
            if let Some(ref mut delta) = self.state_until_checkpoint {
                delta.merge(new_state_after_checkpoint);
            } else {
                self.state_until_checkpoint = Some(Box::new(new_state_after_checkpoint));
            }
        } else {
            ensure!(
                new_state_after_checkpoint.base_version == self.state_after_checkpoint.base_version
            );
            self.state_after_checkpoint = new_state_after_checkpoint;
        }
        self.maybe_commit(sync_commit);
        Ok(())
    }
}

impl Drop for BufferedState {
    fn drop(&mut self) {
        self.maybe_commit(true /* sync_commit */);
        self.state_commit_sender.send((None, None)).unwrap();
        self.join_handle
            .take()
            .expect("snapshot commit thread must exist.")
            .join()
            .expect("snapshot commit thread should join peacefully.");
    }
}
