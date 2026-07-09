// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This file defines the state merkle snapshot committer running in background thread.

use crate::{
    common::{run_batch_committer_loop, CommitMessage, MerkleBatch},
    metrics::{LATEST_SNAPSHOT_VERSION, OTHER_TIMERS_SECONDS},
    pruner::PrunerManager,
    schema::jellyfish_merkle_node::JellyfishMerkleNodeSchema,
    state_store::{persisted_state::PersistedState, StateDb},
};
use anyhow::{anyhow, ensure, Result};
use aptos_jellyfish_merkle::node_type::NodeKey;
use aptos_logger::{info, trace};
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::state_store::{state::State, state_with_summary::StateWithSummary};
use std::sync::{mpsc::Receiver, Arc};

pub(crate) struct StateMerkleCommit {
    pub snapshot: StateWithSummary,
    pub hot_batch: Option<MerkleBatch>,
    pub cold_batch: MerkleBatch,
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
        let Self {
            state_db,
            state_merkle_batch_receiver,
            persisted_state,
        } = self;
        run_batch_committer_loop(state_merkle_batch_receiver, |commit| {
            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["batch_committer_work"]);
            let StateMerkleCommit {
                snapshot,
                hot_batch,
                cold_batch,
            } = commit;
            let base_version = persisted_state.get_state_summary().version();
            let current_version = snapshot
                .version()
                .expect("Current version should not be None");

            // commit jellyfish merkle nodes
            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["commit_jellyfish_merkle_nodes"]);
            // `ShardedJmtMerkleDb::commit` handles version-cache eviction
            // internally — see `sharded_jmt_merkle_db.rs`.
            if let Some(hot) = hot_batch {
                state_db
                    .hot_state_merkle_db
                    .commit(
                        current_version,
                        hot.top_levels_batch,
                        hot.batches_for_shards,
                    )
                    .expect("Hot state merkle nodes commit failed.");
            }
            state_db
                .state_merkle_db
                .commit(
                    current_version,
                    cold_batch.top_levels_batch,
                    cold_batch.batches_for_shards,
                )
                .expect("State merkle nodes commit failed.");

            info!(
                version = current_version,
                base_version = base_version,
                root_hash = snapshot.summary().root_hash(),
                hot_root_hash = snapshot
                    .summary()
                    .hot_root_hash()
                    .expect("main state always has a hot half"),
                "State snapshot committed."
            );
            LATEST_SNAPSHOT_VERSION.set(current_version as i64);
            state_db
                .state_pruner
                .hot_state_merkle_pruner
                .maybe_set_pruner_target_db_version(current_version);
            state_db
                .state_pruner
                .hot_epoch_snapshot_pruner
                .maybe_set_pruner_target_db_version(current_version);
            state_db
                .state_pruner
                .state_merkle_pruner
                .maybe_set_pruner_target_db_version(current_version);
            state_db
                .state_pruner
                .epoch_snapshot_pruner
                .maybe_set_pruner_target_db_version(current_version);

            check_usage_consistency(&state_db, &snapshot).unwrap();

            snapshot
                .summary()
                .global_state_summary
                .log_generation("buffered_state_commit");
            persisted_state.set(snapshot);
        });
        trace!("State merkle batch committing thread exit.")
    }
}

fn check_usage_consistency(state_db: &StateDb, state: &State) -> Result<()> {
    let version = state
        .version()
        .ok_or_else(|| anyhow!("Committing without version."))?;

    let usage_from_ledger_db = state_db.ledger_db.metadata_db().get_usage(version)?;
    let leaf_count_from_jmt = state_db
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
