// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file defines the state merkle snapshot committer running in background thread.

use crate::{
    jellyfish_merkle_node::JellyfishMerkleNodeSchema,
    metrics::LATEST_SNAPSHOT_VERSION,
    state_store::{buffered_state::CommitMessage, StateDb},
    version_data::VersionDataSchema,
    PrunerManager, ShardedStateMerkleSchemaBatch, OTHER_TIMERS_SECONDS,
};
use anyhow::{anyhow, ensure, Result};
use aptos_crypto::HashValue;
use aptos_jellyfish_merkle::node_type::NodeKey;
use aptos_logger::{info, trace};
use aptos_schemadb::SchemaBatch;
use aptos_storage_interface::state_delta::StateDelta;
use aptos_types::state_store::state_storage_usage::StateStorageUsage;
use std::sync::{mpsc::Receiver, Arc};

pub struct StateMerkleBatch {
    pub top_levels_batch: SchemaBatch,
    pub sharded_batch: ShardedStateMerkleSchemaBatch,
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
                CommitMessage::Data(state_merkle_batch) => {
                    let StateMerkleBatch {
                        top_levels_batch,
                        sharded_batch,
                        root_hash,
                        state_delta,
                    } = state_merkle_batch;

                    let current_version = state_delta
                        .current_version
                        .expect("Current version should not be None");

                    // commit jellyfish merkle nodes
                    let _timer = OTHER_TIMERS_SECONDS
                        .with_label_values(&["commit_jellyfish_merkle_nodes"])
                        .start_timer();
                    self.state_db
                        .state_merkle_db
                        .commit(current_version, top_levels_batch, sharded_batch)
                        .expect("State merkle nodes commit failed.");
                    if self.state_db.state_merkle_db.cache_enabled() {
                        self.state_db
                            .state_merkle_db
                            .version_cache()
                            .maybe_evict_version(self.state_db.state_merkle_db.lru_cache());
                    }
                    info!(
                        version = current_version,
                        base_version = state_delta.base_version,
                        root_hash = root_hash,
                        "State snapshot committed."
                    );
                    LATEST_SNAPSHOT_VERSION.set(current_version as i64);
                    self.state_db
                        .state_merkle_pruner
                        .maybe_set_pruner_target_db_version(current_version);
                    self.state_db
                        .epoch_snapshot_pruner
                        .maybe_set_pruner_target_db_version(current_version);

                    self.check_usage_consistency(&state_delta).unwrap();
                },
                CommitMessage::Sync(finish_sender) => finish_sender.send(()).unwrap(),
                CommitMessage::Exit => {
                    break;
                },
            }
        }
        trace!("State merkle batch committing thread exit.")
    }

    fn check_usage_consistency(&self, state_delta: &StateDelta) -> Result<()> {
        let version = state_delta
            .current_version
            .ok_or_else(|| anyhow!("Committing without version."))?;

        let usage_from_ledger_db: StateStorageUsage = self
            .state_db
            .ledger_db
            .metadata_db()
            .get::<VersionDataSchema>(&version)?
            .ok_or_else(|| anyhow!("VersionData missing for version {}", version))?
            .get_state_storage_usage();
        let leaf_count_from_jmt = self
            .state_db
            .state_merkle_db
            .metadata_db()
            .get::<JellyfishMerkleNodeSchema>(&NodeKey::new_empty_path(version))?
            .ok_or_else(|| anyhow!("Root node missing at version {}", version))?
            .leaf_count();

        ensure!(
            usage_from_ledger_db.items() == leaf_count_from_jmt,
            "State item count inconsistent, {} from ledger db and {} from state tree.",
            usage_from_ledger_db.items(),
            leaf_count_from_jmt,
        );

        let usage_from_smt = state_delta.current.usage();
        if !usage_from_smt.is_untracked() {
            ensure!(
                usage_from_smt == usage_from_ledger_db,
                "State storage usage info inconsistent. from smt: {:?}, from ledger_db: {:?}",
                usage_from_smt,
                usage_from_ledger_db,
            );
        }

        Ok(())
    }
}
