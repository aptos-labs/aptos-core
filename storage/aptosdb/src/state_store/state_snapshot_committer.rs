// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This file defines the state snapshot committer running in background thread within StateStore.

use crate::{
    metrics::OTHER_TIMERS_SECONDS,
    state_store::{
        buffered_state::CommitMessage,
        persisted_state::PersistedState,
        state_merkle_batch_committer::{StateMerkleBatch, StateMerkleBatchCommitter},
        StateDb,
    },
};
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_logger::trace;
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::{
    jmt_update_refs, state_store::state_with_summary::StateWithSummary, Result,
};
use itertools::Itertools;
use rayon::prelude::*;
use std::sync::{mpsc::Receiver, Arc};

pub(crate) struct StateSnapshotCommitter {
    state_db: Arc<StateDb>,
    last_snapshot: StateWithSummary,
    state_snapshot_commit_receiver: Receiver<CommitMessage<StateWithSummary>>,
    state_merkle_batch_committer: StateMerkleBatchCommitter,
}

impl StateSnapshotCommitter {
    pub fn new(
        state_db: Arc<StateDb>,
        state_snapshot_commit_receiver: Receiver<CommitMessage<StateWithSummary>>,
        last_snapshot: StateWithSummary,
        persisted_state: PersistedState,
    ) -> Self {
        let arc_state_db = Arc::clone(&state_db);
        let committer = StateMerkleBatchCommitter::new(arc_state_db, persisted_state);
        Self {
            state_db,
            last_snapshot,
            state_snapshot_commit_receiver,
            state_merkle_batch_committer: committer,
        }
    }

    pub fn run(mut self) {
        while let Ok(msg) = self.state_snapshot_commit_receiver.recv() {
            match msg {
                CommitMessage::Data(snapshot) => {
                    let version = snapshot.version().expect("Cannot be empty");
                    let base_version = self.last_snapshot.version();
                    let previous_epoch_ending_version = self
                        .state_db
                        .ledger_db
                        .metadata_db()
                        .get_previous_epoch_ending(version)
                        .unwrap()
                        .map(|(v, _e)| v);

                    let (shard_root_nodes, batches_for_shards) = {
                        let _timer =
                            OTHER_TIMERS_SECONDS.timer_with(&["calculate_batches_for_shards"]);

                        let shard_persisted_versions = self
                            .state_db
                            .state_merkle_db
                            .get_shard_persisted_versions(base_version)
                            .unwrap();

                        let min_version = self.last_snapshot.next_version();

                        THREAD_MANAGER.get_non_exe_cpu_pool().install(|| {
                            snapshot
                                .make_delta(&self.last_snapshot)
                                .shards
                                .par_iter()
                                .enumerate()
                                .map(|(shard_id, updates)| {
                                    let node_hashes = snapshot
                                        .summary()
                                        .global_state_summary
                                        .new_node_hashes_since(
                                            &self.last_snapshot.summary().global_state_summary,
                                            shard_id as u8,
                                        );
                                    // TODO(aldenhu): iterator of refs
                                    let updates = {
                                        let _timer =
                                            OTHER_TIMERS_SECONDS.timer_with(&["hash_jmt_updates"]);

                                        updates
                                            .iter()
                                            .filter_map(|(key, slot)| {
                                                slot.maybe_update_jmt(key, min_version)
                                            })
                                            .collect_vec()
                                    };

                                    self.state_db.state_merkle_db.merklize_value_set_for_shard(
                                        shard_id,
                                        jmt_update_refs(&updates),
                                        Some(&node_hashes),
                                        version,
                                        base_version,
                                        shard_persisted_versions[shard_id],
                                        previous_epoch_ending_version,
                                    )
                                })
                                .collect::<Result<Vec<_>>>()
                                .expect("Error calculating StateMerkleBatch for shards.")
                                .into_iter()
                                .unzip()
                        })
                    };

                    let (root_hash, leaf_count, top_levels_batch) = {
                        let _timer =
                            OTHER_TIMERS_SECONDS.timer_with(&["calculate_top_levels_batch"]);
                        self.state_db
                            .state_merkle_db
                            .calculate_top_levels(
                                shard_root_nodes,
                                version,
                                base_version,
                                previous_epoch_ending_version,
                            )
                            .expect("Error calculating StateMerkleBatch for top levels.")
                    };
                    assert_eq!(
                        root_hash,
                        snapshot.summary().root_hash(),
                        "root hash mismatch: jmt: {}, smt: {}",
                        root_hash,
                        snapshot.summary().root_hash(),
                    );

                    let usage = snapshot.state().usage();
                    if !usage.is_untracked() {
                        assert_eq!(
                            leaf_count,
                            usage.items(),
                            "Num of state items mismatch: jmt: {}, state: {}",
                            leaf_count,
                            usage.items(),
                        );
                    }

                    self.state_merkle_batch_committer.commit(StateMerkleBatch {
                        top_levels_batch,
                        batches_for_shards,
                        snapshot,
                    });
                    self.last_snapshot = snapshot.clone();
                },
                CommitMessage::Sync(finish_sender) => finish_sender.send(()).unwrap(),
                CommitMessage::Exit => break,
            }
            trace!("State snapshot committing thread exit.")
        }
    }
}
