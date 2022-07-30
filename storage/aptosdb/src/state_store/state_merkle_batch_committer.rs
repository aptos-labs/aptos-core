// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This file defines the state merkle snapshot committer running in background thread.

use crate::metrics::LATEST_SNAPSHOT_VERSION;
use crate::state_merkle_db::StateMerkleDb;
use crate::state_store::buffered_state::CommitMessage;
use crate::OTHER_TIMERS_SECONDS;
use aptos_crypto::HashValue;
use aptos_logger::trace;
use schemadb::SchemaBatch;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use storage_interface::state_delta::StateDelta;

pub struct StateMerkleBatch {
    pub batch: SchemaBatch,
    pub root_hash: HashValue,
    pub state_delta: Arc<StateDelta>,
}

pub(crate) struct StateMerkleBatchCommitter {
    state_merkle_db: Arc<StateMerkleDb>,
    state_merkle_batch_receiver: Receiver<CommitMessage<StateMerkleBatch>>,
}

impl StateMerkleBatchCommitter {
    pub fn new(
        state_merkle_db: Arc<StateMerkleDb>,
        state_merkle_batch_receiver: Receiver<CommitMessage<StateMerkleBatch>>,
    ) -> Self {
        Self {
            state_merkle_db,
            state_merkle_batch_receiver,
        }
    }

    pub fn run(self) {
        while let Ok(msg) = self.state_merkle_batch_receiver.recv() {
            match msg {
                CommitMessage::Data(state_merkle_batch) => {
                    let StateMerkleBatch {
                        batch,
                        root_hash,
                        state_delta,
                    } = state_merkle_batch;
                    // commit jellyfish merkle nodes
                    let _timer = OTHER_TIMERS_SECONDS
                        .with_label_values(&["commit_jellyfish_merkle_nodes"])
                        .start_timer();
                    self.state_merkle_db
                        .write_schemas(batch)
                        .expect("State merkle batch commit failed.");
                    let current_version = state_delta
                        .current_version
                        .expect("Current version should not be None");
                    LATEST_SNAPSHOT_VERSION.set(current_version as i64);
                    trace!(
                        current_version = current_version,
                        base_version = state_delta.base_version,
                        root_hash = root_hash,
                        "State snapshot committed."
                    );
                }
                CommitMessage::Sync(finish_sender) => finish_sender.send(()).unwrap(),
                CommitMessage::Exit => {
                    break;
                }
            }
        }
        trace!("State merkle batch committing thread exit.")
    }
}
