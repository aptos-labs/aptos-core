// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file defines state store buffered state that has been committed.

use crate::{
    metrics::{LATEST_CHECKPOINT_VERSION, OTHER_TIMERS_SECONDS},
    state_store::{state_snapshot_committer::StateSnapshotCommitter, CurrentState, StateDb},
};
use aptos_logger::info;
use aptos_metrics_core::TimerHelper;
use aptos_scratchpad::SmtAncestors;
use aptos_storage_interface::{
    db_ensure as ensure,
    state_authenticator::StateAuthenticator,
    state_delta::{InMemState, StateDelta},
    AptosDbError, Result,
};
use aptos_types::state_store::{state_value::StateValue, ShardedStateUpdates};
use std::{
    sync::{
        mpsc,
        mpsc::{Sender, SyncSender},
        Arc,
    },
    thread::JoinHandle,
};

pub(crate) const ASYNC_COMMIT_CHANNEL_BUFFER_SIZE: u64 = 1;
pub(crate) const TARGET_SNAPSHOT_INTERVAL_IN_VERSION: u64 = 100_000;

/// The in-memory buffered state that consists of two pieces:
/// `state_until_checkpoint`: The ready-to-commit data in range (last snapshot, latest checkpoint].
/// `state_after_checkpoint`: The pending data from the latest checkpoint(exclusive) until the
/// latest version committed, which has not reached the next checkpoint.
/// Since these are divided by the latest checkpoint, it is guaranteed
/// state_until_checkpoint.current = state_after_checkpoint.base, same for their versions.
#[derive(Debug)]
pub struct BufferedState {
    /// state after the latest checkpoint. The `current` is the latest speculative state.
    ///   n.b. this is an `Arc` shared with the StateStore so that merely querying the latest state
    ///        does not require locking the buffered state.
    state_after_checkpoint: CurrentState,
    committed_state: InMemState,
    committed_auth: StateAuthenticator,
    last_checkpoint_state: InMemState,
    last_checkpoint_auth: StateAuthenticator,
    latest_state: InMemState,
    latest_auth: StateAuthenticator,

    state_commit_sender: SyncSender<CommitMessage<Arc<StateDelta>>>,
    target_items: usize,
    join_handle: Option<JoinHandle<()>>,
}

pub(crate) enum CommitMessage<T> {
    Data(T),
    Sync(Sender<()>),
    Exit,
}

impl BufferedState {
    pub(crate) fn new(
        state_db: &Arc<StateDb>,
        state_after_checkpoint: StateDelta,
        target_items: usize,
    ) -> (Self, SmtAncestors<StateValue>, CurrentState) {
        let (state_commit_sender, state_commit_receiver) =
            mpsc::sync_channel(ASYNC_COMMIT_CHANNEL_BUFFER_SIZE as usize);
        let arc_state_db = Arc::clone(state_db);
        let smt_ancestors = SmtAncestors::new(state_after_checkpoint.base.clone());
        let smt_ancestors_clone = smt_ancestors.clone();
        // Create a new thread with receiver subscribing to state commit changes
        let join_handle = std::thread::Builder::new()
            .name("state-committer".to_string())
            .spawn(move || {
                let committer = StateSnapshotCommitter::new(
                    arc_state_db,
                    state_commit_receiver,
                    smt_ancestors_clone,
                );
                committer.run();
            })
            .expect("Failed to spawn state committer thread.");
        let current_state = CurrentState::new(state_after_checkpoint.clone());
        let myself = Self {
            state_until_checkpoint: None,
            state_after_checkpoint: current_state.clone(),
            state_commit_sender,
            target_items,
            // The join handle of the async state commit thread for graceful drop.
            join_handle: Some(join_handle),
        };
        myself.report_latest_committed_version();
        (myself, smt_ancestors, current_state)
    }

    pub fn current_state(&self) -> &StateAuthenticator {
        &self.state_after_checkpoint
    }

    pub fn current_checkpoint_version(&self) -> Option<Version> {
        self.state_after_checkpoint.base_version
    }

