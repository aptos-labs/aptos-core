// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file defines the state merkle snapshot committer running in background thread.

use crate::{
    metrics::{LATEST_SNAPSHOT_VERSION, OTHER_TIMERS_SECONDS},
    pruner::PrunerManager,
    schema::jellyfish_merkle_node::JellyfishMerkleNodeSchema,
    state_store::{buffered_state::CommitMessage, persisted_state::PersistedState, StateDb},
};
use anyhow::{anyhow, ensure, Result};
use aptos_jellyfish_merkle::node_type::NodeKey;
use aptos_logger::{info, trace};
use aptos_metrics_core::TimerHelper;
use aptos_schemadb::batch::RawBatch;
use aptos_storage_interface::state_store::{state::State, state_with_summary::StateWithSummary};
use std::sync::{mpsc::Receiver, Arc};

pub struct StateMerkleBatch {
    pub top_levels_batch: RawBatch,
    pub batches_for_shards: Vec<RawBatch>,
    pub snapshot: StateWithSummary,
}

pub(crate) struct StateMerkleBatchCommitter {
    state_db: Arc<StateDb>,
    state_merkle_batch_receiver: Receiver<CommitMessage<StateMerkleBatch>>,
    persisted_state: PersistedState,
}

impl StateMerkleBatchCommitter {
    pub fn new(
        state_db: Arc<StateDb>,
        state_merkle_batch_receiver: Receiver<CommitMessage<StateMerkleBatch>>,
        persisted_state: PersistedState,
    ) -> Self {
        Self {
            state_db,
            state_merkle_batch_receiver,
            persisted_state,
        }
    }

    pub fn run(self) {
        while let Ok(msg) = self.state_merkle_batch_receiver.recv() {
            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["batch_committer_work"]);
            match msg {
                CommitMessage::Data(state_merkle_batch) => {
                    let StateMerkleBatch {
                        top_levels_batch,
                        batches_for_shards,
                        snapshot,
                    } = state_merkle_batch;

                    let base_version = self.persisted_state.get_state_summary().version();
                    let current_version = snapshot
                        .version()
                        .expect("Current version should not be None");

                    // commit jellyfish merkle nodes
                    let _timer =
                        OTHER_TIMERS_SECONDS.timer_with(&["commit_jellyfish_merkle_nodes"]);
                    self.state_db
                        .state_merkle_db
                        .commit(current_version, top_levels_batch, batches_for_shards)
                        .expect("State merkle nodes commit failed.");
                    if let Some(lru_cache) = self.state_db.state_merkle_db.lru_cache() {
                        self.state_db
                            .state_merkle_db
                            .version_caches()
                            .iter()
                            .for_each(|(_, cache)| cache.maybe_evict_version(lru_cache));
                    }

                    info!(
                        version = current_version,
                        base_version = base_version,
                        root_hash = snapshot.summary().root_hash(),
                        "State snapshot committed."
                    );
                    LATEST_SNAPSHOT_VERSION.set(current_version as i64);
                    self.state_db
                        .state_merkle_pruner
                        .maybe_set_pruner_target_db_version(current_version);
                    self.state_db
                        .epoch_snapshot_pruner
                        .maybe_set_pruner_target_db_version(current_version);

                    self.check_usage_consistency(&snapshot).unwrap();

                    snapshot
                        .summary()
                        .global_state_summary
                        .log_generation("buffered_state_commit");
                    self.persisted_state.set(snapshot);
                },
                CommitMessage::Sync(finish_sender) => finish_sender.send(()).unwrap(),
                CommitMessage::Exit => {
                    break;
                },
            }
        }
        trace!("State merkle batch committing thread exit.")
    }

    fn check_usage_consistency(&self, state: &State) -> Result<()> {
        let version = state
            .version()
            .ok_or_else(|| anyhow!("Committing without version."))?;

        let usage_from_ledger_db = self.state_db.ledger_db.metadata_db().get_usage(version)?;
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

        let usage_from_in_mem_state = state.usage();
        if !usage_from_in_mem_state.is_untracked() {
            ensure!(
                usage_from_in_mem_state == usage_from_ledger_db,
                "State storage usage info inconsistent. from smt: {:?}, from ledger_db: {:?}",
                usage_from_in_mem_state,
                usage_from_ledger_db,
            );
        }

        Ok(())
    }
}
