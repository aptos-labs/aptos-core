// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This file defines the state snapshot committer running in background thread within StateStore.

use crate::state_merkle_db::StateMerkleDb;
use crate::state_store::buffered_state::CommitMessage;
use crate::OTHER_TIMERS_SECONDS;
use aptos_crypto::HashValue;
use aptos_logger::trace;
use aptos_types::transaction::Version;
use schemadb::SchemaBatch;
use std::sync::mpsc::Receiver;
use std::sync::Arc;

pub struct StateMerkleBatch {
    pub batch: SchemaBatch,
    pub base_version: Option<Version>,
    pub version: Version,
    pub root_hash: HashValue,
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
                        base_version,
                        version,
                        root_hash,
                    } = state_merkle_batch;
                    // commit jellyfish merkle nodes
                    let _timer = OTHER_TIMERS_SECONDS
                        .with_label_values(&["commit_jellyfish_merkle_nodes"])
                        .start_timer();
                    self.state_merkle_db
                        .write_schemas(batch)
                        .expect("State merkle batch commit failed.");
                    trace!(
                        version = version,
                        base_version = base_version,
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
