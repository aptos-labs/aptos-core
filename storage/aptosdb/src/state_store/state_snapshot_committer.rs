// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file defines the state snapshot committer running in background thread within StateStore.

use crate::{
    metrics::OTHER_TIMERS_SECONDS,
    state_store::{
        buffered_state::CommitMessage,
        persisted_state::PersistedState,
        state_merkle_batch_committer::{StateMerkleBatch, StateMerkleBatchCommitter},
        StateDb,
    },
    versioned_node_cache::VersionedNodeCache,
};
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_logger::{info, trace};
use aptos_metrics_core::TimerHelper;
use aptos_storage_interface::{
    jmt_update_refs, state_store::state_with_summary::StateWithSummary, Result,
};
use itertools::Itertools;
use rayon::prelude::*;
use static_assertions::const_assert;
use std::{
    sync::{
        mpsc,
        mpsc::{Receiver, SyncSender},
        Arc,
    },
    thread::JoinHandle,
};

pub(crate) struct StateSnapshotCommitter {
    state_db: Arc<StateDb>,
    /// Last snapshot merklized and sent for persistence, not guaranteed to have committed already.
    last_snapshot: StateWithSummary,
    state_snapshot_commit_receiver: Receiver<CommitMessage<StateWithSummary>>,
    state_merkle_batch_commit_sender: SyncSender<CommitMessage<StateMerkleBatch>>,
    join_handle: Option<JoinHandle<()>>,
}

impl StateSnapshotCommitter {
    const CHANNEL_SIZE: usize = 0;

    pub fn new(
        state_db: Arc<StateDb>,
        state_snapshot_commit_receiver: Receiver<CommitMessage<StateWithSummary>>,
        last_snapshot: StateWithSummary,
        persisted_state: PersistedState,
    ) -> Self {
        // Note: This is to ensure we cache nodes in memory from previous batches before they get committed to DB.
        const_assert!(
            StateSnapshotCommitter::CHANNEL_SIZE < VersionedNodeCache::NUM_VERSIONS_TO_CACHE
        );
        // Rendezvous channel
        let (state_merkle_batch_commit_sender, state_merkle_batch_commit_receiver) =
            mpsc::sync_channel(Self::CHANNEL_SIZE);
        let arc_state_db = Arc::clone(&state_db);
        let join_handle = std::thread::Builder::new()
            .name("state_batch_committer".to_string())
            .spawn(move || {
                let committer = StateMerkleBatchCommitter::new(
                    arc_state_db,
                    state_merkle_batch_commit_receiver,
                    persisted_state.clone(),
                );
                committer.run();
            })
            .expect("Failed to spawn state merkle batch committer thread.");
        Self {
            state_db,
            last_snapshot,
            state_snapshot_commit_receiver,
            state_merkle_batch_commit_sender,
            join_handle: Some(join_handle),
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
                                    info!(
                                        "shard_id: {}, min_version: {}, updates: {:?}",
                                        shard_id,
                                        min_version,
                                        updates.iter().collect_vec()
                                    );
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

                                    info!(
                                        "shard_id: {}, filtered updates: {:?}",
                                        shard_id, updates
                                    );

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

                    self.last_snapshot = snapshot.clone();

                    self.state_merkle_batch_commit_sender
                        .send(CommitMessage::Data(StateMerkleBatch {
                            top_levels_batch,
                            batches_for_shards,
                            snapshot,
                        }))
                        .unwrap();
                },
                CommitMessage::Sync(finish_sender) => {
                    self.state_merkle_batch_commit_sender
                        .send(CommitMessage::Sync(finish_sender))
                        .unwrap();
                },
                CommitMessage::Exit => {
                    self.state_merkle_batch_commit_sender
                        .send(CommitMessage::Exit)
                        .unwrap();
                    break;
                },
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
