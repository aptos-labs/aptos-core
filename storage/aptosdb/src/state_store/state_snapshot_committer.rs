// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This file defines the state snapshot committer running in background thread within StateStore.

use crate::state_merkle_db::StateMerkleDb;
use crate::state_store::buffered_state::{CommitMessage, ASYNC_COMMIT_CHANNEL_BUFFER_SIZE};
use crate::state_store::state_merkle_batch_committer::{
    StateMerkleBatch, StateMerkleBatchCommitter,
};
use aptos_logger::trace;
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use storage_interface::state_delta::StateDelta;
use storage_interface::{jmt_update_refs, jmt_updates};

pub(crate) struct StateSnapshotCommitter {
    state_merkle_db: Arc<StateMerkleDb>,
    state_snapshot_commit_receiver: Receiver<CommitMessage<Arc<StateDelta>>>,
    state_merkle_batch_commit_sender: SyncSender<CommitMessage<StateMerkleBatch>>,
    join_handle: Option<JoinHandle<()>>,
}

impl StateSnapshotCommitter {
    pub fn new(
        state_merkle_db: Arc<StateMerkleDb>,
        state_snapshot_commit_receiver: Receiver<CommitMessage<Arc<StateDelta>>>,
    ) -> Self {
        let (state_merkle_batch_commit_sender, state_merkle_batch_commit_receiver) =
            mpsc::sync_channel(ASYNC_COMMIT_CHANNEL_BUFFER_SIZE as usize);
        let arc_state_merkle_db = Arc::clone(&state_merkle_db);
        let join_handle = std::thread::Builder::new()
            .name("state_merkle_batch_committer".to_string())
            .spawn(move || {
                let committer = StateMerkleBatchCommitter::new(
                    arc_state_merkle_db,
                    state_merkle_batch_commit_receiver,
                );
                committer.run();
            })
            .expect("Failed to spawn state merkle batch committer thread.");
        Self {
            state_merkle_db,
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
                    let (batch, root_hash) = self
                        .state_merkle_db
                        .merklize_value_set(
                            jmt_update_refs(&jmt_updates(&delta_to_commit.updates_since_base)),
                            Some(&node_hashes),
                            version,
                            base_version,
                        )
                        .expect("Error writing snapshot");
                    self.state_merkle_batch_commit_sender
                        .send(CommitMessage::Data(StateMerkleBatch {
                            batch,
                            root_hash,
                            state_delta: delta_to_commit,
                        }))
                        .unwrap();
                }
                CommitMessage::Sync(finish_sender) => {
                    self.state_merkle_batch_commit_sender
                        .send(CommitMessage::Sync(finish_sender))
                        .unwrap();
                }
                CommitMessage::Exit => {
                    self.state_merkle_batch_commit_sender
                        .send(CommitMessage::Exit)
                        .unwrap();
                    break;
                }
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
