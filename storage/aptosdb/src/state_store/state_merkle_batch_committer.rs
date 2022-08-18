// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This file defines the state merkle snapshot committer running in background thread.

use crate::jellyfish_merkle_node::JellyfishMerkleNodeSchema;
use crate::state_store::buffered_state::CommitMessage;
use crate::state_store::StateDb;
use crate::version_data::{VersionData, VersionDataSchema};
use crate::OTHER_TIMERS_SECONDS;
use anyhow::{anyhow, ensure, Result};
use aptos_crypto::HashValue;
use aptos_jellyfish_merkle::node_type::NodeKey;
use aptos_logger::{info, trace, warn};
use aptos_types::transaction::Version;
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
    state_db: Arc<StateDb>,
    state_merkle_batch_receiver: Receiver<CommitMessage<StateMerkleBatch>>,
}

impl StateMerkleBatchCommitter {
    pub fn new(
        state_db: Arc<StateDb>,
        state_merkle_batch_receiver: Receiver<CommitMessage<StateMerkleBatch>>,
    ) -> Self {
        Self {
            state_db,
            state_merkle_batch_receiver,
        }
    }

    pub fn run(self) {
        while let Ok(msg) = self.state_merkle_batch_receiver.recv() {
            match msg {
                CommitMessage::Data {
                    data,
                    snapshot_ready_sender,
                    ..
                } => {
                    let StateMerkleBatch {
                        batch,
                        root_hash,
                        state_delta,
                    } = data;
                    // commit jellyfish merkle nodes
                    let _timer = OTHER_TIMERS_SECONDS
                        .with_label_values(&["commit_jellyfish_merkle_nodes"])
                        .start_timer();
                    self.state_db
                        .state_merkle_db
                        .write_schemas(batch)
                        .expect("State merkle batch commit failed.");
                    snapshot_ready_sender.send(()).unwrap();
                    info!(
                        version = state_delta.current_version,
                        base_version = state_delta.base_version,
                        root_hash = root_hash,
                        "State snapshot committed."
                    );
                    self.check_state_item_count_consistency(state_delta.current_version.unwrap())
                        .unwrap_or_else(|e| warn!("{}", e));
                }
                CommitMessage::Sync(finish_sender) => finish_sender.send(()).unwrap(),
                CommitMessage::Exit => {
                    break;
                }
            }
        }
        trace!("State merkle batch committing thread exit.")
    }

    fn check_state_item_count_consistency(&self, version: Version) -> Result<()> {
        let VersionData {
            state_items: count_from_ledger_db,
            total_state_bytes: _,
        } = self
            .state_db
            .ledger_db
            .get::<VersionDataSchema>(&version)?
            .ok_or_else(|| anyhow!("VersionData missing for version {}", version))?;

        let count_from_state_tree = self
            .state_db
            .state_merkle_db
            .get::<JellyfishMerkleNodeSchema>(&NodeKey::new_empty_path(version))?
            .ok_or_else(|| anyhow!("Root node missing at version {}", version))?
            .leaf_count();

        ensure!(
            count_from_ledger_db == count_from_state_tree,
            "State item count inconsistent, {} from ledger db and {} from state tree.",
            count_from_ledger_db,
            count_from_state_tree,
        );
        Ok(())
    }
}
