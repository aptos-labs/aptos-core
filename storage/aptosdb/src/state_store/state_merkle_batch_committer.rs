// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This file defines the state merkle snapshot committer running in background thread.

use crate::{
    metrics::{LATEST_SNAPSHOT_VERSION, OTHER_TIMERS_SECONDS},
    pruner::PrunerManager,
    schema::jellyfish_merkle_node::JellyfishMerkleNodeSchema,
    state_merkle_db::StateMerkleDb,
    state_store::{buffered_state::CommitMessage, persisted_state::PersistedState, StateDb},
};
use anyhow::{anyhow, ensure, Result};
use aptos_jellyfish_merkle::node_type::NodeKey;
use aptos_logger::{info, trace};
use aptos_metrics_core::TimerHelper;
use aptos_schemadb::batch::RawBatch;
use aptos_storage_interface::state_store::{state::State, state_with_summary::StateWithSummary};
use aptos_types::transaction::Version;
use std::sync::{mpsc::Receiver, Arc};

pub(crate) struct StateMerkleCommit {
    pub snapshot: StateWithSummary,
    pub hot_batch: Option<StateMerkleBatch>,
    pub cold_batch: StateMerkleBatch,
}

pub(crate) struct StateMerkleBatch {
    pub top_levels_batch: RawBatch,
    pub batches_for_shards: Vec<RawBatch>,
}

pub(crate) struct StateMerkleBatchCommitter {
    state_db: Arc<StateDb>,
    state_merkle_batch_receiver: Receiver<CommitMessage<StateMerkleCommit>>,
    persisted_state: PersistedState,
}

impl StateMerkleBatchCommitter {
    pub fn new(
        state_db: Arc<StateDb>,
        state_merkle_batch_receiver: Receiver<CommitMessage<StateMerkleCommit>>,
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
                CommitMessage::Data(StateMerkleCommit {
                    snapshot,
                    hot_batch,
                    cold_batch,
                }) => {
                    let base_version = self.persisted_state.get_state_summary().version();
                    let current_version = snapshot
                        .version()
                        .expect("Current version should not be None");

                    // commit jellyfish merkle nodes
                    let _timer =
                        OTHER_TIMERS_SECONDS.timer_with(&["commit_jellyfish_merkle_nodes"]);
                    if let Some(hot_state_merkle_batch) = hot_batch {
                        self.commit(
                            self.state_db
                                .hot_state_merkle_db
                                .as_ref()
                                .expect("Hot state merkle db must exist."),
                            current_version,
                            hot_state_merkle_batch,
                        )
                        .expect("Hot state merkle nodes commit failed.");
                    }
                    self.commit(&self.state_db.state_merkle_db, current_version, cold_batch)
                        .expect("State merkle nodes commit failed.");

                    info!(
                        version = current_version,
                        base_version = base_version,
                        root_hash = snapshot.summary().root_hash(),
                        hot_root_hash = snapshot.summary().hot_root_hash(),
                        "State snapshot committed."
                    );
                    LATEST_SNAPSHOT_VERSION.set(current_version as i64);
                    if let Some(pruner) = &self.state_db.state_pruner.hot_state_merkle_pruner {
                        pruner.maybe_set_pruner_target_db_version(current_version);
                    }
                    if let Some(pruner) = &self.state_db.state_pruner.hot_epoch_snapshot_pruner {
                        pruner.maybe_set_pruner_target_db_version(current_version);
                    }
                    self.state_db
                        .state_pruner
                        .state_merkle_pruner
                        .maybe_set_pruner_target_db_version(current_version);
                    self.state_db
                        .state_pruner
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

    fn commit(
        &self,
        db: &StateMerkleDb,
        current_version: Version,
        state_merkle_batch: StateMerkleBatch,
    ) -> Result<()> {
        let StateMerkleBatch {
            top_levels_batch,
            batches_for_shards,
        } = state_merkle_batch;
        db.commit(current_version, top_levels_batch, batches_for_shards)?;
        if let Some(lru_cache) = db.lru_cache() {
            db.version_caches()
                .iter()
                .for_each(|(_, cache)| cache.maybe_evict_version(lru_cache));
        }
        Ok(())
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