    /// This method checks whether a commit is needed based on the target_items value and the number of items in state_until_checkpoint.
    /// If a commit is needed, it sends a CommitMessage::Data message to the StateSnapshotCommitter thread to commit the data.
    /// If sync_commit is true, it also sends a CommitMessage::Sync message to ensure that the commit is completed before returning.
    fn maybe_commit(&mut self, sync_commit: bool) {
        if sync_commit {
            let (commit_sync_sender, commit_sync_receiver) = mpsc::channel();
            if let Some(to_commit) = self.state_until_checkpoint.take().map(Arc::from) {
                self.state_commit_sender
                    .send(CommitMessage::Data(to_commit))
                    .unwrap();
            }
            self.state_commit_sender
                .send(CommitMessage::Sync(commit_sync_sender))
                .unwrap();
            commit_sync_receiver.recv().unwrap(); // blocks until the to_commit is received.
        } else if self.state_until_checkpoint.is_some() {
            let take_out_to_commit = {
                let state_until_checkpoint =
                    self.state_until_checkpoint.as_ref().expect("Must exist");
                state_until_checkpoint
                    .updates_since_base
                    .shards
                    .iter()
                    .map(|shard| shard.len())
                    .sum::<usize>()
                    >= self.target_items
                    || state_until_checkpoint.current_version.map_or(0, |v| v + 1)
                        - state_until_checkpoint.base_version.map_or(0, |v| v + 1)
                        >= TARGET_SNAPSHOT_INTERVAL_IN_VERSION
            };
            if take_out_to_commit {
                let to_commit: Arc<StateDelta> = self
                    .state_until_checkpoint
                    .take()
                    .map(Arc::from)
                    .expect("Must exist");
                info!(
                    base_version = to_commit.base_version,
                    version = to_commit.current_version,
                    "Sent StateDelta to async commit thread."
                );
                self.state_commit_sender
                    .send(CommitMessage::Data(to_commit))
                    .unwrap();
            }
        }
    }

    pub(crate) fn sync_commit(&mut self) {
        self.maybe_commit(true /* sync_commit */);
    }

    fn report_latest_committed_version(&self) {
        LATEST_CHECKPOINT_VERSION.set(
            self.state_after_checkpoint
                .lock()
                .base_version
                .map_or(-1, |v| v as i64),
        );
    }

    /// This method updates the buffered state with new data.
    pub fn update(
        &mut self,
        updates_until_next_checkpoint_since_current_option: Option<&ShardedStateUpdates>,
        new_state_after_checkpoint: &StateDelta,
        sync_commit: bool,
    ) -> Result<()> {
        {
            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["update_current_state"]);
            let mut state_after_checkpoint = self.state_after_checkpoint.lock();

            assert!(new_state_after_checkpoint
                .current
                .is_family(&state_after_checkpoint.current));
            ensure!(
                new_state_after_checkpoint.base_version >= state_after_checkpoint.base_version,
                "new state base version smaller than state after checkpoint base version",
            );
            if let Some(updates_until_next_checkpoint_since_current) =
                updates_until_next_checkpoint_since_current_option
            {
                ensure!(
                    new_state_after_checkpoint.base_version > state_after_checkpoint.base_version,
                    "Diff between base and latest checkpoints provided, while they are the same.",
                );
                state_after_checkpoint
                    .updates_since_base
                    .clone_merge(updates_until_next_checkpoint_since_current);

                let mut old_state =
                    state_after_checkpoint.replace_with(new_state_after_checkpoint.clone());
                old_state.current = state_after_checkpoint.base.clone();
                old_state.current_version = state_after_checkpoint.base_version;

                if let Some(ref mut delta) = self.state_until_checkpoint {
                    delta.merge(old_state);
                } else {
                    self.state_until_checkpoint = Some(Box::new(old_state));
                }
            } else {
                ensure!(
                    new_state_after_checkpoint.base_version == state_after_checkpoint.base_version,
                    "Diff between base and latest checkpoints not provided.",
                );
                *state_after_checkpoint = new_state_after_checkpoint.clone();
            }
        }

        // n.b. make sure these are called after self.state_after_checkpoint is unlocked.
        //      otherwise things reading the "pre-committed version" will be blocked by
        //      the buffered state lock.
        self.maybe_commit(sync_commit);
        self.report_latest_committed_version();
        Ok(())
    }
}

impl Drop for BufferedState {
    fn drop(&mut self) {
        self.sync_commit();
        self.state_commit_sender.send(CommitMessage::Exit).unwrap();
        self.join_handle
            .take()
            .expect("snapshot commit thread must exist.")
            .join()
            .expect("snapshot commit thread should join peacefully.");
    }
}
