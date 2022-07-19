// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This file defines the state snapshot committer running in background thread within StateStore.

use crate::state_merkle_db::StateMerkleDb;
use aptos_logger::trace;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use storage_interface::state_delta::StateDelta;
use storage_interface::{jmt_update_refs, jmt_updates};

pub(crate) struct StateSnapshotCommitter {
    state_merkle_db: Arc<StateMerkleDb>,
    state_commit_receiver: Receiver<(Option<Arc<StateDelta>>, Option<Sender<()>>)>,
}

impl StateSnapshotCommitter {
    pub fn new(
        state_merkle_db: Arc<StateMerkleDb>,
        state_commit_receiver: Receiver<(Option<Arc<StateDelta>>, Option<Sender<()>>)>,
    ) -> Self {
        Self {
            state_merkle_db,
            state_commit_receiver,
        }
    }

    pub fn run(self) {
        while let Ok((delta_to_commit_option, finish_sender_option)) =
            self.state_commit_receiver.recv()
        {
            // We use (None, None) as exit signal.
            if delta_to_commit_option.is_none() && finish_sender_option.is_none() {
                break;
            }

            if let Some(delta_to_commit) = delta_to_commit_option {
                let node_hashes = delta_to_commit
                    .current
                    .clone()
                    .freeze()
                    .new_node_hashes_since(&delta_to_commit.base.clone().freeze());
                let version = delta_to_commit.current_version.expect("Cannot be empty");
                let base_version = delta_to_commit.base_version;
                let root_hash = self
                    .state_merkle_db
                    .merklize_value_set(
                        jmt_update_refs(&jmt_updates(&delta_to_commit.updates_since_base)),
                        Some(&node_hashes),
                        version,
                        base_version,
                    )
                    .expect("Error writing snapshot");
                trace!(
                    version = version,
                    base_version = base_version,
                    root_hash = root_hash,
                    "State snapshot committed."
                );
            }
            if let Some(sender) = finish_sender_option {
                sender.send(()).unwrap()
            }
        }
        trace!("State snapshot committing thread exit.")
    }
}
