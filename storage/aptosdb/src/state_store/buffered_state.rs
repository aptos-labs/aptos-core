// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file defines state store buffered state that has been committed.

// FIXME(aldenhu)
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use crate::{
    metrics::{LATEST_CHECKPOINT_VERSION, OTHER_TIMERS_SECONDS},
    state_store::{
        persisted_state::PersistedState, state_snapshot_committer::StateSnapshotCommitter,
        CurrentState, StateDb,
    },
};
use aptos_infallible::Mutex;
use aptos_logger::info;
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::{
    db_ensure as ensure,
    state_store::{
        sharded_state_updates::ShardedStateUpdates,
        state::State,
        state_delta::StateDelta,
        state_summary::{LedgerStateSummary, StateWithSummary},
    },
    AptosDbError, Result,
};
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

/// BufferedState manages a range of recent state checkpoints and asynchronously commits
/// the updates in batches.
#[derive(Debug)]
pub struct BufferedState {
    /// Needed for estimating the size of the buffer by counting the diff between this and a later
    /// checkpoint added via `update()`
    latest_checkpoint: StateWithSummary,
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
    pub(crate) fn new(
        state_db: &Arc<StateDb>,
        last_snapshot: StateWithSummary,
        target_items: usize,
        persisted_state: Arc<Mutex<PersistedState>>,
    ) -> Self {
        let (state_commit_sender, state_commit_receiver) =
            mpsc::sync_channel(ASYNC_COMMIT_CHANNEL_BUFFER_SIZE as usize);
        let arc_state_db = Arc::clone(state_db);
        persisted_state.lock().set(last_snapshot.clone());
        let persisted_state_clone = persisted_state.clone();
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
        let myself = Self {
            latest_checkpoint: last_snapshot.clone(),
            last_snapshot,
            state_commit_sender,
            estimated_items: 0,
            target_items,
            // The join handle of the async state commit thread for graceful drop.
            join_handle: Some(join_handle),
        };
        myself.report_latest_committed_version();
        myself
    }

    /// This method checks whether a commit is needed based on the target_items value and the number of items in state_until_checkpoint.
    /// If a commit is needed, it sends a CommitMessage::Data message to the StateSnapshotCommitter thread to commit the data.
    /// If sync_commit is true, it also sends a CommitMessage::Sync message to ensure that the commit is completed before returning.
    fn maybe_commit(&mut self, sync_commit: bool) {
        if sync_commit
            || self.estimated_items >= self.target_items
            || self.buffered_versions() >= TARGET_SNAPSHOT_INTERVAL_IN_VERSION
        {
            self.enqueue_commit();
        }
        if sync_commit {
            self.drain_commits();
        }
    }

    fn buffered_versions(&self) -> u64 {
        self.latest_checkpoint.next_version() - self.last_snapshot.next_version()
    }

    fn enqueue_commit(&mut self) {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["buffered_state___enqueue_commit"]);

        self.state_commit_sender
            .send(CommitMessage::Data(self.latest_checkpoint.clone()))
            .unwrap();
        self.estimated_items = 0;
        self.last_snapshot = self.latest_checkpoint.clone();
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
        self.maybe_commit(true /* sync_commit */);
    }

    fn report_latest_committed_version(&self) {
        LATEST_CHECKPOINT_VERSION.set(self.latest_checkpoint.version().map_or(-1, |v| v as i64));
    }

    /// This method updates the buffered state with new data.
    pub fn update(&mut self, latest_checkpoint: StateWithSummary, sync_commit: bool) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["buffered_state___update"]);

        assert!(latest_checkpoint.follows(&self.latest_checkpoint));
        self.estimated_items += {
            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["buffered_state___count_items_heavy"]);
            latest_checkpoint
                .make_delta(&self.latest_checkpoint)
                .count_items_heavy()
        };
        self.latest_checkpoint = latest_checkpoint;

        self.maybe_commit(sync_commit);
        self.report_latest_committed_version();
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
}

impl Drop for BufferedState {
    fn drop(&mut self) {
        self.quit()
    }
}
