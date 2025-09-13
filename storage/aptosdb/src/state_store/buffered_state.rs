// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file defines state store buffered state that has been committed.

use crate::{
    metrics::{LATEST_CHECKPOINT_VERSION, OTHER_TIMERS_SECONDS},
    state_store::{
        persisted_state::PersistedState, state_snapshot_committer::StateSnapshotCommitter, StateDb,
    },
};
use aptos_infallible::Mutex;
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::{
    state_store::state_with_summary::{LedgerStateWithSummary, StateWithSummary},
    Result,
};
use aptos_types::transaction::Version;
use std::{
    sync::{
        mpsc,
        mpsc::{Sender, SyncSender},
        Arc, MutexGuard,
    },
    thread::JoinHandle,
};

pub(crate) const ASYNC_COMMIT_CHANNEL_BUFFER_SIZE: u64 = 1;
pub(crate) const TARGET_SNAPSHOT_INTERVAL_IN_VERSION: u64 = 100_000;

/// BufferedState manages a range of recent state checkpoints and asynchronously commits
/// the updates in batches.
#[derive(Debug)]
pub struct BufferedState {
    /// the current state and the last checkpoint. shared with outside world.
    current_state: Arc<Mutex<LedgerStateWithSummary>>,
    /// The most recent checkpoint sent for persistence, not guaranteed to have committed already.
    last_snapshot: StateWithSummary,
    /// channel to send a checkpoint for persistence asynchronously
    state_commit_sender: SyncSender<CommitMessage<StateWithSummary>>,
    /// Estimated number of items in the buffer.
    estimated_items: usize,
    /// The target number of items in the buffer between commits.
    target_items: usize,
    join_handle: Option<JoinHandle<()>>,
}

pub(crate) enum CommitMessage<T> {
    Data(T),
    Sync(Sender<()>),
    Exit,
}

impl BufferedState {
    pub(crate) fn new_at_snapshot(
        state_db: &Arc<StateDb>,
        last_snapshot: StateWithSummary,
        target_items: usize,
        out_current_state: Arc<Mutex<LedgerStateWithSummary>>,
        out_persisted_state: PersistedState,
    ) -> Self {
        let (state_commit_sender, state_commit_receiver) =
            mpsc::sync_channel(ASYNC_COMMIT_CHANNEL_BUFFER_SIZE as usize);
        let arc_state_db = Arc::clone(state_db);
        *out_current_state.lock() =
            LedgerStateWithSummary::new_at_checkpoint(last_snapshot.clone());
        out_persisted_state.hack_reset(last_snapshot.clone());

        let persisted_state_clone = out_persisted_state.clone();
        let last_snapshot_clone = last_snapshot.clone();
        // Create a new thread with receiver subscribing to state commit changes
        let join_handle = std::thread::Builder::new()
            .name("state-committer".to_string())
            .spawn(move || {
                let committer = StateSnapshotCommitter::new(
                    arc_state_db,
                    state_commit_receiver,
                    last_snapshot_clone,
                    persisted_state_clone,
                );
                committer.run();
            })
            .expect("Failed to spawn state committer thread.");
        Self::report_last_checkpoint_version(last_snapshot.version());
        Self {
            current_state: out_current_state.clone(),
            last_snapshot,
            state_commit_sender,
            estimated_items: 0,
            target_items,
            // The join handle of the async state commit thread for graceful drop.
            join_handle: Some(join_handle),
        }
    }

    /// This method checks whether a commit is needed based on the target_items value and the number of items in state_until_checkpoint.
    /// If a commit is needed, it sends a CommitMessage::Data message to the StateSnapshotCommitter thread to commit the data.
    /// If sync_commit is true, it also sends a CommitMessage::Sync message to ensure that the commit is completed before returning.
    fn maybe_commit(&mut self, checkpoint: Option<StateWithSummary>, sync_commit: bool) {
        if let Some(checkpoint) = checkpoint {
            if !checkpoint.is_the_same(&self.last_snapshot)
                && (sync_commit
                    || self.estimated_items >= self.target_items
                    || self.buffered_versions() >= TARGET_SNAPSHOT_INTERVAL_IN_VERSION)
            {
                self.enqueue_commit(checkpoint);
            }
        }

        if sync_commit {
            self.drain_commits();
        }
    }

    fn current_state_locked(&self) -> MutexGuard<'_, LedgerStateWithSummary> {
        self.current_state.lock()
    }

    fn buffered_versions(&self) -> u64 {
        self.current_state_locked().next_version() - self.last_snapshot.next_version()
    }

    fn enqueue_commit(&mut self, checkpoint: StateWithSummary) {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["buffered_state___enqueue_commit"]);

        self.state_commit_sender
            .send(CommitMessage::Data(checkpoint.clone()))
            .unwrap();
        // n.b. if the latest state is not a (the latest) checkpoint, the items between them are
        // not counted towards the next commit. If this becomes a concern we can count the items
        // instead of putting it 0 here.
        self.estimated_items = 0;
        self.last_snapshot = checkpoint;
    }

    fn drain_commits(&mut self) {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["buffered_state___drain_commits"]);

        let (commit_sync_sender, commit_sync_receiver) = mpsc::channel();
        self.state_commit_sender
            .send(CommitMessage::Sync(commit_sync_sender))
            .unwrap();
        commit_sync_receiver.recv().unwrap();
    }

    pub(crate) fn sync_commit(&mut self) {
        let checkpoint = self.current_state_locked().last_checkpoint().clone();
        self.maybe_commit(Some(checkpoint), true /* sync_commit */);
    }

    fn report_last_checkpoint_version(version: Option<Version>) {
        LATEST_CHECKPOINT_VERSION.set(version.map_or(-1, |v| v as i64));
    }

    /// This method updates the buffered state with new data.
    pub fn update(
        &mut self,
        new_state: LedgerStateWithSummary,
        estimated_new_items: usize,
        sync_commit: bool,
    ) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["buffered_state___update"]);

        let old_state = self.current_state_locked().clone();
        assert!(new_state.is_descendant_of(&old_state));

        self.estimated_items += estimated_new_items;
        let version = new_state.last_checkpoint().version();

        let last_checkpoint = new_state.last_checkpoint().clone();
        // Commit state only if there is a new checkpoint, eases testing and make estimated
        // buffer size a tad more realistic.
        let checkpoint_to_commit_opt =
            (old_state.next_version() < last_checkpoint.next_version()).then_some(last_checkpoint);
        *self.current_state_locked() = new_state;
        self.maybe_commit(checkpoint_to_commit_opt, sync_commit);
        Self::report_last_checkpoint_version(version);
        Ok(())
    }

    pub(crate) fn quit(&mut self) {
        if let Some(handle) = self.join_handle.take() {
            self.sync_commit();
            self.state_commit_sender.send(CommitMessage::Exit).unwrap();
            handle
                .join()
                .expect("snapshot commit thread should join peacefully.");
        }
    }

    /// used by restore tooling
    pub(crate) fn force_last_snapshot(&mut self, snapshot: StateWithSummary) {
        self.last_snapshot = snapshot
    }
}

impl Drop for BufferedState {
    fn drop(&mut self) {
        self.quit()
    }
}
