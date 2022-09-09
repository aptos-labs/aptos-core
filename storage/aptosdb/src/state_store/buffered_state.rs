// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This file defines state store buffered state that has been committed.

use crate::{
    metrics::LATEST_CHECKPOINT_VERSION,
    state_store::{state_snapshot_committer::StateSnapshotCommitter, StateDb},
};
use anyhow::{ensure, Result};
use aptos_logger::info;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use std::{
    collections::{HashMap, VecDeque},
    mem::swap,
    sync::{
        mpsc,
        mpsc::{Receiver, Sender, SyncSender},
        Arc,
    },
    thread::JoinHandle,
};
use storage_interface::state_delta::StateDelta;

pub(crate) const ASYNC_COMMIT_CHANNEL_BUFFER_SIZE: u64 = 1;
pub(crate) const TARGET_SNAPSHOT_INTERVAL_IN_VERSION: u64 = 20_000;

/// The in-memory buffered state that consists of two pieces:
/// `state_until_checkpoint`: The ready-to-commit data in range (last snapshot, latest checkpoint].
/// `state_after_checkpoint`: The pending data from the latest checkpoint(exclusive) until the
/// latest version committed, which has not reached the next checkpoint.
/// Since these are divided by the latest checkpoint, it is guaranteed
/// state_until_checkpoint.current = state_after_checkpoint.base, same for their versions.
#[derive(Debug)]
pub struct BufferedState {
    // state until the latest checkpoint.
    state_until_checkpoint: Option<Box<StateDelta>>,
    // state after the latest checkpoint.
    state_after_checkpoint: StateDelta,
    state_commit_sender: SyncSender<CommitMessage<Arc<StateDelta>>>,
    target_snapshot_size: usize,
    snapshot_ready_receivers: VecDeque<Receiver<()>>,
    join_handle: Option<JoinHandle<()>>,
}

pub(crate) enum CommitMessage<T> {
    Data {
        data: T,
        prev_snapshot_ready_receiver: Option<Receiver<()>>,
        snapshot_ready_sender: Sender<()>,
    },
    Sync(Sender<()>),
    Exit,
}

impl BufferedState {
    pub(crate) fn new(
        state_db: &Arc<StateDb>,
        state_after_checkpoint: StateDelta,
        target_snapshot_size: usize,
    ) -> Self {
        let (state_commit_sender, state_commit_receiver) =
            mpsc::sync_channel(ASYNC_COMMIT_CHANNEL_BUFFER_SIZE as usize);
        let arc_state_db = Arc::clone(state_db);
        let (initial_snapshot_ready_sender, initial_snapshot_ready_receiver) = mpsc::channel();
        let join_handle = std::thread::Builder::new()
            .name("state-committer".to_string())
            .spawn(move || {
                let committer = StateSnapshotCommitter::new(arc_state_db, state_commit_receiver);
                committer.run();
            })
            .expect("Failed to spawn state committer thread.");
        // The initial snapshot is always already persisted in db.
        initial_snapshot_ready_sender.send(()).unwrap();
        let myself = Self {
            state_until_checkpoint: None,
            state_after_checkpoint,
            state_commit_sender,
            target_snapshot_size,
            snapshot_ready_receivers: VecDeque::from([initial_snapshot_ready_receiver]),
            // The join handle of the async state commit thread for graceful drop.
            join_handle: Some(join_handle),
        };
        myself.report_latest_committed_version();
        myself
    }

    pub fn current_state(&self) -> &StateDelta {
        &self.state_after_checkpoint
    }

    pub fn current_checkpoint_version(&self) -> Option<Version> {
        self.state_after_checkpoint.base_version
    }

    fn send_to_commit(&mut self, to_commit: Arc<StateDelta>) {
        let prev_snapshot_ready_receiver = self
            .snapshot_ready_receivers
            .pop_front()
            .expect("receivers should never be empty");
        assert!(self.snapshot_ready_receivers.is_empty());
        let (snapshot_ready_sender, snapshot_ready_receiver) = mpsc::channel();
        self.snapshot_ready_receivers
            .push_back(snapshot_ready_receiver);
        self.state_commit_sender
            .send(CommitMessage::Data {
                data: to_commit,
                prev_snapshot_ready_receiver: Some(prev_snapshot_ready_receiver),
                snapshot_ready_sender,
            })
            .unwrap();
    }

    fn maybe_commit(&mut self, sync_commit: bool) {
        if sync_commit {
            let (commit_sync_sender, commit_sync_receiver) = mpsc::channel();
            if let Some(to_commit) = self.state_until_checkpoint.take().map(Arc::from) {
                self.send_to_commit(to_commit);
            }
            self.state_commit_sender
                .send(CommitMessage::Sync(commit_sync_sender))
                .unwrap();
            commit_sync_receiver.recv().unwrap();
        } else if self.state_until_checkpoint.is_some() {
            let take_out_to_commit = {
                let state_until_checkpoint =
                    self.state_until_checkpoint.as_ref().expect("Must exist");
                state_until_checkpoint.updates_since_base.len() >= self.target_snapshot_size
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
                self.send_to_commit(to_commit);
            }
        }
    }

    pub(crate) fn sync_commit(&mut self) {
        self.maybe_commit(true /* sync_commit */);
    }

    fn report_latest_committed_version(&self) {
        LATEST_CHECKPOINT_VERSION.set(
            self.state_after_checkpoint
                .base_version
                .map_or(-1, |v| v as i64),
        );
    }

    pub fn update(
        &mut self,
        updates_until_next_checkpoint_since_current_option: Option<
            HashMap<StateKey, Option<StateValue>>,
        >,
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
