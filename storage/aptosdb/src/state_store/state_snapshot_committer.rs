// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file defines the state snapshot committer running in background thread within StateStore.

use crate::{
    state_store::{
        buffered_state::CommitMessage,
        state_merkle_batch_committer::{StateMerkleBatch, StateMerkleBatchCommitter},
        StateDb,
    },
    versioned_node_cache::VersionedNodeCache,
};
use aptos_logger::trace;
use aptos_storage_interface::{jmt_update_refs, jmt_updates, state_delta::StateDelta};
use static_assertions::const_assert;
use std::{
    sync::{
        mpsc,
        mpsc::{Receiver, SyncSender},
        Arc,
    },
    thread::JoinHandle,
};

pub(crate) struct StateSnapshotCommitter {
    state_db: Arc<StateDb>,
    state_snapshot_commit_receiver: Receiver<CommitMessage<Arc<StateDelta>>>,
    state_merkle_batch_commit_sender: SyncSender<CommitMessage<StateMerkleBatch>>,
    join_handle: Option<JoinHandle<()>>,
}

impl StateSnapshotCommitter {
    const CHANNEL_SIZE: usize = 0;

    pub fn new(
        state_db: Arc<StateDb>,
        state_snapshot_commit_receiver: Receiver<CommitMessage<Arc<StateDelta>>>,
    ) -> Self {
        // Note: This is to ensure we cache nodes in memory from previous batches before they get committed to DB.
        const_assert!(
            StateSnapshotCommitter::CHANNEL_SIZE < VersionedNodeCache::NUM_VERSIONS_TO_CACHE
        );
        // Rendezvous channel
        let (state_merkle_batch_commit_sender, state_merkle_batch_commit_receiver) =
            mpsc::sync_channel(Self::CHANNEL_SIZE);
        let arc_state_db = Arc::clone(&state_db);
        let join_handle = std::thread::Builder::new()
            .name("state_batch_committer".to_string())
            .spawn(move || {
                let committer = StateMerkleBatchCommitter::new(
                    arc_state_db,
                    state_merkle_batch_commit_receiver,
                );
                committer.run();
            })
            .expect("Failed to spawn state merkle batch committer thread.");
        Self {
            state_db,
            state_snapshot_commit_receiver,
            state_merkle_batch_commit_sender,
            join_handle: Some(join_handle),
        }
    }

    pub fn run(self) {
        while let Ok(msg) = self.state_snapshot_commit_receiver.recv() {
            match msg {
                CommitMessage::Data(delta_to_commit) => {
                    let node_hashes = delta_to_commit
                        .current
                        .clone()
                        .freeze()
                        .new_node_hashes_since(&delta_to_commit.base.clone().freeze());
                    let version = delta_to_commit.current_version.expect("Cannot be empty");
                    let base_version = delta_to_commit.base_version;

                    let (top_levels_batch, sharded_batch, root_hash) = self
                        .state_db
                        .state_merkle_db
                        // TODO(grao): Provide a sharded version of this function.
                        .merklize_value_set(
                            jmt_update_refs(&jmt_updates(
                                &delta_to_commit
                                    .updates_since_base
                                    .iter()
                                    .flatten()
                                    .map(|(k, v)| (k, v.as_ref()))
                                    .collect(),
                            )),
                            Some(&node_hashes),
                            version,
                            base_version,
                            self.state_db
                                .get_previous_epoch_ending(version)
                                .unwrap()
                                .map(|(v, _e)| v),
                        )
                        .expect("Error writing snapshot");
                    self.state_merkle_batch_commit_sender
                        .send(CommitMessage::Data(StateMerkleBatch {
                            top_levels_batch,
                            sharded_batch,
                            root_hash,
                            state_delta: delta_to_commit,
                        }))
                        .unwrap();
                },
                CommitMessage::Sync(finish_sender) => {
                    self.state_merkle_batch_commit_sender
                        .send(CommitMessage::Sync(finish_sender))
                        .unwrap();
                },
                CommitMessage::Exit => {
                    self.state_merkle_batch_commit_sender
                        .send(CommitMessage::Exit)
                        .unwrap();
                    break;
                },
            }
            trace!("State snapshot committing thread exit.")
        }
    }
}

impl Drop for StateSnapshotCommitter {
    fn drop(&mut self) {
        self.join_handle
            .take()
            .expect("state merkle batch commit thread must exist.")
            .join()
            .expect("state merkle batch thread should join peacefully.");
    }
}
